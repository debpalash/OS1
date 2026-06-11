#!/bin/bash
# Drive the Calamares wizard unattended in a --keep-running autotest VM.
#
# Usage: autotest/install-flow.sh <run-dir>
#
# REQUIRES a headless VM (no --gui): the guest then stays at exactly
# 1280x800 and these coordinates are stable. A visible GTK window gets
# resized by the host WM and breaks blind clicking. Presets fill the
# users page except passwords. Screenshots land in <run-dir> per step.

set -euo pipefail
RUN=$1
W=1280; H=800
STEP="$(dirname "$0")/../autotest/install-step.sh"

click() {  # click <x> <y> [text]
python3 - "$RUN" "$1" "$2" "${3:-}" <<'PY'
import sys, time
sys.path.insert(0, 'autotest')
from autotest import QMP
run, x, y = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
q = QMP(f'{run}/qmp.sock', timeout=5)
q.click(x, y, w=1280, h=800)
if len(sys.argv) > 4 and sys.argv[4]:
    time.sleep(0.5)
    q.type_text(sys.argv[4])
PY
sleep 2
}

shot() {
P=$(jq -r .ssh_port "$RUN/report.json")
SSH="ssh -p $P -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o BatchMode=yes indos@127.0.0.1"
F=$($SSH 'export NIRI_SOCKET=$(ls /run/user/1000/niri.*.sock | head -1); niri msg action screenshot-screen >/dev/null; sleep 1; ls -t Pictures/Screenshots/*.png | head -1' 2>/dev/null)
scp -P "$P" -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o BatchMode=yes "indos@127.0.0.1:$F" "$RUN/flow-$1.png" 2>/dev/null
echo "$RUN/flow-$1.png"
}

NEXT_X=1008; NEXT_Y=756

click $NEXT_X $NEXT_Y            # welcome -> location
click $NEXT_X $NEXT_Y            # location -> keyboard
click $NEXT_X $NEXT_Y            # keyboard -> partition (erase preselected)
click $NEXT_X $NEXT_Y            # partition -> users
shot users
click 236 278 "indos123"         # user password
click 442 278 "indos123"         # repeat
click 236 400 "indos123"         # admin password
click 442 400 "indos123"         # repeat
click $NEXT_X $NEXT_Y            # users -> summary
shot summary
click $NEXT_X $NEXT_Y            # Install
sleep 1
click 742 451                    # Install Now (confirm dialog)
shot installing
echo "install started"
