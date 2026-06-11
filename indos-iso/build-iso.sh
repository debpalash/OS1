#!/usr/bin/env bash
# Build IndOS ISO (CachyOS Base)
#
# Usage: sudo ./build-iso.sh [work-dir] [out-dir]
#
# Prerequisites:
# - archiso installed (pacman -S archiso)
# - CachyOS repos configured on build host (for keyring)
# - Run as root
# - Release binaries built

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
ISO_PROFILE="$SCRIPT_DIR"
# Use disk-backed dirs — /tmp is tmpfs (RAM) and too small for ISO builds
WORK_DIR="${1:-${PROJECT_ROOT}/build/work}"
OUT_DIR="${2:-${PROJECT_ROOT}/build/out}"

echo "═══════════════════════════════════════════"
echo " IndOS ISO Builder (CachyOS Base)"
echo "═══════════════════════════════════════════"
echo " Profile: $ISO_PROFILE"
echo " Work:    $WORK_DIR"
echo " Output:  $OUT_DIR"
echo " Kernel:  linux-cachyos (BORE scheduler)"
echo "═══════════════════════════════════════════"

# Sanity checks
if [[ $EUID -ne 0 ]]; then
    echo "ERROR: Must run as root (sudo ./build-iso.sh)"
    exit 1
fi

if ! command -v mkarchiso &>/dev/null; then
    echo "ERROR: archiso not installed. Run: pacman -S archiso"
    exit 1
fi

# Step 0: Ensure CachyOS keyring is available on host
echo ""
echo "[0/5] Checking CachyOS keyring..."
if ! pacman-key --list-keys | grep -q cachyos 2>/dev/null; then
    echo "Installing CachyOS keyring..."
    pacman-key --recv-keys F3B607488DB35A47 --keyserver keyserver.ubuntu.com
    pacman-key --lsign-key F3B607488DB35A47
    echo "NOTE: You may need CachyOS repos on your host. See: https://cachyos.org/docs/"
fi

# Step 1: Set up CachyOS mirrorlists in the profile
echo ""
echo "[1/5] Setting up CachyOS mirrorlists..."
mkdir -p "$ISO_PROFILE/airootfs/etc/pacman.d"

# CachyOS main mirror
cat > "$ISO_PROFILE/airootfs/etc/pacman.d/cachyos-mirrorlist" << 'EOF'
Server = https://mirror.cachyos.org/repo/$repo/$arch
Server = https://de-mirror.cachyos.org/repo/$repo/$arch
Server = https://us-mirror.cachyos.org/repo/$repo/$arch
EOF

# CachyOS v3 (x86-64-v3 optimized)
cat > "$ISO_PROFILE/airootfs/etc/pacman.d/cachyos-v3-mirrorlist" << 'EOF'
Server = https://mirror.cachyos.org/repo/$repo/$arch
Server = https://de-mirror.cachyos.org/repo/$repo/$arch
Server = https://us-mirror.cachyos.org/repo/$repo/$arch
EOF

# Step 2: Build release binaries (as the real user, not root)
echo ""
echo "[2/5] Building release binaries..."
if [[ -n "${SUDO_USER:-}" ]]; then
    echo "    Building as $SUDO_USER (root has no rustup)..."
    sudo -u "$SUDO_USER" bash -c "cd '$PROJECT_ROOT/indos-orchestrator' && cargo build --release"
    sudo -u "$SUDO_USER" bash -c "cd '$PROJECT_ROOT/indos-shell' && cargo build --release"
else
    (cd "$PROJECT_ROOT/indos-orchestrator" && cargo build --release)
    (cd "$PROJECT_ROOT/indos-shell" && cargo build --release)
fi

# Step 3: Copy binaries into airootfs overlay
echo ""
echo "[3/5] Installing binaries..."
mkdir -p "$ISO_PROFILE/airootfs/usr/local/bin"
if [[ -f "$PROJECT_ROOT/indos-orchestrator/target/release/indos-orchestrator" ]]; then
    cp "$PROJECT_ROOT/indos-orchestrator/target/release/indos-orchestrator" \
       "$ISO_PROFILE/airootfs/usr/local/bin/"
else
    echo "    WARNING: indos-orchestrator binary not found, skipping"
fi
if [[ -f "$PROJECT_ROOT/indos-shell/target/release/indos-shell" ]]; then
    cp "$PROJECT_ROOT/indos-shell/target/release/indos-shell" \
       "$ISO_PROFILE/airootfs/usr/local/bin/"
else
    echo "    WARNING: indos-shell binary not found, skipping"
fi

# Copy session script
cp "$PROJECT_ROOT/indos-settings/session/indos-session.sh" \
   "$ISO_PROFILE/airootfs/usr/local/bin/indos-session"

# Copy waybar helper script (custom module data provider)
cp "$PROJECT_ROOT/indos-settings/waybar/indos-waybar.sh" \
   "$ISO_PROFILE/airootfs/usr/local/bin/indos-waybar"
chmod +x "$ISO_PROFILE/airootfs/usr/local/bin/indos-waybar"

# Step 4: Copy configs into skel AND /usr/share/indos/settings
echo ""
echo "[4/5] Installing default configs..."

# a) Configs into /etc/skel (user gets them on useradd)
SKEL="$ISO_PROFILE/airootfs/etc/skel/.config"
mkdir -p "$SKEL"/{niri,waybar,swaync}
cp "$PROJECT_ROOT/indos-settings/niri/config.kdl" "$SKEL/niri/" 2>/dev/null || true
cp "$PROJECT_ROOT/indos-settings/waybar/config.jsonc" "$SKEL/waybar/" 2>/dev/null || true
cp "$PROJECT_ROOT/indos-settings/waybar/style.css" "$SKEL/waybar/" 2>/dev/null || true
cp "$PROJECT_ROOT/indos-settings/swaync/config.json" "$SKEL/swaync/" 2>/dev/null || true
cp "$PROJECT_ROOT/indos-settings/swaync/style.css" "$SKEL/swaync/" 2>/dev/null || true

# b) Configs into /usr/share/indos/settings (session script fallback)
SETTINGS="$ISO_PROFILE/airootfs/usr/share/indos/settings"
mkdir -p "$SETTINGS"/{niri,waybar,swaync}
cp "$PROJECT_ROOT/indos-settings/niri/config.kdl" "$SETTINGS/niri/" 2>/dev/null || true
cp "$PROJECT_ROOT/indos-settings/waybar/config.jsonc" "$SETTINGS/waybar/" 2>/dev/null || true
cp "$PROJECT_ROOT/indos-settings/waybar/style.css" "$SETTINGS/waybar/" 2>/dev/null || true
cp "$PROJECT_ROOT/indos-settings/swaync/config.json" "$SETTINGS/swaync/" 2>/dev/null || true
cp "$PROJECT_ROOT/indos-settings/swaync/style.css" "$SETTINGS/swaync/" 2>/dev/null || true

# c) Calamares overlay configs
if [ -d "$PROJECT_ROOT/indos-iso/calamares" ]; then
    mkdir -p "$ISO_PROFILE/airootfs/root/indos-calamares"
    cp -r "$PROJECT_ROOT/indos-iso/calamares"/* "$ISO_PROFILE/airootfs/root/indos-calamares/"
    echo "    Calamares overlay installed"
fi

# d) Dev SSH key for passwordless access during testing
if [ -f "${HOME}/.ssh/id_ed25519.pub" ]; then
    cp "${HOME}/.ssh/id_ed25519.pub" "$ISO_PROFILE/airootfs/root/dev-ssh-key.pub"
    echo "    Dev SSH key installed"
elif [ -n "${SUDO_USER:-}" ] && [ -f "/home/${SUDO_USER}/.ssh/id_ed25519.pub" ]; then
    cp "/home/${SUDO_USER}/.ssh/id_ed25519.pub" "$ISO_PROFILE/airootfs/root/dev-ssh-key.pub"
    echo "    Dev SSH key installed"
fi

# Step 5: Build ISO
echo ""
echo "[5/5] Building ISO image..."
mkdir -p "$OUT_DIR"

# Run mkarchiso with debug output to catch silent failures
set +e
mkarchiso -v -w "$WORK_DIR" -o "$OUT_DIR" "$ISO_PROFILE"
MKARCHISO_EXIT=$?
set -e

if [[ $MKARCHISO_EXIT -ne 0 ]]; then
    echo ""
    echo "ERROR: mkarchiso failed with exit code $MKARCHISO_EXIT"
    exit $MKARCHISO_EXIT
fi

echo ""
echo "═══════════════════════════════════════════"
echo " IndOS ISO built successfully!"
echo " Base:   CachyOS (x86-64-v3)"
echo " Kernel: linux-cachyos (BORE)"
echo "═══════════════════════════════════════════"
ls -lh "$OUT_DIR"/indos-*.iso 2>/dev/null
