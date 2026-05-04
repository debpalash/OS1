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
WORK_DIR="${1:-/tmp/indos-build}"
OUT_DIR="${2:-/tmp/indos-out}"

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

# Step 2: Build release binaries
echo ""
echo "[2/5] Building release binaries..."
(cd "$PROJECT_ROOT/indos-orchestrator" && cargo build --release)
(cd "$PROJECT_ROOT/indos-shell" && cargo build --release)

# Step 3: Copy binaries into airootfs overlay
echo ""
echo "[3/5] Installing binaries..."
mkdir -p "$ISO_PROFILE/airootfs/usr/local/bin"
cp "$PROJECT_ROOT/indos-orchestrator/target/release/indos-orchestrator" \
   "$ISO_PROFILE/airootfs/usr/local/bin/"
cp "$PROJECT_ROOT/indos-shell/target/release/indos-shell" \
   "$ISO_PROFILE/airootfs/usr/local/bin/"

# Copy session script
cp "$PROJECT_ROOT/indos-settings/session/indos-session.sh" \
   "$ISO_PROFILE/airootfs/usr/local/bin/indos-session"

# Step 4: Copy configs into skel
echo ""
echo "[4/5] Installing default configs..."
SKEL="$ISO_PROFILE/airootfs/etc/skel/.config"
mkdir -p "$SKEL"/{niri,waybar,swaync}
cp "$PROJECT_ROOT/indos-settings/niri/config.kdl" "$SKEL/niri/" 2>/dev/null || true
cp "$PROJECT_ROOT/indos-settings/waybar/config.jsonc" "$SKEL/waybar/" 2>/dev/null || true
cp "$PROJECT_ROOT/indos-settings/waybar/style.css" "$SKEL/waybar/" 2>/dev/null || true
cp "$PROJECT_ROOT/indos-settings/swaync/config.json" "$SKEL/swaync/" 2>/dev/null || true
cp "$PROJECT_ROOT/indos-settings/swaync/style.css" "$SKEL/swaync/" 2>/dev/null || true

# Step 5: Build ISO
echo ""
echo "[5/5] Building ISO image..."
mkdir -p "$OUT_DIR"
mkarchiso -v -w "$WORK_DIR" -o "$OUT_DIR" "$ISO_PROFILE"

echo ""
echo "═══════════════════════════════════════════"
echo " IndOS ISO built successfully!"
echo " Base:   CachyOS (x86-64-v3)"
echo " Kernel: linux-cachyos (BORE)"
echo "═══════════════════════════════════════════"
ls -lh "$OUT_DIR"/indos-*.iso 2>/dev/null
