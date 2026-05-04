#!/bin/bash
# indos-waybar — Waybar custom module data provider
# Called by Waybar's custom modules to get AI/voice/privacy/model status
# Output format: Waybar JSON ({"text":"...", "tooltip":"...", "class":"..."})

ORCHESTRATOR_SOCK="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/indos/orchestrator.sock"

json_output() {
    local text="$1" tooltip="$2" class="$3"
    printf '{"text":"%s","tooltip":"%s","class":"%s"}\n' "$text" "$tooltip" "$class"
}

case "$1" in
    ai-status)
        # Check if orchestrator is running
        if [ -S "$ORCHESTRATOR_SOCK" ]; then
            # Query status via socat
            if command -v socat &>/dev/null; then
                RESP=$(echo '{"type":"status"}' | socat - UNIX-CONNECT:"$ORCHESTRATOR_SOCK" 2>/dev/null | head -1)
                if echo "$RESP" | grep -q '"ollama_connected":true'; then
                    MODEL=$(echo "$RESP" | grep -o '"model":"[^"]*"' | cut -d'"' -f4)
                    json_output "ready" "Model: $MODEL" "idle"
                else
                    json_output "no model" "Ollama disconnected" "error"
                fi
            else
                json_output "ready" "Orchestrator running" "idle"
            fi
        else
            json_output "offline" "Orchestrator not running" "error"
        fi
        ;;

    voice-status)
        # Check if voice pipeline is running
        if pgrep -x "indos-voice" &>/dev/null; then
            json_output "" "Voice active" "listening"
        else
            json_output "" "Voice off" "off"
        fi
        ;;

    privacy-zone)
        # Default to green (all local)
        json_output "" "All data stays local" "green"
        ;;

    model-status)
        # Query current model from orchestrator
        if [ -S "$ORCHESTRATOR_SOCK" ] && command -v socat &>/dev/null; then
            RESP=$(echo '{"type":"status"}' | socat - UNIX-CONNECT:"$ORCHESTRATOR_SOCK" 2>/dev/null | head -1)
            MODEL=$(echo "$RESP" | grep -o '"model":"[^"]*"' | cut -d'"' -f4)
            if [ -n "$MODEL" ]; then
                json_output "$MODEL" "Active model: $MODEL" ""
            else
                json_output "none" "No model loaded" ""
            fi
        else
            json_output "offline" "Orchestrator not running" ""
        fi
        ;;

    *)
        echo "Usage: indos-waybar {ai-status|voice-status|privacy-zone|model-status}" >&2
        exit 1
        ;;
esac
