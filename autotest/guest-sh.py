#!/usr/bin/env python3
"""Run commands inside a live autotest VM over its serial socket.

Usage:
    ./guest-sh.py <run-dir> "cmd1" ["cmd2" ...]

The VM must have been left up with --keep-running. Assumes a root shell
is already logged in on the serial console (autotest leaves it that way);
if not, it logs in as root/indos first.
"""

import sys
import os
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from autotest import Serial, GuestShell


def main():
    run_dir, cmds = sys.argv[1], sys.argv[2:]
    s = Serial(os.path.join(run_dir, "serial.sock"), timeout=5)
    s.send("\r")
    time.sleep(1)
    tail = s.text()[-200:]
    if "login:" in tail:
        s.send("root\r")
        if s.wait_for(r"Password:", timeout=10):
            s.send("indos\r")
        time.sleep(2)
    sh = GuestShell(s)
    sh.run("stty -echo; dmesg -n 1", timeout=10)
    for cmd in cmds:
        rc, out = sh.run(cmd, timeout=60)
        print(f"$ {cmd}\n[rc={rc}]\n{out}\n")


if __name__ == "__main__":
    main()
