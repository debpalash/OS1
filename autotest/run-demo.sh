#!/usr/bin/env bash
# Launch OS 1 interactively in QEMU — window, GPU acceleration, and
# two-way audio (your real mic and speakers pass through), so you can
# talk to it like in HER.
#
# Usage: autotest/run-demo.sh [path/to/os1.iso]
#
# In the guest:
#   - say "hey OS, <anything>"        → spoken reply (needs an ollama
#     model: open the terminal and `ollama pull qwen2.5:3b` once)
#   - Mod+Shift+V                     → push-to-talk, no wake phrase
#   - Mod+Space                       → toggle the conversation shell
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ISO="${1:-$(ls -t "$ROOT"/build/out/os1-*.iso 2>/dev/null | head -1)}"
[ -f "$ISO" ] || { echo "No ISO found — build one first (sudo autotest/build.sh)"; exit 1; }

echo "Booting: $ISO"
echo "Audio: host microphone + speakers passed through (PipeWire)"
echo "Close the window or Ctrl+C here to stop."

exec qemu-system-x86_64 \
    -enable-kvm -machine q35 -cpu host \
    -m 8G -smp 8 \
    -device virtio-vga-gl -display gtk,gl=on \
    -audiodev pipewire,id=snd0 \
    -device virtio-sound-pci,audiodev=snd0 \
    -netdev user,id=n0,hostfwd=tcp:127.0.0.1:2299-:22 \
    -device virtio-net-pci,netdev=n0 \
    -cdrom "$ISO"
