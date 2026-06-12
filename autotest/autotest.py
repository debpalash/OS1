#!/usr/bin/env python3
"""IndOS autonomous boot-test harness.

Boots an IndOS ISO headless in QEMU/KVM, watches the serial console,
logs in as root, runs in-guest health checks, takes QMP screenshots
(PNG — directly readable by Claude), and writes a machine-readable
report. Designed so an agent can build → test → diagnose → fix → repeat
without a human.

Usage:
    ./autotest.py [--iso PATH] [--mem 8G] [--smp 8] [--timeout 300]
                  [--disk PATH] [--vnc] [--keep-running] [--settle SECS]

Exit codes: 0 = all required checks passed, 1 = failures, 2 = harness error.

Artifacts land in test-logs/run-<timestamp>/:
    qemu.log       QEMU stderr
    serial.log     full serial console transcript (chardev logfile)
    NN-<stage>.png screenshots at key stages
    report.json    structured results
    report.md      human/agent-readable summary
"""

import argparse
import base64
import glob
import json
import os
import re
import shlex
import signal
import socket
import subprocess
import sys
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT_DIR = os.path.join(ROOT, "build", "out")
LOG_ROOT = os.path.join(ROOT, "test-logs")

# CSI sequences, OSC strings (incl. shell-integration OSC 3008, terminated
# by BEL or ST), charset selects, and stray control bytes
ANSI_RE = re.compile(
    rb"\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)?"   # OSC ... BEL/ST
    rb"|\x1b\[[0-9;?]*[a-zA-Z]"               # CSI
    rb"|\x1b[()][0B]|\x1b[=>]"
    rb"|[\x00-\x08\x0b\x0c\x0e-\x1a\x1c-\x1f]")


def strip_ansi(data: bytes) -> str:
    return ANSI_RE.sub(b"", data).decode("utf-8", errors="replace")


class QMP:
    """Minimal QMP client over a unix socket."""

    def __init__(self, path: str, timeout: float = 30.0):
        deadline = time.time() + timeout
        while True:
            try:
                self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                self.sock.connect(path)
                break
            except (FileNotFoundError, ConnectionRefusedError):
                if time.time() > deadline:
                    raise
                time.sleep(0.3)
        self.sock.settimeout(10)
        self.buf = b""
        self._recv_msg()  # greeting
        self.cmd("qmp_capabilities")

    def _recv_msg(self) -> dict:
        while b"\n" not in self.buf:
            self.buf += self.sock.recv(65536)
        line, self.buf = self.buf.split(b"\n", 1)
        return json.loads(line)

    def cmd(self, name: str, args: dict | None = None) -> dict:
        msg = {"execute": name}
        if args:
            msg["arguments"] = args
        self.sock.sendall(json.dumps(msg).encode() + b"\n")
        while True:
            resp = self._recv_msg()
            if "event" in resp:
                continue
            return resp

    def screendump(self, path: str):
        resp = self.cmd("screendump", {"filename": path, "format": "png"})
        return resp.get("error")  # None on success

    def quit(self):
        try:
            self.cmd("quit")
        except Exception:
            pass

    def click(self, x: int, y: int, w: int = 1280, h: int = 800, button: str = "left"):
        """Absolute click via the usb-tablet (coords in guest pixels)."""
        ax, ay = int(x * 32767 / w), int(y * 32767 / h)
        move = [{"type": "abs", "data": {"axis": "x", "value": ax}},
                {"type": "abs", "data": {"axis": "y", "value": ay}}]
        self.cmd("input-send-event", {"events": move})
        self.cmd("input-send-event", {"events": [
            {"type": "btn", "data": {"button": button, "down": True}}]})
        self.cmd("input-send-event", {"events": [
            {"type": "btn", "data": {"button": button, "down": False}}]})

    KEYMAP = {**{c: c for c in "abcdefghijklmnopqrstuvwxyz0123456789"},
              " ": "spc", ".": "dot", "-": "minus", "_": ("shift", "minus"),
              "/": "slash", "@": ("shift", "2"), "!": ("shift", "1"),
              "\n": "ret", "\t": "tab"}

    def type_text(self, text: str):
        for ch in text:
            key = self.KEYMAP.get(ch.lower() if ch.isalpha() else ch)
            if key is None:
                continue
            keys = []
            if ch.isupper() or isinstance(key, tuple):
                keys.append({"type": "qcode", "data": "shift"})
            name = key[1] if isinstance(key, tuple) else key
            keys.append({"type": "qcode", "data": name})
            self.cmd("input-send-event", {"events": [
                {"type": "key", "data": {"key": {"type": "qcode", "data": k["data"]},
                                          "down": True}} for k in keys]})
            self.cmd("input-send-event", {"events": [
                {"type": "key", "data": {"key": {"type": "qcode", "data": k["data"]},
                                          "down": False}} for k in reversed(keys)]})
            time.sleep(0.04)


class Serial:
    """Expect-style client for QEMU's serial chardev socket."""

    def __init__(self, path: str, timeout: float = 30.0):
        deadline = time.time() + timeout
        while True:
            try:
                self.sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                self.sock.connect(path)
                break
            except (FileNotFoundError, ConnectionRefusedError):
                if time.time() > deadline:
                    raise
                time.sleep(0.3)
        self.sock.setblocking(False)
        self.buf = b""

    def _pump(self):
        while True:
            try:
                data = self.sock.recv(65536)
                if not data:
                    return
                self.buf += data
            except BlockingIOError:
                return

    def text(self) -> str:
        self._pump()
        return strip_ansi(self.buf)

    def wait_for(self, pattern: str, timeout: float, flags=re.M):
        """Wait until regex matches console text. Returns the match or None."""
        rx = re.compile(pattern, flags)
        deadline = time.time() + timeout
        while time.time() < deadline:
            m = rx.search(self.text())
            if m:
                return m
            time.sleep(0.5)
        return None

    def send(self, s: str):
        self.sock.sendall(s.encode())

    def mark(self) -> int:
        self._pump()
        return len(self.buf)

    def text_since(self, mark: int) -> str:
        self._pump()
        return strip_ansi(self.buf[mark:])


class GuestShell:
    """Runs commands on a logged-in root serial shell using exit-code markers."""

    def __init__(self, serial: Serial):
        self.serial = serial
        self.n = 0

    def run(self, cmd: str, timeout: float = 30.0):
        """Returns (exit_code|None, output_text)."""
        self.n += 1
        # $((...)) keeps the tag out of the echoed command line, so the
        # marker regex only matches real output even if echo is on.
        tag = f"@@M{1000 + self.n}@@"
        mark = self.serial.mark()
        self.serial.send(f"{cmd}; echo @@M$((1000 + {self.n}))@@:$?\r")
        rx = re.compile(re.escape(tag) + r":(\d+)")
        deadline = time.time() + timeout
        while time.time() < deadline:
            out = self.serial.text_since(mark)
            m = rx.search(out)
            if m:
                body = out[: m.start()]
                # drop echoed command, prompts, and shell-integration junk
                # (the capture mark can split an OSC sequence, leaving tails)
                junk = re.compile(
                    r"\[root@[\w-]+[^\]]*\]#|;machineid=|;exit=|;bootid=|^\s*\\?\s*$")
                lines = [l for l in body.splitlines()
                         if cmd[:40] not in l and not junk.search(l)]
                return int(m.group(1)), "\n".join(lines).strip()
            time.sleep(0.3)
        return None, self.serial.text_since(mark).strip()


class Runner:
    def __init__(self, args):
        self.args = args
        ts = time.strftime("%Y%m%d-%H%M%S")
        self.run_dir = os.path.join(LOG_ROOT, f"run-{ts}")
        os.makedirs(self.run_dir, exist_ok=True)
        self.qmp_path = os.path.join(self.run_dir, "qmp.sock")
        self.ser_path = os.path.join(self.run_dir, "serial.sock")
        self.serial_log = os.path.join(self.run_dir, "serial.log")
        self.shot_n = 0
        self.report = {
            "iso": args.iso,
            "started": ts,
            "stages": [],
            "checks": [],
            "screenshots": [],
            "verdict": None,
        }
        self.proc = None
        self.qmp = None

    # ---------- lifecycle ----------

    def qemu_cmd(self):
        a = self.args
        cmd = [
            "qemu-system-x86_64",
            "-S",  # start paused; we resume after the serial client is attached
            "-machine", "q35,accel=kvm",
            "-cpu", "host",
            "-smp", str(a.smp),
            "-m", a.mem,
        ]
        if not a.boot_disk:
            cmd += ["-cdrom", a.iso, "-boot", "d"]
        cmd += [
            # virgl 3D accel is required: niri refuses software EGL renderers
            # and comes up with zero outputs on a plain virtio-vga
            # xres/yres pin the guest at 1280x800; zoom-to-fit scales the
            # window instead of resizing the guest (stable click coords)
            "-device", "virtio-vga-gl,xres=1280,yres=800",
            "-display", "gtk,gl=on,show-cursor=on,zoom-to-fit=on" if a.gui else "egl-headless",
            "-qmp", f"unix:{self.qmp_path},server=on,wait=off",
            "-chardev",
            f"socket,id=ser0,path={self.ser_path},server=on,wait=off,logfile={self.serial_log}",
            "-serial", "chardev:ser0",
            "-netdev", f"user,id=n0,hostfwd=tcp:127.0.0.1:{a.ssh_port}-:22",
            "-device", "virtio-net-pci,netdev=n0",
            # absolute-coordinate pointer so QMP input-send-event can drive GUIs
            "-device", "qemu-xhci", "-device", "usb-tablet",
        ]
        if a.disk:
            cmd += ["-drive", f"file={a.disk},if=virtio,format=qcow2"]
        if a.vnc:
            cmd += ["-vnc", "127.0.0.1:77"]
        return cmd

    def launch(self):
        # auto-pick a free SSH forward port so parallel/leaked VMs can't collide
        with socket.socket() as s:
            s.bind(("127.0.0.1", 0))
            self.args.ssh_port = s.getsockname()[1]
        self.report["ssh_port"] = self.args.ssh_port
        qemu_log = open(os.path.join(self.run_dir, "qemu.log"), "wb")
        self.proc = subprocess.Popen(
            self.qemu_cmd(), stdout=qemu_log, stderr=qemu_log,
            stdin=subprocess.DEVNULL,
        )
        time.sleep(1)
        if self.proc.poll() is not None:
            raise RuntimeError(
                f"QEMU exited immediately (rc={self.proc.returncode}); see qemu.log")
        self.qmp = QMP(self.qmp_path)
        self.serial = Serial(self.ser_path)
        self.qmp.cmd("cont")

    def shutdown(self):
        if self.args.keep_running:
            print(f"--keep-running: VM left up (pid {self.proc.pid}), "
                  f"QMP={self.qmp_path} VNC={'127.0.0.1:5977' if self.args.vnc else 'off'}")
            return
        if self.qmp:
            self.qmp.quit()
        if self.proc:
            try:
                self.proc.wait(timeout=15)
            except subprocess.TimeoutExpired:
                self.proc.kill()

    # ---------- helpers ----------

    def shot(self, name: str) -> str:
        self.shot_n += 1
        path = os.path.join(self.run_dir, f"{self.shot_n:02d}-{name}.png")
        if self.qmp:
            err = self.qmp.screendump(path)
            if err is None:
                self.report["screenshots"].append(os.path.basename(path))
            else:
                self.report["screenshots"].append(
                    f"FAILED {os.path.basename(path)}: {err.get('desc', err)}")
        return path

    def ssh(self, cmd: str, timeout: int = 30):
        base = ["ssh", "-p", str(self.args.ssh_port), "-o", "StrictHostKeyChecking=no",
                "-o", "UserKnownHostsFile=/dev/null", "-o", "BatchMode=yes",
                "-o", f"ConnectTimeout={timeout}", "indos@127.0.0.1", cmd]
        return subprocess.run(base, capture_output=True, text=True, timeout=timeout + 10)

    def shot_desktop(self, name: str):
        """True-pixel desktop capture: niri screenshot inside the guest,
        pulled out over SSH. QMP screendump can't read virgl surfaces."""
        self.shot_n += 1
        fname = f"{self.shot_n:02d}-{name}.png"
        path = os.path.join(self.run_dir, fname)
        try:
            r = self.ssh("export NIRI_SOCKET=$(ls /run/user/1000/niri.*.sock | head -1); "
                         "niri msg action screenshot-screen && sleep 1 && "
                         "ls -t $HOME/Pictures/Screenshots/*.png | head -1")
            remote = r.stdout.strip().splitlines()[-1] if r.returncode == 0 and r.stdout.strip() else None
            if remote:
                subprocess.run(
                    ["scp", "-P", str(self.args.ssh_port), "-o", "StrictHostKeyChecking=no",
                     "-o", "UserKnownHostsFile=/dev/null", "-o", "BatchMode=yes",
                     f"indos@127.0.0.1:{remote}", path],
                    capture_output=True, timeout=30, check=True)
                self.report["screenshots"].append(fname)
                return path
        except Exception:
            pass
        self.shot_n -= 1
        return self.shot(name)  # fallback: QMP screendump

    def stage(self, name: str, ok: bool, detail: str = ""):
        self.report["stages"].append({"name": name, "ok": ok, "detail": detail})
        print(f"[{'PASS' if ok else 'FAIL'}] stage: {name}" + (f" — {detail}" if detail else ""))
        return ok

    def check(self, name: str, ok, detail: str, required: bool = True):
        self.report["checks"].append(
            {"name": name, "ok": bool(ok), "required": required, "detail": detail})
        print(f"[{'PASS' if ok else 'FAIL'}] check: {name}")

    # ---------- test flow ----------

    def wait_boot(self) -> bool:
        s = self.serial
        if self.args.boot_disk:
            # installed system: GRUB + kernel are silent on serial; the
            # enabled serial-getty@ttyS0 prints the login prompt when up
            m = s.wait_for(r"(indos login:|\w+ login:)", timeout=self.args.timeout)
            self.shot("disk-boot")
            return self.stage("disk-boot-login-prompt", bool(m),
                              m.group(0) if m else f"no login prompt in {self.args.timeout}s")
        if not s.wait_for(r"ISOLINUX", timeout=60):
            self.shot("no-bootloader")
            return self.stage("bootloader", False, "ISOLINUX banner never appeared on serial")
        self.stage("bootloader", True)

        if self.args.inject_console:
            # Older ISOs lack console=ttyS0 on the kernel cmdline. The
            # bootloader has SERIAL 0, so edit the entry live: Tab opens
            # the cmdline editor, typed text appends, Enter boots.
            if s.wait_for(r"(Automatic boot in|Press \[Tab\])", timeout=30):
                time.sleep(1)
                self.shot("boot-menu")
                s.send("\t")
                time.sleep(1.5)
                s.send(" console=tty0 console=ttyS0,115200\r")
                self.stage("inject-console", True, "appended console=ttyS0 at boot prompt")
            else:
                self.shot("boot-menu")
                self.stage("inject-console", False, "menu prompt not seen; autoboot only")
        else:
            time.sleep(2)
            self.shot("boot-menu")

        # syslinux suppresses its "Loading ..." lines when the kernel
        # cmdline contains "quiet" (which os1 uses for the plymouth
        # splash), and plymouth.ignore-serial-consoles keeps boot
        # details off ttyS0 entirely — so also accept the first
        # kernel-side serial output: systemd's OSC context marker
        # (\x1b]3008;), an [  OK  ] unit line, or the getty banner.
        if not s.wait_for(r"(Loading /arch.*vmlinuz|type=boot|\[ *OK *\]|:: running early hook"
                          r"|\]3008;|\w+ login:)",
                          timeout=90, flags=re.M | re.S):
            self.shot("no-kernel-load")
            return self.stage("kernel-load", False, "kernel never loaded (menu stuck?)")
        self.stage("kernel-load", True)

        # Either a getty login prompt or an autologin root shell on ttyS0.
        # systemd-firstboot may interactively block boot on broken ISOs —
        # skip its prompts with empty answers so testing can continue
        # (the report still flags it via the firstboot-blocked stage).
        firstboot_hits = 0
        deadline = time.time() + self.args.timeout
        m = None
        while time.time() < deadline:
            m = s.wait_for(r"(os1 login:|indos login:|archiso login:|root@[\w-]+|\[root@[\w-]+)",
                           timeout=5)
            if m:
                break
            if re.search(r"(Please enter|--prompt|empty to skip)", s.text()[-400:]):
                s.send("\r")
                firstboot_hits += 1
                time.sleep(1)
        if firstboot_hits:
            self.stage("firstboot-blocked", False,
                       f"systemd-firstboot prompted {firstboot_hits}x — boot blocks "
                       "without autotest intervention (mask it in customize_airootfs.sh)")
        self.shot("post-boot")
        if not m:
            txt = s.text()
            panic = "KERNEL PANIC" if "panic" in txt.lower() else ""
            emergency = "EMERGENCY MODE" if "emergency" in txt.lower() else ""
            return self.stage("login-prompt", False,
                              f"no login prompt within {self.args.timeout}s {panic}{emergency}".strip())
        self.stage("login-prompt", True, m.group(0))
        return True

    def login(self) -> bool:
        s = self.serial
        txt = s.text()
        if re.search(r"(\[)?root@[\w-]+", txt.splitlines()[-1] if txt.splitlines() else ""):
            self.stage("login", True, "autologin shell")
        else:
            user, pw = (("indos", "indos123") if self.args.boot_disk
                        else ("root", "indos"))
            s.send(user + "\r")
            if s.wait_for(r"Password:", timeout=15):
                s.send(pw + "\r")
            # confirm shell
            sh = GuestShell(s)
            rc, _ = sh.run("true", timeout=20)
            if rc is None:
                self.shot("login-failed")
                return self.stage("login", False, "no shell after root/indos login")
            self.stage("login", True)
        self.sh = GuestShell(s)
        # quiet kernel messages on console so command output stays clean
        if self.args.boot_disk:
            # non-root login: cache sudo credentials for later checks
            self.sh.run("echo indos123 | sudo -S true; sudo dmesg -n 1; stty -echo",
                        timeout=15)
        else:
            self.sh.run("dmesg -n 1; stty -echo", timeout=10)
        return True

    def guest_checks(self):
        sh = self.sh
        run = sh.run

        # boot may still be settling — wait out the "starting" state
        state = ""
        for _ in range(18):
            rc, out = run("systemctl is-system-running", timeout=20)
            state = out.strip().splitlines()[-1] if out.strip() else ""
            if state not in ("starting", "initializing", ""):
                break
            time.sleep(5)
        self.check("system-state", state in ("running", "degraded"),
                   f"state={state}", required=True)

        rc, out = run("systemctl --failed --no-legend --plain | cat", timeout=20)
        clean = out.strip()
        # out may be empty (no failed) OR contain "0 loaded units listed."
        # Shell prompt fragments may leak through GuestShell junk-stripping,
        # so also check that no real unit lines (STATE columns) are present.
        has_failed = bool(clean) and "loaded" not in clean.lower() and re.search(r"\b(failed|error)\b", clean, re.I)
        self.check("no-failed-units", rc == 0 and not has_failed, clean or "none")

        for unit in ("greetd", "sshd", "NetworkManager"):
            rc, out = run(f"systemctl is-active {unit}", timeout=15)
            last = out.strip().splitlines()[-1] if out.strip() else ""
            self.check(f"unit-{unit}", last == "active", last)

        for binary in ("indos-orchestrator", "indos-shell", "indos-session",
                       "indos-voiced"):
            rc, out = run(f"command -v {binary}", timeout=15)
            self.check(f"bin-{binary}", rc == 0, out.strip())

        rc, out = run("command -v calamares", timeout=15)
        self.check("bin-calamares", rc == 0, out.strip())

        if self.args.boot_disk:
            # installed systems show tuigreet — log in via the VM keyboard
            self.qmp.type_text("indos\n")
            time.sleep(2)
            self.qmp.type_text("indos123\n")
            self.stage("tuigreet-login", True, "credentials typed via QMP")
        # give the graphical session time to come up before judging it
        time.sleep(self.args.settle)
        rc, out = run("pgrep -ax niri | head -3", timeout=15)
        self.check("proc-niri", bool(out.strip()), out.strip())
        niri_q = ("env XDG_RUNTIME_DIR=/run/user/1000 sh -c "
                  "'NIRI_SOCKET=$(ls /run/user/1000/niri.*.sock) niri msg outputs' | head -2")
        if not self.args.boot_disk:
            niri_q = "sudo -u indos " + niri_q
        rc, out = run(niri_q, timeout=15)
        # must contain a real output line, not an error message
        self.check("niri-outputs", out.strip().startswith("Output"), out.strip()[:120])
        # NOTE: pass/fail must come from pgrep's output, not the pipeline's
        # exit code — `pgrep | head` always exits 0 (head's status)
        for proc in ("waybar", "foot", "swaync"):
            rc, out = run(f"pgrep -a {proc} | head -2", timeout=15)
            self.check(f"proc-{proc}", bool(out.strip()), out.strip())
        rc, out = run("pgrep -a calamares | head -2", timeout=15)
        if self.args.boot_disk:
            # the installer must NOT autostart on an installed system
            self.check("calamares-not-running", not out.strip(), out.strip() or "absent")
        else:
            self.check("proc-calamares", bool(out.strip()), out.strip())
        rc, out = run("pgrep -af 'indos-shell' | head -2", timeout=15)
        self.check("proc-indos-shell", bool(out.strip()), out.strip())
        rc, out = run("pgrep -af indos-orchestrator | head -3", timeout=15)
        self.check("proc-orchestrator", bool(out.strip()), out.strip(), required=False)
        rc, out = run("pgrep -af 'ollama serve' | head -3", timeout=15)
        self.check("proc-ollama", bool(out.strip()), out.strip(), required=False)
        # voice daemon idles gracefully without STT/TTS deps, but the
        # process itself must be up under the user session
        rc, out = run("pgrep -ax indos-voiced | head -2", timeout=15)
        self.check("proc-voiced", bool(out.strip()), out.strip(), required=False)

        if self.args.ai:
            self.ai_checks()

        self.shot_desktop("desktop")

        # Informational diagnostics — always recorded, never pass/fail
        diag = {}
        for name, cmd in [
            ("journal-errors", "journalctl -b -p err --no-pager | tail -n 40"),
            ("greetd-journal", "journalctl -b -u greetd --no-pager | tail -n 25"),
            ("user-session", "loginctl list-sessions --no-legend; loginctl list-users --no-legend"),
            ("failed-detail", "systemctl --failed --no-pager | cat"),
            ("drm", "ls -l /dev/dri/ 2>&1"),
        ]:
            _, out = run(cmd, timeout=25)
            diag[name] = out
        self.report["diagnostics"] = diag

    def ai_checks(self):
        """M1 exit criteria: pull a model and get a chat answer through
        the orchestrator IPC socket. Needs guest network + cow_spacesize
        big enough for the model (~400MB)."""
        run = self.sh.run

        rc, out = run("sudo -u indos ollama list 2>/dev/null | grep -c qwen2.5:0.5b", timeout=15)
        if out.strip().splitlines()[-1:] != ["1"]:
            rc, out = run("sudo -u indos ollama pull qwen2.5:0.5b 2>&1 | tail -1",
                          timeout=600)
            self.check("ai-model-pull", rc == 0 and "error" not in out.lower(),
                       out.strip()[-150:])
        else:
            self.check("ai-model-pull", True, "already present")

        chat = ('{"type":"chat","content":"Reply with exactly the word PONG.",'
                '"session_id":null}\n')
        b64 = base64.b64encode(chat.encode()).decode()
        run(f"echo {b64} | base64 -d > /tmp/chat.json", timeout=10)
        rc, out = run(
            "sudo -u indos socat -t 90 - "
            "UNIX-CONNECT:/run/user/1000/indos/orchestrator.sock "
            "< /tmp/chat.json | tail -2", timeout=120)
        ok = '"done"' in out
        self.check("ai-chat-roundtrip", ok, out.strip()[-200:])

    # ---------- report ----------

    def finalize(self) -> int:
        req_checks = [c for c in self.report["checks"] if c["required"]]
        stages_ok = all(s["ok"] for s in self.report["stages"])
        checks_ok = all(c["ok"] for c in req_checks)
        self.report["verdict"] = "PASS" if (stages_ok and checks_ok) else "FAIL"

        with open(os.path.join(self.run_dir, "report.json"), "w") as f:
            json.dump(self.report, f, indent=2)

        lines = [f"# IndOS autotest — {self.report['verdict']}",
                 f"ISO: `{self.args.iso}`", f"Run dir: `{self.run_dir}`", "",
                 "## Stages"]
        for s in self.report["stages"]:
            lines.append(f"- {'✅' if s['ok'] else '❌'} {s['name']}"
                         + (f" — {s['detail']}" if s["detail"] else ""))
        lines.append("\n## Checks")
        for c in self.report["checks"]:
            req = "" if c["required"] else " (optional)"
            lines.append(f"- {'✅' if c['ok'] else '❌'} {c['name']}{req}: {c['detail'][:200]}")
        lines.append("\n## Screenshots")
        for p in self.report["screenshots"]:
            lines.append(f"- {p}")
        if "diagnostics" in self.report:
            lines.append("\n## Diagnostics")
            for k, v in self.report["diagnostics"].items():
                lines.append(f"\n### {k}\n```\n{v[:3000]}\n```")
        with open(os.path.join(self.run_dir, "report.md"), "w") as f:
            f.write("\n".join(lines) + "\n")

        print(f"\nVERDICT: {self.report['verdict']}")
        print(f"Report: {self.run_dir}/report.md")
        return 0 if self.report["verdict"] == "PASS" else 1

    def run(self) -> int:
        try:
            self.launch()
            if self.wait_boot() and self.login():
                self.guest_checks()
        except Exception as e:
            self.report["stages"].append(
                {"name": "harness", "ok": False, "detail": f"{type(e).__name__}: {e}"})
            print(f"[FAIL] harness error: {e}", file=sys.stderr)
        finally:
            try:
                rc = self.finalize()
            finally:
                self.shutdown()
        return rc


def latest_iso() -> str | None:
    isos = sorted(glob.glob(os.path.join(OUT_DIR, "os1-*.iso"))
                  + glob.glob(os.path.join(OUT_DIR, "indos-*.iso")),
                  key=os.path.getmtime)
    return isos[-1] if isos else None


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--iso", default=latest_iso(), help="ISO path (default: newest in build/out)")
    ap.add_argument("--mem", default="8G")
    ap.add_argument("--smp", type=int, default=8)
    ap.add_argument("--timeout", type=int, default=300, help="seconds to wait for login prompt")
    ap.add_argument("--settle", type=int, default=25, help="seconds to let the desktop settle")
    ap.add_argument("--disk", help="attach a qcow2 disk (installer tests)")
    ap.add_argument("--boot-disk", action="store_true",
                    help="boot from --disk instead of the ISO (installed-system test)")
    ap.add_argument("--ssh-port", type=int, default=2222)
    ap.add_argument("--inject-console", action="store_true",
                    help="append console=ttyS0 at the syslinux prompt (for ISOs built without it)")
    ap.add_argument("--vnc", action="store_true", help="also expose VNC on 127.0.0.1:5977")
    ap.add_argument("--gui", action="store_true",
                    help="show a native QEMU window (no VNC client needed)")
    ap.add_argument("--ai", action="store_true",
                    help="run AI-stack checks: model pull + orchestrator chat round-trip")
    ap.add_argument("--keep-running", action="store_true")
    args = ap.parse_args()
    if not args.iso or not os.path.exists(args.iso):
        print("No ISO found. Build one first (sudo autotest/build.sh).", file=sys.stderr)
        return 2
    return Runner(args).run()


if __name__ == "__main__":
    sys.exit(main())
