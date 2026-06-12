#!/usr/bin/env bash
# Root build wrapper for the autonomous loop.
# Installed in sudoers (NOPASSWD) so the agent can rebuild without a human.
#
# Usage: sudo /home/pal/Desktop/IndOS/autotest/build.sh [--clean|--quick]
#
#   --clean   Wipe work dir, full rebuild (~5-10 min)
#   --quick   Skip pacstrap if packages unchanged, only repack airootfs (~1-2 min)
#   (default) Incremental mkarchiso — reuses existing pacstrap if work dir exists
#
# Logs to test-logs/build-<timestamp>.log and prints the log path + ISO path.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_DIR="$ROOT/test-logs"
mkdir -p "$LOG_DIR"
LOG="$LOG_DIR/build-$(date +%Y%m%d-%H%M%S).log"
WORK="$ROOT/build/work"

MODE="${1:-incremental}"

case "$MODE" in
    --clean)
        echo "Cleaning work dir..."
        { findmnt -rno TARGET | grep -F "$WORK" || true; } | sort -r | while read -r m; do
            umount -l "$m" 2>/dev/null || true
        done
        rm -rf "$WORK"
        ;;
    --quick)
        echo "Quick rebuild (repack only)..."
        # Only delete the squashfs + ISO, keep the pacstrap rootfs intact
        rm -f "$WORK"/iso/arch/x86_64/airootfs.sfs 2>/dev/null || true
        rm -f "$WORK"/iso/arch/x86_64/airootfs.sha512 2>/dev/null || true
        # Drop mkarchiso's run-once stamps for everything downstream of
        # the pacstrap, otherwise it skips those steps and ships a stale
        # ISO (boot configs, squashfs, and the ISO image are all gated
        # on these zero-byte stamp files in $WORK). Keep
        # base._make_boot_on_iso9660: it copies kernel+initramfs out of
        # airootfs /boot, which _cleanup_pacstrap_dir already purged —
        # re-running it fails, and its outputs never change on a repack.
        rm -f "$WORK"/base._make_bootmode_* \
              "$WORK"/base._mkairootfs_squashfs \
              "$WORK"/base._prepare_airootfs_image \
              "$WORK"/iso._build_iso_image \
              "$WORK"/build._build_buildmode_iso 2>/dev/null || true
        # (restore in case an older quick run deleted it)
        [ -d "$WORK" ] && touch "$WORK"/base._make_boot_on_iso9660 2>/dev/null || true
        # Delete the ISO lock so mkarchiso regenerates it
        rm -f "$WORK"/.lock* 2>/dev/null || true
        ;;
    *)
        echo "Incremental build..."
        ;;
esac

# --- Cargo cache: skip rebuild if sources unchanged ---
CARGO_HASH_FILE="$ROOT/build/.cargo_hash"
CURRENT_HASH=$(find "$ROOT/indos-orchestrator/src" "$ROOT/indos-shell/src" \
    "$ROOT/indos-voice/src" \
    "$ROOT/indos-orchestrator/Cargo.toml" "$ROOT/indos-shell/Cargo.toml" \
    "$ROOT/indos-voice/Cargo.toml" \
    -newer "$CARGO_HASH_FILE" 2>/dev/null | head -1 || echo "changed")

if [[ "$MODE" == "--quick" && -z "$CURRENT_HASH" && \
      -f "$ROOT/indos-orchestrator/target/release/indos-orchestrator" && \
      -f "$ROOT/indos-shell/target/release/indos-shell" && \
      -f "$ROOT/indos-voice/target/release/indos-voiced" ]]; then
    echo "Cargo binaries up to date, skipping compile"
    export INDOS_SKIP_CARGO=1
fi

# --- Package cache: use host pacman cache ---
export PACMAN_CACHE="/var/cache/pacman/pkg/"

echo "Build log: $LOG"
START=$(date +%s)
set +e
bash "$ROOT/indos-iso/build-iso.sh" >"$LOG" 2>&1
RC=$?
set -e
END=$(date +%s)
ELAPSED=$((END - START))

if [[ $RC -ne 0 ]]; then
    echo "BUILD FAILED (rc=$RC, ${ELAPSED}s). Last 60 lines:"
    tail -n 60 "$LOG"
    exit $RC
fi

# Update cargo hash tracker
mkdir -p "$ROOT/build"
touch "$CARGO_HASH_FILE"

ISO=$(ls -t "$ROOT"/build/out/*.iso | head -1)
# Make artifacts readable/removable by the user
chown "$(stat -c %U:%G "$ROOT")" "$ISO" "$LOG" 2>/dev/null || true
echo "BUILD OK: $ISO (${ELAPSED}s)"
tail -n 5 "$LOG"
