#!/bin/bash
# IndOS ISO post-build script — runs inside the chroot during ISO build
# Sets up default user, enables services, pre-pulls AI model

set -euo pipefail

# === CREATE DEFAULT USER ===
useradd -m -G wheel,video,audio,input -s /bin/zsh indos
echo "indos:indos" | chpasswd
echo "%wheel ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers.d/wheel

# === ENABLE SERVICES ===
systemctl enable NetworkManager
systemctl enable greetd

# === CONFIGURE GREETD ===
cat > /etc/greetd/config.toml << 'EOF'
[terminal]
vt = 1

[default_session]
command = "tuigreet --remember --time --greeting 'Welcome to IndOS' --sessions /usr/share/wayland-sessions"
user = "greeter"
EOF

# === INSTALL INDOS SESSION ===
# (binaries are copied in by airootfs overlay)
chmod +x /usr/local/bin/indos-orchestrator 2>/dev/null || true
chmod +x /usr/local/bin/indos-shell 2>/dev/null || true
chmod +x /usr/local/bin/indos-waybar 2>/dev/null || true
chmod +x /usr/local/bin/indos-session 2>/dev/null || true

# === INSTALL SYSTEMD USER SERVICE ===
mkdir -p /etc/systemd/user
cat > /etc/systemd/user/indos-orchestrator.service << 'EOF'
[Unit]
Description=IndOS Orchestrator
After=default.target

[Service]
ExecStart=/usr/local/bin/indos-orchestrator
Restart=on-failure
RestartSec=3
RuntimeDirectory=indos

[Install]
WantedBy=default.target
EOF

# === PRE-PULL OLLAMA MODEL (if network available during build) ===
if command -v ollama &>/dev/null; then
    echo "[IndOS] Pre-pulling qwen2.5:0.5b..."
    ollama serve &
    OLLAMA_PID=$!
    sleep 3
    ollama pull qwen2.5:0.5b || echo "[IndOS] Model pull failed (offline build?)"
    kill $OLLAMA_PID 2>/dev/null || true
fi

# === SET DEFAULT CONFIGS ===
# Configs are deployed via airootfs/etc/skel/ overlay

# === BRANDING ===
echo "IndOS Generative Desktop" > /etc/hostname
cat > /etc/os-release << 'EOF'
NAME="IndOS"
PRETTY_NAME="IndOS Generative Desktop"
ID=indos
ID_LIKE=arch
BUILD_ID=rolling
HOME_URL="https://github.com/user/IndOS"
EOF

echo "[IndOS] Post-build complete."
