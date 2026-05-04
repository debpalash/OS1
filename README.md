# IndOS

**The ChatGPT moment for Desktop OS.**

An AI-first, agentic-first, local-first Linux distribution where the conversation IS the desktop.

---

## What is IndOS?

IndOS is a complete Linux operating system that replaces the traditional desktop metaphor (icons, menus, file managers, app launchers) with an AI-driven generative interface. You talk to your computer. It generates whatever you need to see, right now.

- No app switching. No menu diving. No settings panels.
- Express intent → OS generates the right interface → thing is done.
- Local-first: your desktop works offline, your data stays on your machine.
- Agent harness: orchestrates OpenCode, Claude Code, Codex CLI, Crush, and more.
- Built on Arch Linux with CachyOS performance optimizations.

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                   IndOS Generative Shell                  │
│            (Wayland client — the primary UI)              │
│                                                           │
│  ┌─────────────┐ ┌──────────────┐ ┌───────────────────┐ │
│  │ Conversation │ │  UI Fragment  │ │  Ambient Surface  │ │
│  │   Canvas     │ │  Renderer     │ │  (contextual)     │ │
│  └──────┬──────┘ └──────┬───────┘ └───────────────────┘ │
└─────────┼───────────────┼────────────────────────────────┘
          │               │
┌─────────▼───────────────▼────────────────────────────────┐
│                  IndOS Orchestrator                        │
│    Intent Classification → Agent Routing → Session Mgmt   │
└─────────┬───────────────┬───────────────┬────────────────┘
          │               │               │
┌─────────▼──────┐ ┌─────▼─────┐ ┌──────▼───────────────┐
│  Model Fabric   │ │ MCP Server │ │  Agent Harness       │
│  (Ollama/APIs)  │ │ (OS tools) │ │  (OpenCode/Claude/…) │
└────────────────┘ └───────────┘ └──────────────────────┘
          │
┌─────────▼────────────────────────────────────────────────┐
│              Linux (CachyOS Kernel / Wayland / systemd)   │
└──────────────────────────────────────────────────────────┘
```

## Repository Map

### Foundation
| Repo | Description |
|------|-------------|
| [`indos-kernel`](indos-kernel/) | CachyOS-based kernel config with AI workload optimizations |
| [`indos-iso`](indos-iso/) | archiso profile for bootable ISO |
| [`indos-pkgbuilds`](indos-pkgbuilds/) | Custom package build scripts |
| [`indos-settings`](indos-settings/) | System defaults (sysctl, udev, schedulers) |
| [`indos-installer`](indos-installer/) | AI-guided conversation-based installer |

### AI Layer
| Repo | Description |
|------|-------------|
| [`indos-shell`](indos-shell/) | **The generative desktop shell** — core product |
| [`indos-orchestrator`](indos-orchestrator/) | Agent routing daemon (systemd service) |
| [`indos-model-fabric`](indos-model-fabric/) | Local + API model management |
| [`indos-mcp-server`](indos-mcp-server/) | MCP server exposing OS capabilities to agents |
| [`indos-agent-harness`](indos-agent-harness/) | Unified agent abstraction (OpenCode, Claude Code, etc.) |
| [`indos-context-engine`](indos-context-engine/) | User pattern learning + semantic memory |

### Desktop Experience
| Repo | Description |
|------|-------------|
| [`indos-fragments`](indos-fragments/) | UI component library (file browser, terminal, editor, etc.) |
| [`indos-themes`](indos-themes/) | Theme definitions |
| [`indos-welcome`](indos-welcome/) | First-boot experience |

## Tech Stack

- **Language:** Rust (core), TypeScript (fragments/UI)
- **Kernel:** Linux 6.x (CachyOS BORE/EEVDF scheduler)
- **Display:** Wayland (cosmic-comp compositor)
- **Audio:** PipeWire
- **Packages:** pacman + IndOS custom repo
- **Inference:** Ollama (local), API providers (optional)
- **Protocol:** MCP (Model Context Protocol)

## Status

🚧 **Pre-alpha** — Foundation scaffolding in progress.

## License

GPL-3.0 — See [LICENSE](LICENSE)
