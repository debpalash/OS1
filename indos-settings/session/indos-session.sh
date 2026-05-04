#!/bin/bash
# IndOS Desktop Session
# Starts Niri compositor → IndOS shell → orchestrator
#
# This is the session entry point, registered as a .desktop file
# in /usr/share/wayland-sessions/ for display managers (greetd, etc.)

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
export INDOS_ORCHESTRATOR_SOCKET="/run/indos/orchestrator.sock"

# Ensure directories exist
mkdir -p "$INDOS_CONFIG_DIR"
mkdir -p "$INDOS_DATA_DIR"
mkdir -p "$INDOS_DATA_DIR/context"
mkdir -p "$INDOS_DATA_DIR/sessions"

# === Pre-launch: Start background services ===

# 1. Start Ollama (if not running)
if ! pgrep -x ollama > /dev/null 2>&1; then
    echo "[IndOS] Starting Ollama inference server..."
    ollama serve &
    sleep 1
fi

# 2. Start IndOS orchestrator (systemd user service)
if systemctl --user is-enabled indos-orchestrator.service > /dev/null 2>&1; then
    systemctl --user start indos-orchestrator.service
    echo "[IndOS] Orchestrator started."
fi

# 3. Start IndOS voice pipeline (optional, if enabled)
if [ -f "$INDOS_CONFIG_DIR/voice.enabled" ]; then
    echo "[IndOS] Starting voice pipeline..."
    indos-voice &
fi

# 4. Start Screenpipe (optional, if enabled)
if [ -f "$INDOS_CONFIG_DIR/screenpipe.enabled" ]; then
    echo "[IndOS] Starting screen context daemon..."
    screenpipe &
fi

# === Launch Niri compositor ===
# IndOS shell will be started by Niri via spawn-at-startup
echo "[IndOS] Launching Niri compositor..."
exec niri
