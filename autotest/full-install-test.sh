#!/bin/bash
# Full unattended installer pipeline:
#   rebuild ISO → boot live → drive Calamares → wait for install →
#   prep installed disk (test channels) → boot installed disk → checks.
#
# Usage: autotest/full-install-test.sh [--no-build]
# Exit 0 = installed system passed all health checks.

set -euo pipefail
cd "$(dirname "$0")/.."
DISK=test-logs/install-disk.qcow2

if [[ "${1:-}" != "--no-build" ]]; then
    echo "=== build ==="
    sudo -n autotest/build.sh --clean | tail -1
fi

echo "=== live boot ==="
rm -f $DISK
qemu-img create -f qcow2 $DISK 50G >/dev/null
python3 autotest/autotest.py --timeout 300 --disk $DISK --keep-running | grep -E 'VERDICT|left up'
RUN=$(ls -td test-logs/run-* | head -1)

echo "=== drive installer ==="
autotest/install-flow.sh "$RUN"

P=$(jq -r .ssh_port "$RUN/report.json")
SSH="ssh -p $P -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o BatchMode=yes indos@127.0.0.1"

echo "=== wait for install ==="
ok=0
for i in $(seq 1 60); do
    if $SSH 'grep -qE "uninhibitSleep|onInstallationFailed" /home/indos/.cache/calamares/session.log' 2>/dev/null; then
        ok=1; break
    fi
    sleep 20
done
[ "$ok" = 1 ] || { echo "TIMEOUT waiting for install"; exit 1; }
if $SSH 'grep -q onInstallationFailed /home/indos/.cache/calamares/session.log' 2>/dev/null; then
    echo "INSTALL FAILED:"
    $SSH 'grep -A10 onInstallationFailed /home/indos/.cache/calamares/session.log | head -14' 2>/dev/null
    $SSH 'cat /home/indos/.cache/calamares/session.log' 2>/dev/null > "$RUN/calamares-session.log" || true
    exit 1
fi
echo "INSTALL OK"
$SSH 'cat /home/indos/.cache/calamares/session.log' 2>/dev/null > "$RUN/calamares-session.log" || true

echo "=== prep installed disk (test channels: ssh key + serial getty) ==="
$SSH 'sudo sh -c "mount /dev/vda2 /mnt && \
  install -d -m700 -o1000 -g1000 /mnt/home/indos/.ssh && \
  install -m600 -o1000 -g1000 /home/indos/.ssh/authorized_keys /mnt/home/indos/.ssh/authorized_keys && \
  systemctl --root=/mnt enable serial-getty@ttyS0.service && \
  umount /mnt && echo PREP-OK"' 2>/dev/null

python3 - "$RUN" <<'PY'
import sys
sys.path.insert(0, 'autotest')
from autotest import QMP
QMP(f'{sys.argv[1]}/qmp.sock', timeout=5).quit()
PY
sleep 3

echo "=== boot installed disk ==="
python3 autotest/autotest.py --disk $DISK --boot-disk --timeout 240
