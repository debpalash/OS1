# IndOS

**The ChatGPT moment for Desktop OS.**

An AI-first, agentic-first, local-first Linux distribution where the conversation IS the desktop.

---

## What is IndOS?

IndOS replaces the traditional desktop metaphor — icons, menus, file managers, app launchers — with an AI-driven generative interface. You talk to your computer. It generates whatever you need.

- **No app switching.** No menu diving. No settings panels.
- **Express intent → OS generates the right interface → thing is done.**
- **Local-first:** your desktop works offline, your data stays on your machine.
- **Agent harness:** orchestrates coding agents, browser automation, and system tools.
- **Built on Arch Linux** with Niri (Rust Wayland compositor).

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                   IndOS Generative Shell                  │
│         (Iced + Wayland layer-shell — the primary UI)    │
│                                                           │
│  ┌─────────────┐ ┌──────────────┐ ┌───────────────────┐ │
│  │ Conversation │ │  UI Fragment  │ │  System Monitor   │ │
│  │   Canvas     │ │  (Tier 1/2/3) │ │  Terminal Embed   │ │
│  └──────┬──────┘ └──────┬───────┘ └───────────────────┘ │
└─────────┼───────────────┼────────────────────────────────┘
          │ Unix Socket   │ NDJSON
┌─────────▼───────────────▼────────────────────────────────┐
│                  IndOS Orchestrator                        │
│    Intent → Model Router → Agent Dispatch → Tool Exec    │
│    Context Engine (LanceDB) ← remembers everything       │
└─────────┬───────────┬───────────────┬────────────────────┘
          │           │               │
┌─────────▼──────┐ ┌──▼──────────┐ ┌─▼──────────────────┐
│  Ollama LLM     │ │ MCP Tools   │ │  Voice Pipeline    │
│  (local models)  │ │ (7 tools)   │ │  (STT + TTS)       │
│  Model routing   │ │ files, cmds │ │  Faster-Whisper    │
│  S/M/L/Code/Vis │ │ packages    │ │  + Piper           │
└────────────────┘ └─────────────┘ └────────────────────┘
          │
┌─────────▼──────────────────────────────────────────────┐
│  Linux (Arch / Niri compositor / PipeWire / systemd)    │
└────────────────────────────────────────────────────────┘
```

## Crate Map

| Crate | Lines | Description |
|-------|-------|-------------|
| [`indos-shell`](indos-shell/) | ~440 | Generative desktop shell — Iced 0.14 + iced_layershell 0.18 |
| [`indos-orchestrator`](indos-orchestrator/) | ~600 | Brain — IPC server, Ollama streaming, tool execution, context engine |
| [`indos-context-engine`](indos-context-engine/) | ~390 | LanceDB vector memory + Ollama embeddings |
| [`indos-voice`](indos-voice/) | ~400 | Voice I/O — Faster-Whisper STT + Piper TTS pipeline |
| [`indos-privacy`](indos-privacy/) | ~50 | PII redaction filter (pre-API) |
| [`indos-security`](indos-security/) | ~50 | Security policy engine |
| [`indos-canvas`](indos-canvas/) | ~50 | Conversation rendering (shared types) |
| [`indos-settings`](indos-settings/) | — | Niri, Waybar, SwayNC configs |
| [`indos-iso`](indos-iso/) | — | archiso profile → bootable ISO |

## Fragment System (A2UI)

IndOS renders dynamic UI components called **fragments** in a tiered system:

| Tier | Renderer | Latency | Example |
|------|----------|---------|---------|
| **Tier 1** | Native Iced widgets | <1ms | System monitor, terminal, text |
| **Tier 2** | Webview (Bun/Tambo) | ~50ms | Rich editors, data tables |
| **Tier 3** | Generative (OpenUI) | ~500ms | Novel one-off UIs, AI-designed |

Fragments graduate: Tier 3 → Tier 2 → Perry (TS→native) → Tier 1.

## MCP Tools

The orchestrator exposes 7 tools to the LLM:

| Tool | Description | Safety |
|------|-------------|--------|
| `list_files` | List directory contents | ✅ Read-only |
| `read_file` | Read file contents (line-limited) | ✅ Read-only |
| `write_file` | Write to file | ⚠️ Blocks system paths |
| `run_command` | Execute shell command | ⚠️ Blocks destructive patterns |
| `system_info` | CPU, RAM, disk, GPU, uptime | ✅ Read-only |
| `search_files` | Find files by pattern | ✅ Read-only |
| `package_manager` | Search, info, install, remove | 🔒 Install/remove needs confirmation |

## Model Routing

The orchestrator auto-selects the best model based on task complexity:

| Task | Model Tier | Examples |
|------|-----------|----------|
| Chat / Q&A | Small (<2B) | qwen2.5:0.5b, phi3:mini |
| Information / Reasoning | Medium (7-9B) | qwen2.5:7b, llama3.1:8b |
| Coding | Code-specialized | qwen2.5-coder:7b, codellama |
| Vision / GUI | Vision models | qwen2.5-vl, UI-TARS, Fara-7B |

## Tech Stack

- **Language:** Rust (core), Python (STT/TTS), TypeScript (fragments)
- **Compositor:** Niri (Rust + Smithay, scrollable tiling)
- **Shell:** Iced 0.14 + iced_layershell 0.18 (Wayland layer-shell)
- **Audio:** PipeWire (via cpal)
- **Display Manager:** greetd + tuigreet
- **Inference:** Ollama (local models)
- **Memory:** LanceDB (embedded vector DB) + nomic-embed-text
- **Voice:** Faster-Whisper (STT) + Piper (TTS)
- **Protocol:** MCP (Model Context Protocol) / NDJSON over Unix socket
- **Packages:** pacman (Arch Linux)

## Building

```bash
# Build all crates
cargo build --release -p indos-orchestrator
cargo build --release -p indos-shell

# Run orchestrator (needs Ollama running)
./indos-orchestrator/target/release/indos-orchestrator

# Build bootable ISO (needs archiso + sudo)
sudo ./indos-iso/build-iso.sh
```

## Status

🟡 **M2 Complete** — Core infrastructure built and tested.

- ✅ Orchestrator running, IPC verified
- ✅ Shell compiles (layer-shell on Niri)
- ✅ Context engine (LanceDB) wired
- ✅ Voice pipeline scaffolded
- ✅ Model routing implemented
- ✅ MCP tools (7 system tools)
- ✅ ISO profile ready
- ⏳ Full live test (needs Ollama + Niri)

## License

GPL-3.0 — See [LICENSE](LICENSE)
