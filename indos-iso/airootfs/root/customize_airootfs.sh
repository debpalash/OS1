#!/bin/bash
# IndOS ISO post-build script — runs inside the chroot during ISO build
# CachyOS base with IndOS generative desktop

set -euo pipefail

echo "[IndOS] Post-build customization starting..."

# NOTE: User creation is at the END of this script
# so that /etc/skel is fully populated first.
echo "%wheel ALL=(ALL:ALL) NOPASSWD: ALL" > /etc/sudoers.d/wheel
chmod 440 /etc/sudoers.d/wheel

# === ENABLE SERVICES ===
systemctl enable NetworkManager
systemctl enable greetd
systemctl enable sshd

# NetworkManager owns networking — systemd-networkd-wait-online otherwise
# fails on the live ISO (no networkd config) and degrades boot state
systemctl disable systemd-networkd.service systemd-networkd.socket \
    systemd-networkd-wait-online.service 2>/dev/null || true
systemctl mask systemd-networkd-wait-online.service

# === CACHYOS PERFORMANCE TUNING ===
# Enable ananicy (auto process priority — AI workloads get CPU priority)
systemctl enable ananicy-cpp 2>/dev/null || true

# Enable zram (compressed swap — more usable RAM for ML models)
if command -v zram-generator &>/dev/null; then
    mkdir -p /etc/systemd/zram-generator.conf.d
    cat > /etc/systemd/zram-generator.conf.d/indos.conf << 'EOF'
[zram0]
zram-size = ram / 2
compression-algorithm = zstd
EOF
fi

# === CONFIGURE GREETD ===
# Live ISO: auto-login directly into Niri (no login prompt)
# After install, Calamares post-install hook switches to tuigreet
mkdir -p /etc/greetd
cat > /etc/greetd/config.toml << 'EOF'
[terminal]
vt = 1

[default_session]
command = "indos-session"
user = "indos"
EOF

# === INSTALL SYSTEMD USER SERVICES ===
mkdir -p /etc/systemd/user

# Orchestrator service (starts with user session)
cat > /etc/systemd/user/indos-orchestrator.service << 'EOF'
[Unit]
Description=IndOS Orchestrator
After=default.target

[Service]
ExecStart=/usr/local/bin/indos-orchestrator
Restart=on-failure
RestartSec=3
RuntimeDirectory=indos
Environment=RUST_LOG=info

[Install]
WantedBy=default.target
EOF

# Ollama as user service (starts before orchestrator)
# Models stored in user-writable dir (not /usr/share which is root-owned)
cat > /etc/systemd/user/ollama-user.service << 'EOF'
[Unit]
Description=Ollama LLM Server (User)
Before=indos-orchestrator.service

[Service]
Environment=OLLAMA_MODELS=%h/.ollama/models
ExecStartPre=/bin/mkdir -p %h/.ollama/models
ExecStart=/usr/bin/ollama serve
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
EOF

# === ENABLE USER SERVICES GLOBALLY ===
# --global enables for ALL users (works in chroot, unlike --user)
systemctl --global enable ollama-user.service
systemctl --global enable indos-orchestrator.service

# === INSTALL VOICE PIPELINE DEPS ===
echo "[IndOS] Installing voice pipeline..."
pip install --break-system-packages faster-whisper 2>/dev/null || true
# Piper: install from AUR or binary
if ! command -v piper &>/dev/null; then
    echo "[IndOS] Piper not available yet — will install on first boot"
fi

# Ollama models will be pulled on first boot, not during ISO build
# This saves ~700MB and 40s+ per build cycle

# === PATCH CALAMARES UNPACKFS FOR PYTHON 3.14 ===
# Boost.Python's host_env_process_output is broken on Python 3.14
# Replace with subprocess-based wrapper that streams output and returns exit code
UNPACKFS="/usr/lib/calamares/modules/unpackfs/main.py"
if [ -f "$UNPACKFS" ] && grep -q 'host_env_process_output' "$UNPACKFS"; then
    echo "[IndOS] Patching unpackfs for Python 3.14 compatibility..."
    cp "$UNPACKFS" "${UNPACKFS}.orig"

    # Create the patch as a separate file for readability
    cat > /tmp/unpackfs_patch.py << 'PYPATCH'
import subprocess as _sp

# Python 3.14 compat: replace broken Boost.Python host_env_process_output
# Must stream output line-by-line (for rsync progress) and return exit code
def _safe_process_output(cmd, callback):
    proc = _sp.Popen(cmd, stdout=_sp.PIPE, stderr=_sp.PIPE, text=True)
    for line in proc.stdout:
        callback(line.strip())
    proc.wait()
    return proc.returncode

PYPATCH

    # Prepend the patch to the module
    cat /tmp/unpackfs_patch.py "$UNPACKFS" > /tmp/unpackfs_patched.py
    mv /tmp/unpackfs_patched.py "$UNPACKFS"
    # Replace all calls to host_env_process_output with our safe version
    sed -i 's|libcalamares\.utils\.host_env_process_output|_safe_process_output|g' "$UNPACKFS"
    echo "[IndOS] unpackfs patched: streaming subprocess with exit code return"
fi

# === SET BINARY PERMISSIONS ===
chmod +x /usr/local/bin/indos-orchestrator 2>/dev/null || true
chmod +x /usr/local/bin/indos-shell 2>/dev/null || true
chmod +x /usr/local/bin/indos-session 2>/dev/null || true
chmod +x /usr/local/bin/indos-waybar 2>/dev/null || true
chmod +x /usr/local/bin/llmfit 2>/dev/null || true

# === PRE-CONFIGURE SYSTEM (prevent systemd-firstboot interactive wizard) ===
# Without these, systemd-firstboot blocks boot with timezone/locale prompts

# Locale
echo "en_US.UTF-8 UTF-8" > /etc/locale.gen
locale-gen
echo "LANG=en_US.UTF-8" > /etc/locale.conf

# Timezone (UTC — user changes via Calamares installer or settings)
ln -sf /usr/share/zoneinfo/UTC /etc/localtime

# Keymap
echo "KEYMAP=us" > /etc/vconsole.conf

# X11 keyboard layout (prevents systemd-localed 'custom' fallback)
mkdir -p /etc/X11/xorg.conf.d
cat > /etc/X11/xorg.conf.d/00-keyboard.conf << 'EOF'
Section "InputClass"
    Identifier "system-keyboard"
    MatchIsKeyboard "on"
    Option "XkbLayout" "us"
    Option "XkbModel" "pc105"
EndSection
EOF

# Machine ID (generate now, prevents firstboot trigger)
systemd-machine-id-setup 2>/dev/null || true

# === BRANDING ===
cat > /etc/hostname << 'EOF'
indos
EOF

cat > /etc/os-release << 'EOF'
NAME="IndOS"
PRETTY_NAME="IndOS Generative Desktop"
ID=indos
ID_LIKE=arch cachyos
BUILD_ID=rolling
VARIANT="Generative Desktop"
VARIANT_ID=desktop
HOME_URL="https://github.com/debpalash/IndOS"
DOCUMENTATION_URL="https://github.com/debpalash/IndOS/wiki"
LOGO=indos-logo
EOF

# === DEFAULT SHELL CONFIG ===
cat > /etc/skel/.zshrc << 'ZSHEOF'
# IndOS default shell
eval "$(starship init zsh)"

# Aliases
alias ls='ls --color=auto'
alias ll='ls -la'
alias sysinfo='fastfetch'
alias indos-status='echo '\''{"type":"status"}'\'' | socat - UNIX-CONNECT:$XDG_RUNTIME_DIR/indos/orchestrator.sock'

# IndOS environment
export EDITOR=nano
export VISUAL=nano
export TERM=foot
ZSHEOF

# cachyos-calamares installs configs to /etc/calamares/ and /usr/share/calamares/
# We keep CachyOS's battle-tested module configs and only override:
#   1. settings.conf (switch to offline mode + IndOS branding)
#   2. IndOS branding (alongside CachyOS branding)
#   3. Specific modules we need to customize (post-install, unpackfs, bootloader)
echo "[IndOS] Installing Calamares config overlay..."

# Debug: show what CachyOS installed
echo "[IndOS] CachyOS Calamares configs found:"
[ -f /etc/calamares/settings.conf ] && echo "  /etc/calamares/settings.conf ✓" || echo "  /etc/calamares/settings.conf ✗"
[ -d /usr/share/calamares/branding/cachyos ] && echo "  branding/cachyos ✓" || echo "  branding/cachyos ✗"
echo "  $(find /etc/calamares/modules -name '*.conf' 2>/dev/null | wc -l) module configs in /etc/calamares/modules/"
echo "  $(find /usr/share/calamares/modules -maxdepth 1 -type d 2>/dev/null | wc -l) module dirs in /usr/share/calamares/modules/"

# 1. Override settings.conf in both locations (offline mode + IndOS branding)
cp -f /root/indos-calamares/settings.conf /etc/calamares/settings.conf
cp -f /root/indos-calamares/settings.conf /usr/share/calamares/settings.conf 2>/dev/null || true

# 2. Install IndOS branding (alongside CachyOS — don't remove CachyOS's)
mkdir -p /etc/calamares/branding
cp -rf /root/indos-calamares/branding/indos /etc/calamares/branding/
mkdir -p /usr/share/calamares/branding
cp -rf /root/indos-calamares/branding/indos /usr/share/calamares/branding/ 2>/dev/null || true

# 3. Only override specific module configs that IndOS needs differently
#    Keep CachyOS defaults for: partition, locale, keyboard, users, etc.
for module_conf in shellprocess_indos.conf shellprocess_preinitcpio.conf \
                   unpackfs.conf bootloader.conf \
                   welcome.conf services-systemd.conf displaymanager.conf \
                   removeuser.conf finished.conf users.conf fstab.conf \
                   locale.conf partition.conf; do
    if [ -f "/root/indos-calamares/modules/$module_conf" ]; then
        cp -f "/root/indos-calamares/modules/$module_conf" "/etc/calamares/modules/$module_conf"
        echo "  Overlaid: $module_conf"
    fi
done

echo "[IndOS] Calamares overlay complete:"
ls -la /etc/calamares/settings.conf
ls -d /etc/calamares/branding/indos /usr/share/calamares/branding/indos 2>/dev/null

# Cleanup staging dir
rm -rf /root/indos-calamares

# === INSTALLER DESKTOP ENTRY (live session only) ===
# Autostart Calamares when the live session begins
mkdir -p /etc/skel/.config/autostart
cat > /etc/skel/.config/autostart/calamares.desktop << 'EOF'
[Desktop Entry]
Type=Application
Name=Install IndOS
Comment=Install IndOS Generative Desktop to disk
Exec=sh -c 'sleep 3 && sudo -E calamares'
Icon=calamares
Terminal=false
Categories=System;
EOF

# Also place on desktop for manual launch
mkdir -p /etc/skel/Desktop
cp /etc/skel/.config/autostart/calamares.desktop /etc/skel/Desktop/
chmod +x /etc/skel/Desktop/calamares.desktop

# === NIRI WAYLAND SESSION (for greetd) ===
mkdir -p /usr/share/wayland-sessions
cat > /usr/share/wayland-sessions/indos-niri.desktop << 'EOF'
[Desktop Entry]
Name=IndOS (Niri)
Comment=IndOS Generative Desktop — Niri Compositor
Exec=indos-session
Type=Application
DesktopNames=IndOS
EOF

# === CREATE DEFAULT USER (must be AFTER skel is populated) ===
useradd -m -G wheel,video,audio,input -s /bin/zsh indos
echo "indos:indos" | chpasswd
echo "root:indos" | chpasswd

# === DEV SSH KEY (passwordless access from host) ===
mkdir -p /home/indos/.ssh
chmod 700 /home/indos/.ssh
# Inject builder's public key if available
if [ -f /root/dev-ssh-key.pub ]; then
    cp /root/dev-ssh-key.pub /home/indos/.ssh/authorized_keys
    chmod 600 /home/indos/.ssh/authorized_keys
    chown -R indos:indos /home/indos/.ssh
    echo "[IndOS] Dev SSH key injected for passwordless access"
fi

echo "[IndOS] Post-build complete. CachyOS base + IndOS generative desktop + Calamares installer."
