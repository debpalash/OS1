#!/bin/bash
# IndOS QEMU Test Harness — Fully Autonomous
# 
# Boots the ISO headless with:
#   - QEMU monitor socket → screendump for visual capture
#   - Serial console → kernel/systemd boot log
#   - SSH port forwarding → remote command execution
#   - VNC display → optional browser-based viewing
#
# Usage: ./test-harness/qemu-test.sh [ISO_PATH]
#
# Output: test-harness/results/
#   - serial.log          — full boot log
#   - screen_*.png        — periodic screenshots
#   - diagnostics.log     — service/IPC test results
#   - report.txt          — pass/fail summary

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
RESULTS_DIR="$SCRIPT_DIR/results"
ISO_PATH="${1:-$PROJECT_DIR/build/out/$(ls -t $PROJECT_DIR/build/out/*.iso 2>/dev/null | head -1)}"

# Ports
SSH_PORT=2222
VNC_DISPLAY=1
MONITOR_SOCK="$RESULTS_DIR/qemu-monitor.sock"
SERIAL_LOG="$RESULTS_DIR/serial.log"
DISK_IMG="$RESULTS_DIR/test-disk.qcow2"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

log() { echo -e "${GREEN}[TEST]${NC} $*"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
fail() { echo -e "${RED}[FAIL]${NC} $*"; }

# === SETUP ===
setup() {
    log "Setting up test environment..."
    mkdir -p "$RESULTS_DIR"
    rm -f "$RESULTS_DIR"/*.log "$RESULTS_DIR"/*.png "$RESULTS_DIR"/*.ppm
    
    # Create test disk if not exists
    if [ ! -f "$DISK_IMG" ]; then
        log "Creating 30GB test disk..."
        qemu-img create -f qcow2 "$DISK_IMG" 30G
    fi
    
    # Kill any existing QEMU
    pkill -f "qemu-system-x86_64.*IndOS" 2>/dev/null || true
    sleep 1
    
    log "ISO: $ISO_PATH"
    log "Results: $RESULTS_DIR"
}

# === QEMU MONITOR COMMANDS ===
qemu_cmd() {
    python3 -c "
import socket, time
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect('$MONITOR_SOCK')
s.recv(4096)  # consume banner
s.sendall(b'$1\n')
time.sleep(0.5)
data = s.recv(8192)
s.close()
print(data.decode(), end='')
" 2>/dev/null
}

# Capture screenshot via QEMU monitor
screendump() {
    local name="${1:-screen}"
    local ppm="$RESULTS_DIR/${name}.ppm"
    local png="$RESULTS_DIR/${name}.png"
    qemu_cmd "screendump $ppm" >/dev/null 2>&1
    sleep 0.5
    if [ -f "$ppm" ]; then
        convert "$ppm" "$png" 2>/dev/null && rm -f "$ppm"
        log "Screenshot saved: $png"
    else
        warn "Screenshot failed"
    fi
}

# === BOOT QEMU ===
boot_qemu() {
    log "Booting QEMU (headless + VNC:$VNC_DISPLAY + SSH:$SSH_PORT)..."
    
    qemu-system-x86_64 \
        -enable-kvm \
        -m 8G \
        -smp 4 \
        -cpu host \
        -device virtio-vga \
        -display none \
        -vnc :${VNC_DISPLAY} \
        -cdrom "$ISO_PATH" \
        -drive file="$DISK_IMG",format=qcow2,if=virtio \
        -boot d \
        -nic user,model=virtio-net-pci,hostfwd=tcp::${SSH_PORT}-:22 \
        -serial file:"$SERIAL_LOG" \
        -monitor unix:"$MONITOR_SOCK",server,nowait \
        -name "IndOS Test" \
        -daemonize
    
    log "QEMU started (PID: $(pgrep -f 'IndOS Test' | head -1))"
}

# === WAIT FOR BOOT ===
wait_for_boot() {
    local max_wait=${1:-180}  # 3 minutes default
    log "Waiting for boot (max ${max_wait}s)..."
    
    # Send Enter after 8s to pass syslinux menu (for old ISOs without TIMEOUT)
    sleep 8
    qemu_cmd "sendkey ret" >/dev/null 2>&1
    log "Sent Enter to boot menu"
    screendump "01_boot_menu"
    
    # Wait for SSH
    local elapsed=8
    while [ $elapsed -lt $max_wait ]; do
        if ssh -o StrictHostKeyChecking=no -o ConnectTimeout=3 -o UserKnownHostsFile=/dev/null \
           -o BatchMode=yes -i ~/.ssh/id_ed25519 -p $SSH_PORT indos@localhost true 2>/dev/null; then
            log "SSH ready after ${elapsed}s"
            screendump "02_desktop_booted"
            return 0
        fi
        sleep 5
        elapsed=$((elapsed + 5))
        
        # Take a screenshot every 30s to track progress
        if [ $((elapsed % 30)) -eq 0 ]; then
            screendump "boot_${elapsed}s"
        fi
    done
    
    screendump "boot_timeout"
    fail "SSH not ready after ${max_wait}s"
    
    # Dump serial log for diagnosis
    log "Serial log tail:"
    tail -30 "$SERIAL_LOG" 2>/dev/null
    return 1
}

# === SSH COMMAND HELPER ===
vm_ssh() {
    ssh -o StrictHostKeyChecking=no -o ConnectTimeout=10 -o UserKnownHostsFile=/dev/null \
        -o ServerAliveInterval=5 -o ServerAliveCountMax=3 \
        -i ~/.ssh/id_ed25519 -p $SSH_PORT indos@localhost "$@" 2>/dev/null
}

# === RUN DIAGNOSTICS ===
run_diagnostics() {
    log "Running diagnostics..."
    
    vm_ssh bash -s << 'DIAG' > "$RESULTS_DIR/diagnostics.log" 2>&1
#!/bin/bash
echo "============================================"
echo " IndOS Diagnostic Report"
echo " $(date)"
echo "============================================"

echo ""
echo "=== SYSTEM ==="
echo "Kernel: $(uname -r)"
echo "OS: $(cat /etc/os-release | grep PRETTY_NAME | cut -d= -f2)"
echo "Uptime: $(uptime -p)"
echo "RAM: $(free -h | awk '/Mem:/{print $2 " total, " $3 " used, " $7 " avail"}')"
echo ""

echo "=== DISK ==="
lsblk -o NAME,SIZE,TYPE,MOUNTPOINT 2>/dev/null
echo ""
df -h / /home /tmp /mnt/data 2>/dev/null
echo ""

echo "=== DISPLAY ==="
echo "Niri: $(pgrep niri >/dev/null && echo RUNNING || echo NOT_RUNNING)"
echo "Waybar: $(pgrep waybar >/dev/null && echo RUNNING || echo NOT_RUNNING)"
echo "SwayNC: $(pgrep swaync >/dev/null && echo RUNNING || echo NOT_RUNNING)"
echo "Foot: $(pgrep foot >/dev/null && echo RUNNING || echo NOT_RUNNING)"
echo "Calamares: $(pgrep calamares >/dev/null && echo RUNNING || echo NOT_RUNNING)"
echo ""

echo "=== AI SERVICES ==="
echo "Ollama process: $(pgrep ollama >/dev/null && echo RUNNING || echo NOT_RUNNING)"
echo "Ollama service: $(systemctl --user is-active ollama-user.service 2>/dev/null || echo unknown)"
echo "Orchestrator process: $(pgrep indos-orch >/dev/null && echo RUNNING || echo NOT_RUNNING)"
echo "Orchestrator service: $(systemctl --user is-active indos-orchestrator.service 2>/dev/null || echo unknown)"
echo "IPC socket: $(test -S /run/user/$(id -u)/indos/orchestrator.sock && echo EXISTS || echo MISSING)"
echo ""

echo "=== OLLAMA DETAILS ==="
systemctl --user status ollama-user.service --no-pager 2>&1 | head -12
echo ""
journalctl --user -u ollama-user --no-pager -n 10 2>&1
echo ""

echo "=== ORCHESTRATOR DETAILS ==="
systemctl --user status indos-orchestrator.service --no-pager 2>&1 | head -12
echo ""
journalctl --user -u indos-orchestrator --no-pager -n 10 2>&1
echo ""

echo "=== NETWORK ==="
ip -4 addr show | grep inet | head -3
ping -c 1 -W 2 8.8.8.8 >/dev/null 2>&1 && echo "Internet: OK" || echo "Internet: UNREACHABLE"
echo ""

echo "=== KEY BINARIES ==="
ls -la /usr/local/bin/indos-* 2>/dev/null || echo "No IndOS binaries"
ls -la /usr/bin/ollama 2>/dev/null || echo "No ollama binary"
echo ""

echo "=== IPC TEST ==="
if [ -S /run/user/$(id -u)/indos/orchestrator.sock ]; then
    python3 -c "
import socket, json
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.connect('/run/user/$(id -u)/indos/orchestrator.sock')
msg = json.dumps({'type': 'status'}) + '\n'
s.sendall(msg.encode())
s.settimeout(5)
resp = b''
while True:
    try:
        c = s.recv(4096)
        if not c: break
        resp += c
        if b'\n' in c: break
    except: break
s.close()
for line in resp.decode().strip().split('\n'):
    if line.strip():
        print(json.dumps(json.loads(line), indent=2))
" 2>&1
else
    echo "SKIP: No orchestrator socket"
fi

echo ""
echo "============================================"
echo " Diagnostic Report Complete"
echo "============================================"
DIAG

    log "Diagnostics saved to $RESULTS_DIR/diagnostics.log"
}

# === PARSE RESULTS ===
generate_report() {
    log "Generating test report..."
    
    local report="$RESULTS_DIR/report.txt"
    local diag="$RESULTS_DIR/diagnostics.log"
    local pass=0
    local fail_count=0
    
    echo "============================================" > "$report"
    echo " IndOS Test Report — $(date)" >> "$report"
    echo "============================================" >> "$report"
    echo "" >> "$report"
    
    # Test each component
    check_test() {
        local name="$1"
        local pattern="$2"
        local expected="$3"
        if grep -q "$pattern" "$diag" 2>/dev/null; then
            echo "  ✅ PASS: $name" >> "$report"
            pass=$((pass + 1))
        else
            echo "  ❌ FAIL: $name (expected: $expected)" >> "$report"
            fail_count=$((fail_count + 1))
        fi
    }
    
    echo "--- Boot Tests ---" >> "$report"
    check_test "Kernel booted" "Kernel:" "kernel version present"
    check_test "IndOS branding" "IndOS Generative Desktop" "os-release set"
    
    echo "" >> "$report"
    echo "--- Display Tests ---" >> "$report"
    check_test "Niri compositor" "Niri: RUNNING" "niri process"
    check_test "Foot terminal" "Foot: RUNNING" "foot process"
    check_test "SwayNC" "SwayNC: RUNNING" "swaync process"
    check_test "Waybar" "Waybar: RUNNING" "waybar process"
    
    echo "" >> "$report"
    echo "--- AI Stack Tests ---" >> "$report"
    check_test "Ollama service" "Ollama service: active" "ollama active"
    check_test "Orchestrator service" "Orchestrator service: active" "orchestrator active"
    check_test "IPC socket" "IPC socket: EXISTS" "socket file"
    
    echo "" >> "$report"
    echo "--- Network Tests ---" >> "$report"
    check_test "Internet" "Internet: OK" "network connectivity"
    
    echo "" >> "$report"
    echo "--- Summary ---" >> "$report"
    echo "  Passed: $pass" >> "$report"
    echo "  Failed: $fail_count" >> "$report"
    echo "  Total:  $((pass + fail_count))" >> "$report"
    echo "" >> "$report"
    
    # List screenshots
    echo "--- Screenshots ---" >> "$report"
    ls -1 "$RESULTS_DIR"/*.png 2>/dev/null | while read f; do
        echo "  $(basename "$f")" >> "$report"
    done
    
    echo "" >> "$report"
    echo "============================================" >> "$report"
    
    cat "$report"
    return $fail_count
}

# === CLEANUP ===
cleanup() {
    log "Taking final screenshot..."
    screendump "99_final"
    log "Shutting down QEMU..."
    qemu_cmd "quit" >/dev/null 2>&1 || true
    sleep 2
    pkill -f "qemu-system-x86_64.*IndOS" 2>/dev/null || true
}

# === MAIN ===
main() {
    log "IndOS Test Harness v1.0"
    log "========================"
    
    setup
    boot_qemu
    
    if wait_for_boot 180; then
        screendump "03_desktop_ready"
        run_diagnostics
        screendump "04_after_diagnostics"
        generate_report
    else
        fail "Boot failed — check serial log and screenshots"
        generate_report
    fi
    
    # Don't cleanup yet — keep VM running for manual inspection
    log "VM still running. Kill with: pkill -f 'IndOS Test'"
    log "VNC available at: localhost:$((5900 + VNC_DISPLAY))"
    log "SSH available at: ssh -p $SSH_PORT indos@localhost"
}

main "$@"
