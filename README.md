<p align="center">
  <img src="https://img.shields.io/badge/🇮🇳_Made_in_India-FF9933?style=for-the-badge" alt="Made in India" />
  <img src="https://img.shields.io/badge/AI--First_OS-7B2FF7?style=for-the-badge&logo=linux&logoColor=white" alt="AI-First OS" />
  <img src="https://img.shields.io/badge/Written_in_Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Written in Rust" />
  <img src="https://img.shields.io/badge/Built_on_Arch-1793D1?style=for-the-badge&logo=archlinux&logoColor=white" alt="Built on Arch" />
  <img src="https://img.shields.io/badge/100%25_Open_Source-3DA639?style=for-the-badge&logo=opensourceinitiative&logoColor=white" alt="Open Source" />
  <img src="https://img.shields.io/badge/License-GPL_3.0-blue?style=for-the-badge" alt="GPL-3.0" />
</p>

# 🇮🇳 IndOS — India's Sovereign AI-First Operating System with Rust

> **The first operating system where the conversation *is* the desktop.**
> Written in Rust. Built on Arch Linux. Powered by open-source generative AI. Made in India, for the world.

![IndOS Desktop](assets/screenshot.png)

---

## Why IndOS?

Every major OS today bolts AI on top of a 40-year-old desktop metaphor — icons, menus, file managers, app launchers. Microsoft adds Copilot. Apple adds Siri. Google adds Gemini. Same old desktop, new sidebar.

**IndOS throws all of that away.**

When you boot IndOS, there are no desktop icons, no start menus, no settings panels. There is one thing: **a surface that understands what you want and generates whatever you need, right now.** You express intent. The OS generates the right interface. The thing gets done.

This is India's contribution to the next generation of computing — a **sovereign, open-source, AI-native operating system** written from the ground up in **Rust**. No C/C++ legacy. No patching decades-old code. A clean, memory-safe, blazing-fast foundation that runs locally, respects your privacy, and doesn't depend on any single corporation or cloud.

---

## Core Principles

| Principle | What it means |
|-----------|---------------|
| 🇮🇳 **Sovereign** | Built in India. No vendor lock-in. No foreign cloud dependency for core functionality. |
| 🤖 **AI-First** | AI isn't a feature — it's the entire interface. The conversation is the OS. |
| 🦀 **Written in Rust** | Core OS layer built in Rust — memory-safe, blazing fast, zero-cost abstractions. No C/C++ legacy. |
| 🏠 **Local-First** | Your desktop works offline. Your data stays on your machine. Local models by default. |
| 🔓 **Open Source** | GPL-3.0. Every component is open. Built on Arch Linux and OSS generative AI. |
| 🛡️ **Privacy by Design** | PII never leaves your device. API calls go through a privacy filter. You control everything. |

---

## How It Works

```
Human → expresses intent → OS generates the right interface → thing is done
```

**No app switching.** No menu diving. No settings panels.

- Say *"fix the auth bug in my project"* → IndOS finds the repo, opens the relevant files, dispatches a coding agent, shows you the diff. One tap to apply, test, commit.
- Say *"my disk is filling up"* → IndOS scans usage, renders a treemap, identifies caches and duplicates, each with a "Clean" button.
- Say *"play something chill"* → a minimal media control appears. No music player app. Just the controls you need.

---

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
│  Local LLMs     │ │ MCP Tools   │ │  Voice Pipeline    │
│  (Ollama)       │ │ (7 tools)   │ │  (STT + TTS)       │
│  Llama, Qwen,   │ │ files, cmds │ │  Faster-Whisper    │
│  Gemma, DeepSeek│ │ packages    │ │  + Piper           │
└────────────────┘ └─────────────┘ └────────────────────┘
          │
┌─────────▼──────────────────────────────────────────────┐
│  Linux (Arch / Niri compositor / PipeWire / systemd)    │
└────────────────────────────────────────────────────────┘
```

---

## Open-Source AI Stack

IndOS is built entirely on open-source generative AI — no proprietary models required:

| Layer | Technology | Role |
|-------|-----------|------|
| **Inference Runtime** | [Ollama](https://ollama.com) / llama.cpp | Local model serving, GPU management |
| **Language Models** | Llama, Qwen, Gemma, DeepSeek, Phi | Conversation, reasoning, code generation |
| **Embeddings** | nomic-embed-text | Semantic memory and vector search |
| **Vector Database** | LanceDB | Embedded, zero-config — "SQLite for vectors" |
| **Speech-to-Text** | Faster-Whisper | GPU-accelerated, ~100ms, fully offline |
| **Text-to-Speech** | Piper | <100ms latency, CPU-only, offline |
| **Vision** | Qwen2.5-VL | Desktop screenshot understanding |
| **Model Selection** | llmfit | Hardware scan → optimal model recommendation |

**API keys are optional.** IndOS runs fully offline with local models. Add API keys for frontier models (Claude, GPT) when you need extra capability — all API calls pass through a local privacy filter first.

---

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

---

## Fragment System (A2UI)

IndOS renders dynamic UI components called **fragments** — the OS generates the right interface for every task:

| Tier | Renderer | Latency | Example |
|------|----------|---------|---------|
| **Tier 1** | Native Iced widgets | <1ms | System monitor, terminal, text |
| **Tier 2** | Webview (Bun/Tambo) | ~50ms | Rich editors, data tables |
| **Tier 3** | Generative (OpenUI) | ~500ms | Novel one-off UIs, AI-designed |

Fragments graduate: Tier 3 → Tier 2 → Perry (TS→native) → Tier 1.

---

## MCP Tools

The orchestrator exposes 7 tools to the LLM via [Model Context Protocol](https://modelcontextprotocol.io/):

| Tool | Description | Safety |
|------|-------------|--------|
| `list_files` | List directory contents | ✅ Read-only |
| `read_file` | Read file contents (line-limited) | ✅ Read-only |
| `write_file` | Write to file | ⚠️ Blocks system paths |
| `run_command` | Execute shell command | ⚠️ Blocks destructive patterns |
| `system_info` | CPU, RAM, disk, GPU, uptime | ✅ Read-only |
| `search_files` | Find files by pattern | ✅ Read-only |
| `package_manager` | Search, info, install, remove | 🔒 Install/remove needs confirmation |

---

## Model Routing

The orchestrator auto-selects the best model based on task complexity:

| Task | Model Tier | Examples |
|------|-----------|----------|
| Chat / Q&A | Small (<2B) | qwen2.5:0.5b, phi3:mini |
| Information / Reasoning | Medium (7-9B) | qwen2.5:7b, llama3.1:8b |
| Coding | Code-specialized | qwen2.5-coder:7b, codellama |
| Vision / GUI | Vision models | qwen2.5-vl, UI-TARS |

---

## Tech Stack

| Category | Technology |
|----------|-----------|
| **Language** | Rust (core), Python (STT/TTS), TypeScript (fragments) |
| **Base** | Arch Linux (CachyOS kernel, BORE scheduler) |
| **Compositor** | Niri (Rust + Smithay, scrollable tiling Wayland) |
| **Shell** | Iced 0.14 + iced_layershell 0.18 (Wayland layer-shell) |
| **Audio** | PipeWire (via cpal) |
| **Display Manager** | greetd + tuigreet |
| **Inference** | Ollama (local models, fully offline) |
| **Memory** | LanceDB + nomic-embed-text |
| **Voice** | Faster-Whisper (STT) + Piper (TTS) |
| **Protocol** | MCP (Model Context Protocol) / NDJSON over Unix socket |
| **Packages** | pacman (Arch Linux) + AUR |

---

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

### Prerequisites

- Arch Linux or CachyOS
- Rust toolchain (stable, via rustup)
- Node.js 20+ (for fragment development)
- Ollama (for local model testing)

---

## Status

🟢 **v1.0 Complete** — IndOS is ready for release!

- ✅ Orchestrator running, IPC verified
- ✅ Shell compiles (layer-shell on Niri)
- ✅ Context engine (LanceDB) wired
- ✅ Voice pipeline (Faster-Whisper + Piper)
- ✅ Model routing implemented
- ✅ MCP tools (7 system tools with safety guards)
- ✅ ISO profile ready (CachyOS kernel + Niri)
- ✅ Privacy filter + security sandbox (26/26 tests)
- ✅ Tool execution loop + session persistence (9/9 tests)
- ✅ Agent harness (OpenCode, Claude Code, Codex CLI)
- ✅ Full live test on hardware

See [ROADMAP.md](ROADMAP.md) for the full implementation plan.

---

## The Name

**IndOS** — **Ind**ia's **O**perating **S**ystem.

Sovereign. Independent. Not owned by a corporation. Not locked to a cloud. Not dependent on any single AI provider.

Built from India 🇮🇳, for the world 🌏.

---

## Contributing

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

## License

GPL-3.0 — See [LICENSE](LICENSE)

---

<p align="center">
  <sub>🇮🇳 A sovereign open-source project. Built with ❤️ in India.</sub>
</p>
