#!/usr/bin/env bash
# Root build wrapper for the autonomous loop.
# Installed in sudoers (NOPASSWD) so the agent can rebuild without a human.
#
# Usage: sudo /home/pal/Desktop/IndOS/autotest/build.sh [--clean]
#
# Logs to test-logs/build-<timestamp>.log and prints the log path + ISO path.

set -euo pipefail

ROOT="/home/pal/Desktop/IndOS"
LOG_DIR="$ROOT/test-logs"
mkdir -p "$LOG_DIR"
LOG="$LOG_DIR/build-$(date +%Y%m%d-%H%M%S).log"

if [[ "${1:-}" == "--clean" ]]; then
    echo "Cleaning work dir..."
    # an aborted mkarchiso can leave sysfs/proc/dev binds mounted inside
    # the work dir — rm would traverse into them; unmount deepest-first
    { findmnt -rno TARGET | grep -F "$ROOT/build/work" || true; } | sort -r | while read -r m; do
        umount -l "$m" 2>/dev/null || true
    done
    rm -rf "$ROOT/build/work"
fi

echo "Build log: $LOG"
set +e
bash "$ROOT/indos-iso/build-iso.sh" >"$LOG" 2>&1
RC=$?
set -e

if [[ $RC -ne 0 ]]; then
    echo "BUILD FAILED (rc=$RC). Last 60 lines:"
    tail -n 60 "$LOG"
    exit $RC
fi

ISO=$(ls -t "$ROOT"/build/out/indos-*.iso | head -1)
# Make artifacts readable/removable by the user
chown "$(stat -c %U:%G "$ROOT")" "$ISO" "$LOG" 2>/dev/null || true
echo "BUILD OK: $ISO"
tail -n 5 "$LOG"
