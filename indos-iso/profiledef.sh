#!/usr/bin/env bash
# IndOS ISO Profile — CachyOS-based archiso
# Build: mkarchiso -v -w /tmp/indos-build -o /tmp/ /path/to/indos-iso/
#
# Base: CachyOS (linux-cachyos, BORE scheduler, x86-64-v3)
# Filesystem: ext4 (universal, simple, fast — no subvolume complexity)
# Ships: Niri + IndOS shell + Ollama + voice pipeline + greetd

set -euo pipefail

# Profile metadata
iso_name="indos"
iso_label="INDOS_$(date +%Y%m%d)"
iso_publisher="IndOS Project <https://github.com/user/IndOS>"
iso_application="IndOS Generative Desktop"
iso_version="$(date +%Y.%m.%d)"
install_dir="arch"
buildmodes=('iso')
bootmodes=('bios.syslinux.mbr' 'bios.syslinux.eltorito'
            'uefi-ia32.grub.esp' 'uefi-x64.grub.esp'
            'uefi-ia32.grub.eltorito' 'uefi-x64.grub.eltorito')
arch="x86_64"
pacman_conf="pacman.conf"

# Live ISO rootfs — squashfs with zstd (best compression ratio)
airootfs_image_type="squashfs"
airootfs_image_tool_options=('-comp' 'zstd' '-Xcompression-level' '15')

# Installed system uses ext4 — simple, universal, no maintenance overhead
# (btrfs snapshots are nice but add complexity IndOS doesn't need yet)
rootfs_image_type="ext4"

file_permissions=(
    ["/usr/local/bin/indos-orchestrator"]="0:0:755"
    ["/usr/local/bin/indos-shell"]="0:0:755"
    ["/usr/local/bin/indos-session"]="0:0:755"
    ["/root/customize_airootfs.sh"]="0:0:755"
)
