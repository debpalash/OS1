# IndOS Autonomous Build–Test–Fix Loop

Protocol for iterating on the ISO **without a human in the loop**. The agent
(Claude) is the "fix" stage; everything else is scripted.

## One-time setup (human, once)

```bash
sudo install -m 440 -o root -g root autotest/sudoers-indos /etc/sudoers.d/indos
```

This lets the agent run `sudo -n autotest/build.sh` (mkarchiso needs root).
Everything else — QEMU, KVM, screenshots, serial console — already works
unprivileged (`/dev/kvm` is 0666, user is in `video`).

## The loop

```
┌─> 1. BUILD   sudo -n autotest/build.sh [--clean]      (~5–15 min)
│   2. TEST    autotest/autotest.py                      (~3–6 min)
│   3. READ    test-logs/run-*/report.md + report.json
│              test-logs/run-*/serial.log
│              test-logs/run-*/NN-*.png   <- agent views PNGs directly
│   4. FIX     edit indos-iso/ profile, configs, Rust sources
└── 5. repeat until report.json verdict == "PASS"
```

Run the long steps with `run_in_background` and poll; never block the
session foreground on a build.

## Test harness (`autotest/autotest.py`)

- Boots the newest `build/out/indos-*.iso` headless under QEMU/KVM
  (`-display none`, virtio VGA, q35, 8G/8vcpu).
- **Serial console** is the primary sense+control channel: syslinux is
  configured with `SERIAL 0 115200` and the kernel cmdline has
  `console=ttyS0`. The harness expects the boot banner, kernel load, and a
  login prompt; then logs in `root` / `indos` and runs in-guest checks.
- **Screenshots at every stage** via QMP `screendump` (PNG). This captures
  the emulated framebuffer itself, so it works during BIOS, the syslinux
  menu, kernel boot, and the Wayland desktop — no window, VNC, or OCR
  required. The agent reads the PNGs directly.
- **QMP** (`qmp.sock` in the run dir) also provides `sendkey`,
  `input-send-event` (mouse), `system_powerdown`, `quit` — enough to drive
  the Calamares installer GUI when needed.
- **SSH fallback**: sshd is enabled in the ISO; `~/.ssh/id_ed25519.pub` is
  baked into `/home/indos/.ssh/authorized_keys`; the harness forwards
  host `127.0.0.1:2222 -> guest :22`:
  `ssh -p 2222 -o StrictHostKeyChecking=no indos@127.0.0.1`
- `--vnc` additionally exposes VNC on `127.0.0.1:5977` for a human who
  wants to watch; the harness never needs it.
- `--keep-running` leaves the VM up after the checks for interactive
  debugging via QMP/serial/SSH.
- `--disk path.qcow2` attaches a target disk for installer testing.

### What counts as PASS

Required: bootloader seen → kernel loads → login prompt → root shell;
`systemctl is-system-running` is `running`/`degraded` with **zero failed
units**; `greetd`, `sshd`, `NetworkManager` active; `indos-orchestrator`,
`indos-shell`, `indos-session`, `calamares` binaries present; `niri`
process running. Optional (recorded, non-fatal): orchestrator + ollama
processes. Diagnostics (journal errors, greetd log, DRM nodes, session
list) are always captured into the report.

## Debug recipes

- VM won't boot at all → `test-logs/run-*/qemu.log` + `01-*.png`.
- Boot hangs → `serial.log` has the full kernel log; screenshot shows VT.
- Desktop fails → `diagnostics.greetd-journal` and `journal-errors` in
  `report.json`; `NN-desktop.png` shows what the user would see.
- Need to poke a live VM:
  `python3 -c` against `qmp.sock`, or `nc -U .../serial.sock`, or SSH.
- Installer automation: boot with `--disk`, drive Calamares with QMP
  `input-send-event` clicks guided by fresh screendumps.

## Layout

```
autotest/autotest.py   boot-test harness (no deps, stdlib only)
autotest/build.sh      root build wrapper (sudoers target)
autotest/sudoers-indos sudoers rule to install once
test-logs/run-*/       one dir per test run (reports, PNGs, serial log)
test-logs/build-*.log  one log per build
```
