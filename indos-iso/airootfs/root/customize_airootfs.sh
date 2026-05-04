#!/bin/bash
# IndOS ISO post-build script — runs inside the chroot during ISO build
# CachyOS base with IndOS generative desktop

set -euo pipefail

echo "[IndOS] Post-build customization starting..."

# === CREATE DEFAULT USER ===
useradd -m -G wheel,video,audio,input -s /bin/zsh indos
echo "indos:indos" | chpasswd
echo "%wheel ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers.d/wheel

# === ENABLE SERVICES ===
systemctl enable NetworkManager
systemctl enable greetd

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
mkdir -p /etc/greetd
cat > /etc/greetd/config.toml << 'EOF'
[terminal]
vt = 1

[default_session]
command = "tuigreet --remember --time --greeting 'Welcome to IndOS' --sessions /usr/share/wayland-sessions"
user = "greeter"
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
cat > /etc/systemd/user/ollama-user.service << 'EOF'
[Unit]
Description=Ollama LLM Server (User)
Before=indos-orchestrator.service

[Service]
ExecStart=/usr/bin/ollama serve
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
EOF

# === INSTALL VOICE PIPELINE DEPS ===
echo "[IndOS] Installing voice pipeline..."
pip install --break-system-packages faster-whisper 2>/dev/null || true
# Piper: install from AUR or binary
if ! command -v piper &>/dev/null; then
    echo "[IndOS] Piper not available yet — will install on first boot"
fi

# === PRE-PULL OLLAMA MODEL ===
if command -v ollama &>/dev/null; then
    echo "[IndOS] Pre-pulling qwen2.5:0.5b..."
    ollama serve &
    OLLAMA_PID=$!
    sleep 5
    ollama pull qwen2.5:0.5b || echo "[IndOS] Model pull failed (offline build?)"
    ollama pull nomic-embed-text || echo "[IndOS] Embedding model pull failed"
    kill $OLLAMA_PID 2>/dev/null || true
    wait $OLLAMA_PID 2>/dev/null || true
fi

# === SET BINARY PERMISSIONS ===
chmod +x /usr/local/bin/indos-orchestrator 2>/dev/null || true
chmod +x /usr/local/bin/indos-shell 2>/dev/null || true
chmod +x /usr/local/bin/indos-session 2>/dev/null || true

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
HOME_URL="https://github.com/user/IndOS"
DOCUMENTATION_URL="https://github.com/user/IndOS/wiki"
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

echo "[IndOS] Post-build complete. CachyOS base + IndOS generative desktop."
