#!/bin/bash
# IndOS Desktop Session
# Starts Niri compositor with IndOS environment
#
# This is the session entry point for greetd.
# Niri's config.kdl handles spawning: foot, waybar, swaync, calamares

# NO set -euo pipefail — session must always reach niri regardless of failures

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
mkdir -p "$INDOS_CONFIG_DIR" "$INDOS_DATA_DIR" 2>/dev/null || true

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

# === Launch Niri compositor ===
# Niri's config.kdl spawns: foot, waybar, swaync, calamares at startup.
# niri-session (not raw niri) activates systemd graphical-session.target —
# without it xdg-desktop-portal fails and portal-dependent apps break.
if command -v niri-session >/dev/null; then
    exec niri-session
fi
exec niri
