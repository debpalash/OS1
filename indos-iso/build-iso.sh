#!/usr/bin/env bash
# Build IndOS ISO
# Usage: ./build-iso.sh [--work-dir /tmp/indos-build] [--out-dir /tmp/]
#
# Prerequisites:
# - archiso package installed (pacman -S archiso)
# - Run as root (sudo)
# - Release binaries built (cargo build --release in each crate)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
ISO_PROFILE="$SCRIPT_DIR"
WORK_DIR="${1:-/tmp/indos-build}"
OUT_DIR="${2:-/tmp/indos-out}"

echo "═══════════════════════════════════════════"
echo " IndOS ISO Builder"
echo "═══════════════════════════════════════════"
echo " Profile: $ISO_PROFILE"
echo " Work:    $WORK_DIR"
echo " Output:  $OUT_DIR"
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

# Step 1: Build release binaries
echo ""
echo "[1/4] Building release binaries..."
(cd "$PROJECT_ROOT/indos-orchestrator" && cargo build --release)
(cd "$PROJECT_ROOT/indos-shell" && cargo build --release)

# Step 2: Copy binaries into airootfs overlay
echo ""
echo "[2/4] Installing binaries into ISO overlay..."
mkdir -p "$ISO_PROFILE/airootfs/usr/local/bin"
cp "$PROJECT_ROOT/indos-orchestrator/target/release/indos-orchestrator" \
   "$ISO_PROFILE/airootfs/usr/local/bin/"
cp "$PROJECT_ROOT/indos-shell/target/release/indos-shell" \
   "$ISO_PROFILE/airootfs/usr/local/bin/"

# Copy session script
cp "$PROJECT_ROOT/indos-settings/session/indos-session.sh" \
   "$ISO_PROFILE/airootfs/usr/local/bin/indos-session"

# Step 3: Copy configs into skel (default user configs)
echo ""
echo "[3/4] Installing default configs..."
cp "$PROJECT_ROOT/indos-settings/niri/config.kdl" \
   "$ISO_PROFILE/airootfs/etc/skel/.config/niri/"
cp "$PROJECT_ROOT/indos-settings/waybar/config.jsonc" \
   "$ISO_PROFILE/airootfs/etc/skel/.config/waybar/"
cp "$PROJECT_ROOT/indos-settings/waybar/style.css" \
   "$ISO_PROFILE/airootfs/etc/skel/.config/waybar/" 2>/dev/null || true
cp "$PROJECT_ROOT/indos-settings/swaync/config.json" \
   "$ISO_PROFILE/airootfs/etc/skel/.config/swaync/" 2>/dev/null || true
cp "$PROJECT_ROOT/indos-settings/swaync/style.css" \
   "$ISO_PROFILE/airootfs/etc/skel/.config/swaync/" 2>/dev/null || true

# Step 4: Build ISO
echo ""
echo "[4/4] Building ISO image..."
mkdir -p "$OUT_DIR"
mkarchiso -v -w "$WORK_DIR" -o "$OUT_DIR" "$ISO_PROFILE"

echo ""
echo "═══════════════════════════════════════════"
echo " ISO built successfully!"
echo " Output: $OUT_DIR/indos-*.iso"
echo "═══════════════════════════════════════════"
ls -lh "$OUT_DIR"/indos-*.iso 2>/dev/null
