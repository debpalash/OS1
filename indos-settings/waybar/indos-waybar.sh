#!/bin/bash
# indos-waybar — Bridge between IndOS services and Waybar custom modules
# Outputs JSON that Waybar's custom modules consume
#
# Usage: indos-waybar <command>
# Commands: ai-status, voice-status, privacy-zone, model-status

set -euo pipefail

ORCHESTRATOR_SOCKET="${INDOS_ORCHESTRATOR_SOCKET:-/run/indos/orchestrator.sock}"

case "${1:-help}" in

    ai-status)
        # Check if orchestrator is running and what it's doing
        if [ -S "$ORCHESTRATOR_SOCKET" ]; then
            # TODO: Query orchestrator for actual state
            # For now, report based on process state
            if pgrep -x indos-orchestrator > /dev/null 2>&1; then
                echo '{"text": "ready", "class": "idle", "tooltip": "IndOS ready — click to open shell"}'
            else
                echo '{"text": "starting", "class": "thinking", "tooltip": "Orchestrator starting..."}'
            fi
        else
            echo '{"text": "offline", "class": "error", "tooltip": "Orchestrator not running"}'
        fi
        ;;

    voice-status)
        # Check voice pipeline state
        if pgrep -f "indos-voice" > /dev/null 2>&1; then
            # TODO: Query voice pipeline for actual state (listening/speaking/processing)
            echo '{"text": "on", "class": "listening", "tooltip": "Voice active — click to toggle"}'
        else
            echo '{"text": "off", "class": "off", "tooltip": "Voice off — click to enable"}'
        fi
        ;;

    privacy-zone)
        # Show current privacy zone
        # Green = all local, Yellow = API with filter, Red = blocked data present
        if pgrep -x ollama > /dev/null 2>&1; then
            # Check if any API keys are configured
            if [ -f "${XDG_CONFIG_HOME:-$HOME/.config}/indos/api-keys.toml" ]; then
                echo '{"text": "filtered", "class": "yellow", "tooltip": "Privacy: API mode — PII filter active"}'
            else
                echo '{"text": "local", "class": "green", "tooltip": "Privacy: Local only — no data leaves device"}'
            fi
        else
            echo '{"text": "no model", "class": "red", "tooltip": "Privacy: No inference available"}'
        fi
        ;;

    model-status)
        # Show active model
        if pgrep -x ollama > /dev/null 2>&1; then
            # Query Ollama for running model
            RUNNING=$(curl -s http://localhost:11434/api/ps 2>/dev/null | grep -o '"name":"[^"]*"' | head -1 | cut -d'"' -f4)
            if [ -n "$RUNNING" ]; then
                echo "{\"text\": \"$RUNNING\", \"class\": \"active\", \"tooltip\": \"Model: $RUNNING (running)\"}"
            else
                echo '{"text": "idle", "class": "idle", "tooltip": "Ollama running, no model loaded"}'
            fi
        else
            echo '{"text": "offline", "class": "offline", "tooltip": "Ollama not running"}'
        fi
        ;;

    help|*)
        echo "Usage: indos-waybar <ai-status|voice-status|privacy-zone|model-status>"
        exit 1
        ;;
esac
