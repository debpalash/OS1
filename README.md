<p align="center">
  <img src="assets/desktop.png" alt="IndOS Desktop — The Generative Desktop" width="800" />
</p>

<h1 align="center">🇮🇳 IndOS</h1>

<p align="center">
  <strong>India's Sovereign AI-First Operating System</strong><br/>
  <em>The first OS where the conversation <b>is</b> the desktop.</em>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/🇮🇳_Made_in_India-FF9933?style=for-the-badge" alt="Made in India" />
  <img src="https://img.shields.io/badge/AI--Native_OS-7B2FF7?style=for-the-badge&logo=linux&logoColor=white" alt="AI-Native OS" />
  <img src="https://img.shields.io/badge/Written_in_Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Written in Rust" />
  <img src="https://img.shields.io/badge/Built_on_Arch-1793D1?style=for-the-badge&logo=archlinux&logoColor=white" alt="Built on Arch" />
  <img src="https://img.shields.io/badge/100%25_Local--First-3DA639?style=for-the-badge&logo=opensourceinitiative&logoColor=white" alt="Local-First" />
  <img src="https://img.shields.io/badge/License-GPL_3.0-blue?style=for-the-badge" alt="GPL-3.0" />
</p>

<p align="center">
  <a href="#quick-start">Quick Start</a> •
  <a href="#the-idea">The Idea</a> •
  <a href="#how-it-works">How It Works</a> •
  <a href="#architecture">Architecture</a> •
  <a href="#screenshots">Screenshots</a> •
  <a href="#roadmap">Roadmap</a> •
  <a href="#contributing">Contributing</a>
</p>

---

## The Idea

Every major OS today bolts AI on top of a 40-year-old desktop metaphor. Microsoft adds Copilot. Apple adds Siri. Google adds Gemini. Same old desktop, new sidebar.

**IndOS is something different.**

When you boot IndOS, there are no desktop icons. No start menus. No settings panels. There is one thing: **a surface that understands what you want and generates whatever you need, right now.**

```
Traditional:  Human → clicks icon → opens app → navigates menus → does thing
AI-Assisted:  Human → opens assistant → types request → assistant helps with app
IndOS:        Human → expresses intent → OS generates the right interface → done
```

You don't use apps. You express intent. The OS figures out the rest.

This is India's contribution to the next generation of computing — a **sovereign, open-source, AI-native operating system** written from the ground up in **Rust**. No C/C++ legacy. No patching decades-old code. A clean, memory-safe, blazing-fast foundation that runs locally, respects your privacy, and doesn't depend on any single corporation or cloud.

---

## What Makes IndOS Different

| | Traditional OS | AI-Assisted OS | IndOS |
|---|---|---|---|
| **Interface** | Windows, icons, menus | Same + chat sidebar | Conversation generates UI |
| **AI Role** | None | Copilot / assistant | The AI **is** the shell |
| **Data** | Local files + cloud sync | Cloud-dependent AI | 100% local inference |
| **Privacy** | Varies | Data sent to cloud | PII never leaves device |
| **Architecture** | C/C++ (1970s–90s) | Same + Python/JS AI | Rust from the ground up |

---

## How It Works

**Say what you need. IndOS makes it happen.**

- *"Fix the auth bug in my project"* → IndOS finds the repo, opens relevant files, dispatches a coding agent, shows the diff. One tap to apply, test, commit.
- *"My disk is filling up"* → Scans usage, renders a treemap, identifies caches and duplicates, each with a "Clean" button.
- *"Show me system resources"* → A live system monitor fragment appears inline. No htop. No app.
- *"Play something chill"* → Minimal media controls materialize. No music player. Just what you need.

The OS doesn't open applications — it **generates fragments**: purpose-built UI components that appear when needed and disappear when done.

---

## Screenshots

<table>
  <tr>
    <td align="center"><img src="assets/boot.png" alt="IndOS Boot" width="380" /><br/><em>Custom bootloader with IndOS branding</em></td>
    <td align="center"><img src="assets/desktop.png" alt="IndOS Desktop" width="380" /><br/><em>Generative Desktop with AI status bar</em></td>
  </tr>
  <tr>
    <td align="center" colspan="2"><img src="assets/installer.png" alt="IndOS Installer" width="380" /><br/><em>Calamares installer with Niri tiling</em></td>
  </tr>
</table>

---

## Architecture

IndOS is a 10-layer stack, from bare metal to generative shell:

```
┌──────────────────────────────────────────────────────────────────┐
│  GENERATIVE SHELL (Iced + Wayland layer-shell)                    │
│  Conversation Canvas  │  Fragment Renderer  │  AI Status Bar      │
├──────────────────────────────────────────────────────────────────┤
│  VOICE I/O                                                        │
│  Faster-Whisper (STT)  │  Piper (TTS)  │  PipeWire               │
├──────────────────────────────────────────────────────────────────┤
│  ORCHESTRATOR — The Brain                                         │
│  Intent Router → Model Selection → Agent Dispatch → Tool Exec    │
│  Session Persistence  │  Context Engine (LanceDB)                 │
├────────────┬────────────┬─────────────┬──────────────────────────┤
│  MODEL     │  AGENT     │  MCP TOOLS  │  PRIVACY FILTER           │
│  FABRIC    │  HARNESS   │  12 system  │  PII redaction            │
│  Ollama    │  OpenCode  │  tools with │  before any API call      │
│  llmfit    │  Claude    │  sandboxing │                            │
├────────────┴────────────┴─────────────┴──────────────────────────┤
│  CONTEXT ENGINE                                                    │
│  LanceDB + nomic-embed-text  │  Semantic memory across sessions   │
├──────────────────────────────────────────────────────────────────┤
│  SECURITY ENGINE                                                   │
│  Capability-based sandbox  │  Audit log  │  Path/command filtering│
├──────────────────────────────────────────────────────────────────┤
│  FRAGMENT LIBRARY (A2UI)                                           │
│  Terminal │ File Browser │ Code View │ Charts │ Markdown │ Forms  │
├──────────────────────────────────────────────────────────────────┤
│  DESKTOP INTEGRATION                                               │
│  Niri (compositor) │ Waybar │ SwayNC │ PipeWire │ NetworkManager  │
├──────────────────────────────────────────────────────────────────┤
│  LINUX — CachyOS kernel / BORE scheduler / x86-64-v3              │
└──────────────────────────────────────────────────────────────────┘
```

---

## Core Principles

| Principle | What it means |
|-----------|---------------|
| 🇮🇳 **Sovereign** | Built in India. No vendor lock-in. No foreign cloud dependency for core functionality. |
| 🤖 **AI-Native** | AI isn't a feature — it's the entire interface. The conversation is the OS. |
| 🦀 **Written in Rust** | Memory-safe, blazing fast. Shell, orchestrator, and all core services in Rust. |
| 🏠 **Local-First** | Works fully offline. Your data stays on your machine. Local models by default. |
| 🛡️ **Privacy by Design** | PII never leaves your device. All API calls pass through a local privacy filter first. |
| 🔓 **Open Source** | GPL-3.0. Every component is open. No proprietary dependencies required. |

---

## Open-Source AI Stack

IndOS runs entirely on open-source AI — no proprietary models required:

| Layer | Technology | Role |
|-------|-----------|------|
| **Inference** | [Ollama](https://ollama.com) / llama.cpp | Local model serving, GPU management |
| **Language Models** | Llama, Qwen, Gemma, DeepSeek, Phi | Conversation, reasoning, code generation |
| **Embeddings** | nomic-embed-text | Semantic memory and vector search |
| **Vector DB** | LanceDB | Embedded, zero-config — "SQLite for vectors" |
| **Speech-to-Text** | Faster-Whisper | GPU-accelerated, ~100ms, fully offline |
| **Text-to-Speech** | Piper | <100ms latency, CPU-only, offline |
| **Vision** | Qwen2.5-VL | Desktop screenshot understanding |
| **Model Selection** | llmfit | Hardware scan → optimal model recommendation |

> **API keys are optional.** IndOS runs fully offline with local models. Add API keys for frontier models (Claude, GPT-4) when you want extra capability — all API calls pass through the local privacy filter first.

---

## Project Structure

```
IndOS/
├── indos-orchestrator/      # 🧠 The Brain — intent routing, Ollama streaming, tool execution
├── indos-shell/             # 🖥️  Generative shell — Iced 0.14 + Wayland layer-shell
├── indos-context-engine/    # 🔍 LanceDB vector memory + Ollama embeddings
├── indos-voice/             # 🎙️  Voice I/O — Faster-Whisper STT + Piper TTS
├── indos-privacy/           # 🛡️  PII redaction filter
├── indos-security/          # 🔒 Capability-based security engine + audit log
├── indos-canvas/            # 🎨 Conversation rendering (shared types)
├── indos-settings/          # ⚙️  Niri, Waybar, SwayNC default configs
├── indos-iso/               # 📀 archiso profile → bootable Live ISO
├── autotest/                # 🧪 QEMU-based automated test harness
└── .github/workflows/       # 🔄 CI — automated ISO builds
```

---

## Quick Start

### Try IndOS (Live USB)

1. Download the latest ISO from [Releases](https://github.com/debpalash/IndOS/releases)
2. Flash to USB: `sudo dd if=indos-*.iso of=/dev/sdX bs=4M status=progress`
3. Boot from USB
4. Start talking to your OS

### Build from Source

```bash
# Clone
git clone https://github.com/debpalash/IndOS.git
cd IndOS

# Build the Rust crates
cargo build --release -p indos-orchestrator
cargo build --release -p indos-shell

# Build the bootable ISO (needs archiso + root)
sudo ./indos-iso/build-iso.sh

# Run automated tests against the ISO
python3 autotest/autotest.py
```

### Prerequisites

- Arch Linux or CachyOS host (for ISO builds)
- Rust toolchain (stable, via rustup)
- Ollama (for local AI inference)
- archiso (`pacman -S archiso`) for ISO builds

---

## MCP Tools

The orchestrator exposes 12 tools to the LLM, all sandboxed by the Security Engine:

| Tool | Description | Safety |
|------|-------------|--------|
| `list_files` | List directory contents | ✅ Read-only |
| `read_file` | Read file contents (line-limited) | ✅ Read-only |
| `write_file` | Write to file | ⚠️ Blocks system paths |
| `run_command` | Execute shell command | ⚠️ Blocks destructive patterns |
| `system_info` | CPU, RAM, disk, GPU, uptime | ✅ Read-only |
| `search_files` | Find files by pattern | ✅ Read-only |
| `package_manager` | Search, info, install, remove | 🔒 Install/remove needs confirmation |
| `system_setting` | Volume, brightness control | ⚠️ Non-destructive |
| `toggle_focus_mode` | Enable/disable DND | ✅ Safe |
| `hw_info` | Detailed hardware diagnostics | ✅ Read-only |
| `launch_installer` | Start Calamares installer | 🔒 Privileged |
| `donut_fetch` | Fetch URL content | ⚠️ Network |

---

## Model Routing

The orchestrator auto-selects the best local model based on task complexity:

| Task | Model Tier | Examples |
|------|-----------|----------|
| Quick chat / Q&A | Small (<2B) | `qwen2.5:0.5b`, `phi3:mini` |
| Information / Reasoning | Medium (7–9B) | `qwen2.5:7b`, `llama3.1:8b` |
| Coding | Code-specialized | `qwen2.5-coder:7b`, `codellama` |
| Vision / GUI | Vision models | `qwen2.5-vl`, `UI-TARS` |

---

## Tech Stack

| Category | Technology |
|----------|-----------|
| **Language** | Rust (core OS), Python (STT/TTS), TypeScript (fragments) |
| **Base** | Arch Linux (CachyOS kernel, BORE scheduler) |
| **Compositor** | Niri — Rust + Smithay, scrollable tiling Wayland |
| **Shell** | Iced 0.14 + iced_layershell 0.18 (Wayland layer-shell) |
| **Audio** | PipeWire |
| **Display Manager** | greetd + tuigreet |
| **Inference** | Ollama (local, fully offline) |
| **Memory** | LanceDB + nomic-embed-text |
| **Voice** | Faster-Whisper (STT) + Piper (TTS) |
| **Protocol** | MCP / NDJSON over Unix socket |
| **Packages** | pacman (Arch Linux) + AUR |

---

## Roadmap

### ✅ v1.0 — Foundation (Complete)

- [x] Orchestrator with intent classification and model routing
- [x] Generative shell (Iced + Wayland layer-shell)
- [x] Context engine with LanceDB semantic memory
- [x] 12 MCP tools with security sandbox
- [x] Voice pipeline (Faster-Whisper + Piper)
- [x] Privacy filter with PII redaction
- [x] Session persistence across reboots
- [x] Agent harness (OpenCode, Claude Code, Codex CLI)
- [x] Bootable Live ISO with Calamares installer
- [x] Custom bootloader branding
- [x] AI-aware Waybar status bar
- [x] QEMU-based automated test suite (22 stages)
- [x] GitHub Actions CI for ISO builds

### 🔜 v2.0 — Intelligence

- [ ] A2UI fragment generation — LLM emits JSON, OS renders native UI
- [ ] Screenpipe integration — 24/7 screen context for proactive assistance
- [ ] llmfit — automatic model recommendation based on hardware scan
- [ ] Proactive morning dashboard — OS generates daily briefing on boot
- [ ] Multi-agent orchestration — parallel agent dispatch for complex tasks
- [ ] Donut Browser integration — MCP-native browsing with per-profile isolation

### 🔮 v3.0 — Autonomy

- [ ] Self-healing system — OS detects and resolves its own failures
- [ ] Workflow builder (Sim Studio) — visual agentic workflow designer
- [ ] ARM64 / Apple Silicon support (Asahi Linux kernel)
- [ ] Federated learning — opt-in distributed model improvement
- [ ] IndOS App Store — community-built fragment marketplace
- [ ] Multilingual voice — Hindi, Tamil, Telugu, Bengali, and 20+ Indian languages

---

## The Name

**IndOS** — **Ind**ia's **O**perating **S**ystem.

Sovereign. Independent. Not owned by a corporation. Not locked to a cloud. Not dependent on any single AI provider.

Built from India 🇮🇳, for the world 🌏.

---

## Contributing

We welcome contributions of all kinds — code, documentation, translations, testing, and ideas.

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and guidelines.

```bash
# Fork → Clone → Branch → Code → PR
git clone https://github.com/YOUR_USERNAME/IndOS.git
cd IndOS
git checkout -b feature/your-feature
# Make changes
git push origin feature/your-feature
# Open a Pull Request
```

---

## License

GPL-3.0 — See [LICENSE](LICENSE)

---

<p align="center">
  <sub>🇮🇳 A sovereign open-source project. Built with ❤️ in India.</sub><br/>
  <sub>Star ⭐ this repo if you believe the desktop deserves to be reinvented.</sub>
</p>
