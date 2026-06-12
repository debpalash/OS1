#!/usr/bin/env bash
# OS 1 ISO Profile — CachyOS-based archiso
# Build: sudo bash ./indos-iso/build-iso.sh
#
# Base: CachyOS (linux-cachyos, BORE scheduler, x86-64-v3)
# Filesystem: ext4 (universal, simple, fast)
# Ships: Niri + OS 1 shell + Ollama + voice pipeline + greetd

set -euo pipefail

# Profile metadata
iso_name="os1"
iso_label="OS1_$(date +%Y%m%d)"
iso_publisher="OS 1 Project <https://github.com/debpalash/OS1>"
iso_application="OS 1 Generative Desktop"
iso_version="$(date +%Y.%m.%d)"
install_dir="arch"
buildmodes=('iso')
# UEFI systemd-boot disabled: Bash 5.3 breaks mkarchiso's du-based FAT sizing
# The installed system gets GRUB EFI via Calamares — this only affects live ISO boot
bootmodes=('bios.syslinux')
arch="x86_64"
pacman_conf="pacman.conf"

# Live ISO rootfs — squashfs with zstd (best compression ratio)
airootfs_image_type="squashfs"
airootfs_image_tool_options=('-comp' 'zstd' '-Xcompression-level' '15')

# Installed system uses ext4
rootfs_image_type="ext4"

file_permissions=(
    ["/usr/local/bin/indos-orchestrator"]="0:0:755"
    ["/usr/local/bin/indos-shell"]="0:0:755"
    ["/usr/local/bin/indos-voiced"]="0:0:755"
    ["/usr/local/bin/indos-session"]="0:0:755"
    ["/usr/local/bin/indos-preinitcpio-repair"]="0:0:755"
    ["/root/customize_airootfs.sh"]="0:0:755"
)
