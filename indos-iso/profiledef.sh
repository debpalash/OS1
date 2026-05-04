#!/usr/bin/env bash
# IndOS ISO Profile — archiso-based
# Build: mkarchiso -v -w /tmp/indos-build -o /tmp/ /path/to/indos-iso/
#
# This creates a bootable ISO with:
# - Niri compositor + IndOS shell (layer-shell)
# - Waybar + SwayNC + IndOS configs
# - Ollama pre-installed with qwen2.5:0.5b pre-pulled
# - Faster-Whisper + Piper for voice
# - greetd display manager → IndOS session

set -euo pipefail

# Profile metadata
iso_name="indos"
iso_label="INDOS_$(date +%Y%m%d)"
iso_publisher="IndOS Project"
iso_application="IndOS Generative Desktop"
iso_version="$(date +%Y.%m.%d)"
install_dir="arch"
buildmodes=('iso')
bootmodes=('bios.syslinux.mbr' 'bios.syslinux.eltorito'
            'uefi-ia32.grub.esp' 'uefi-x64.grub.esp'
            'uefi-ia32.grub.eltorito' 'uefi-x64.grub.eltorito')
arch="x86_64"
pacman_conf="pacman.conf"
airootfs_image_type="squashfs"
airootfs_image_tool_options=('-comp' 'zstd' '-Xcompression-level' '15')
file_permissions=(
    ["/usr/local/bin/indos-orchestrator"]="0:0:755"
    ["/usr/local/bin/indos-shell"]="0:0:755"
    ["/usr/local/bin/indos-waybar"]="0:0:755"
    ["/usr/local/bin/indos-session"]="0:0:755"
)
