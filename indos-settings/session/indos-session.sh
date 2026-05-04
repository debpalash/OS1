#!/bin/bash
# IndOS Desktop Session
# Starts Niri compositor → IndOS shell → orchestrator → Waybar → SwayNC
#
# This is the session entry point, registered as a .desktop file
# in /usr/share/wayland-sessions/ for display managers (greetd, etc.)
#
# Session manager: UWSM (systemd-aware, crash recovery)

set -euo pipefail

# === Environment ===
export XDG_CURRENT_DESKTOP=IndOS
export XDG_SESSION_TYPE=wayland
export XDG_SESSION_DESKTOP=indos

# Wayland for all toolkits
export QT_QPA_PLATFORM=wayland
export GDK_BACKEND=wayland
export MOZ_ENABLE_WAYLAND=1
export SDL_VIDEODRIVER=wayland
export CLUTTER_BACKEND=wayland

# IndOS paths
export INDOS_CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/indos"
export INDOS_DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/indos"
export INDOS_SETTINGS="/usr/share/indos/settings"

# Ensure directories exist
mkdir -p "$INDOS_CONFIG_DIR"
mkdir -p "$INDOS_DATA_DIR"
mkdir -p "$INDOS_DATA_DIR/context"
mkdir -p "$INDOS_DATA_DIR/sessions"

# === Deploy default configs if not present ===

# Niri
if [ ! -f "$HOME/.config/niri/config.kdl" ]; then
    mkdir -p "$HOME/.config/niri"
    cp "$INDOS_SETTINGS/niri/config.kdl" "$HOME/.config/niri/config.kdl" 2>/dev/null || true
fi

# Waybar
if [ ! -d "$HOME/.config/waybar" ]; then
    mkdir -p "$HOME/.config/waybar"
    cp "$INDOS_SETTINGS/waybar/config.jsonc" "$HOME/.config/waybar/config.jsonc" 2>/dev/null || true
    cp "$INDOS_SETTINGS/waybar/style.css" "$HOME/.config/waybar/style.css" 2>/dev/null || true
fi

# SwayNC
if [ ! -d "$HOME/.config/swaync" ]; then
    mkdir -p "$HOME/.config/swaync"
    cp "$INDOS_SETTINGS/swaync/config.json" "$HOME/.config/swaync/config.json" 2>/dev/null || true
    cp "$INDOS_SETTINGS/swaync/style.css" "$HOME/.config/swaync/style.css" 2>/dev/null || true
fi

# Install indos-waybar helper to PATH
mkdir -p "$HOME/.local/bin"
if [ ! -f "$HOME/.local/bin/indos-waybar" ]; then
    cp "$INDOS_SETTINGS/waybar/indos-waybar.sh" "$HOME/.local/bin/indos-waybar" 2>/dev/null || true
    chmod +x "$HOME/.local/bin/indos-waybar" 2>/dev/null || true
fi
export PATH="$HOME/.local/bin:$PATH"

# === Pre-launch: Start background services ===

# 1. Start Ollama (if not running via systemd)
if ! systemctl --user is-active ollama.service &>/dev/null; then
    if ! pgrep -x ollama > /dev/null 2>&1; then
        echo "[IndOS] Starting Ollama inference server..."
        ollama serve &
        sleep 1
    fi
fi

# 2. Start IndOS orchestrator (systemd user service)
if systemctl --user is-enabled indos-orchestrator.service > /dev/null 2>&1; then
    systemctl --user start indos-orchestrator.service
    echo "[IndOS] Orchestrator started via systemd."
else
    echo "[IndOS] Starting orchestrator directly..."
    indos-orchestrator &
    sleep 0.5
fi

# 3. Start voice pipeline (optional)
if [ -f "$INDOS_CONFIG_DIR/voice.enabled" ]; then
    echo "[IndOS] Starting voice pipeline..."
    indos-voice &
fi

# === Launch Niri compositor ===
# Niri's config.kdl spawns: indos-shell, waybar, swaync at startup
#
# If UWSM is available, use it for crash recovery + session management
if command -v uwsm &>/dev/null; then
    echo "[IndOS] Launching Niri via UWSM (crash recovery enabled)..."
    exec uwsm start niri
else
    echo "[IndOS] Launching Niri directly..."
    exec niri
fi
