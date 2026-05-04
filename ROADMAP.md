# IndOS — Master Implementation Plan v3

> The ChatGPT moment for Desktop OS.
> AI-first, local-first, generative Linux distribution.

---

## Architecture: 10-Layer Stack

```
┌─────────────────────────────────────────────────────────────────────┐
│  L10  GENERATIVE SHELL (Dioxus layer-shell surface on Niri)         │
│       Conversation Canvas │ A2UI Fragment Renderer │ 3 Modes         │
├─────────────────────────────────────────────────────────────────────┤
│  L9   VOICE I/O                                                      │
│       Faster-Whisper (STT) │ Piper (TTS) │ hyprwhspr (dictation)     │
│       Vision Agents (realtime video AI) │ PipeWire                   │
├─────────────────────────────────────────────────────────────────────┤
│  L8   ORCHESTRATOR (systemd, Cap'n Web RPC)                          │
│       Intent Router │ Agent Dispatch │ Session │ Notifications        │
├────────────┬────────────┬──────────────┬────────────────────────────┤
│  L7  MODEL │  L7 AGENT  │  L7  MCP     │  L7  PRIVACY FILTER        │
│  FABRIC    │  HARNESS   │  SERVER      │  (OpenAI Privacy Filter)    │
│  Ollama    │  OpenCode  │  Filesystem  │  PII redaction before API   │
│  llmfit    │  Claude    │  Packages    │                             │
│  API route │  Codex/Sim │  Services    │                             │
├────────────┴────────────┴──────────────┴────────────────────────────┤
│  L6   CONTEXT ENGINE                                                 │
│       LanceDB │ nomic-embed │ Screenpipe │ Qwen-VL │ Context Hub     │
├─────────────────────────────────────────────────────────────────────┤
│  L5   SECURITY                                                       │
│       Agent Sandbox (bubblewrap) │ Capability Model │ Audit Log      │
├─────────────────────────────────────────────────────────────────────┤
│  L4   SYSTEM AWARENESS                                               │
│       GPU/NPU detect │ Network │ Power │ Multi-Monitor │ llmfit      │
├─────────────────────────────────────────────────────────────────────┤
│  L3   FRAGMENT LIBRARY (A2UI)                                        │
│       Terminal │ File Browser │ Editor │ Monitor │ Charts │ Forms     │
├─────────────────────────────────────────────────────────────────────┤
│  L2   DESKTOP INTEGRATION                                            │
│       Niri (compositor) │ Waybar/ashell │ PipeWire │ NetworkManager   │
│       Donut Browser │ systemd │ Dioxus (TUI+GUI+Web)                 │
├─────────────────────────────────────────────────────────────────────┤
│  L1   LINUX (CachyOS kernel / BORE scheduler / x86-64-v3)           │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Standards & Protocols

| Protocol | Source | Role |
|----------|--------|------|
| **A2UI** | Google | Fragment specification — LLM emits JSON describing UI |
| **AG-UI** | CopilotKit | Bidirectional event transport agent↔shell |
| **MCP** | Anthropic | Tool protocol — OS capabilities exposed to agents |
| **Cap'n Web** | Cloudflare | Object-capability RPC for orchestrator↔fragment IPC |

---

## Technology Decisions (Locked)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Compositor | **Niri** (Rust/Smithay) | Scrollable tiling, 23.7k⭐, IPC via JSON socket, layer-shell |
| Shell Framework | **Iced + `iced_layershell`** (Rust) | Native layer-shell support via waycrate. GPU-accelerated (wgpu). Proven by COSMIC DE + ashell. Pure Rust, no webview dependency for shell chrome |
| Fragment Rendering | **OpenUI** (generative) + **Tambo** (interactable) | OpenUI: LLM → HTML/CSS for ephemeral UI. Tambo: schema-defined persistent components AI can read/update across turns. Both render in embedded webview widget inside iced |
| Status Bar | **Waybar** (primary) + **ashell** (future Rust-native) | Waybar for M1 maturity; ashell (904⭐, iced-rs) for pure-Rust migration |
| Base Distro | **CachyOS** (Arch) | BORE scheduler, x86-64-v3, performance-tuned kernel |
| Voice STT | **Faster-Whisper** | GPU-accelerated, ~100ms, fully offline |
| Voice TTS | **Piper** | <100ms latency, CPU-only, offline |
| Dictation | **hyprwhspr** | Push-to-talk → types into active buffer, Waybar integration |
| Vector DB | **LanceDB** | Embedded, zero-config, "SQLite for vectors" |
| Inference | **Ollama** | Model lifecycle, API compatibility, GPU management |
| Browser | **Donut Browser** | Anti-detect profiles, MCP server, per-profile isolation |
| IPC Protocol | **JSON over Unix socket** (M1), evaluate `tarpc` (M2) | Cap'n Web dropped (JS-only). Rust-native IPC for orchestrator↔shell |
| Privacy | **OpenAI Privacy Filter** | Local PII redaction before API-bound data |
| Screen Context | **Screenpipe** | 24/7 capture, local OCR, REST API |
| Vision | **Qwen2.5-VL** | Desktop screenshot understanding, GUI automation |
| Knowledge Base | **Context Hub** | Curated versioned docs, self-improving annotations |
| Model Fitness | **llmfit** | Hardware scan → model recommendations at first boot |
| Workflow Builder | **Sim Studio** | Visual agentic workflows, self-hostable |
| Vision Agents | **GetStream/Vision-Agents** | Realtime voice+vision AI, YOLO integration |

---

## GitHub Organization: `IndOS/` — 27 Repos

### Tier 1 — Foundation (Fork/Customize)

| # | Repo | What | Based on |
|---|------|------|----------|
| 1 | `indos-kernel` | CachyOS BORE/EEVDF + AI workload priorities (CUDA/ROCm, inference memory) | CachyOS/linux-cachyos |
| 2 | `indos-pkgbuilds` | Custom PKGBUILDs for all IndOS packages | CachyOS/CachyOS-PKGBUILDS |
| 3 | `indos-iso` | archiso profile — live USB/ISO contents | CachyOS/CachyOS-Live-ISO |
| 4 | `indos-settings` | System defaults: sysctl, udev, ananicy-cpp, Niri config, Waybar config | CachyOS/CachyOS-Settings |
| 5 | `indos-installer` | AI-guided conversation-based installer | New (AerynOS/lichen-inspired) |
| 6 | `indos-chwd` | Hardware detection + driver auto-config | CachyOS/chwd fork |

### Tier 2 — AI Core (Build)

| # | Repo | What | Key Tech |
|---|------|------|----------|
| 7 | `indos-shell` | **THE core product.** Generative desktop. Layer-shell surface. Conversation canvas + A2UI renderer | Dioxus 0.7, Rust |
| 8 | `indos-orchestrator` | Intent classification → agent routing → session management | Rust, Cap'n Web, systemd |
| 9 | `indos-model-fabric` | Ollama lifecycle, API vault, local/API routing, inference cache, llmfit integration | Rust |
| 10 | `indos-mcp-server` | MCP server: filesystem, packages, services, network, desktop control | Rust |
| 11 | `indos-agent-harness` | Unified agent abstraction (OpenCode, Claude Code, Codex, Aider, Crush, Sim Studio) | Rust |
| 12 | `indos-privacy` | OpenAI Privacy Filter — PII redaction before API calls | Rust + Python |

### Tier 3 — Intelligence (Build)

| # | Repo | What | Key Tech |
|---|------|------|----------|
| 13 | `indos-context-engine` | LanceDB vectors + nomic-embed + temporal patterns + clipboard + Context Hub | Rust |
| 14 | `indos-voice` | Voice I/O: Faster-Whisper + Piper + PipeWire + hyprwhspr coordination | Rust + Python |
| 15 | `indos-vision` | Screen understanding via Qwen-VL + Vision Agents for realtime video AI | Python + Rust |
| 16 | `indos-screenpipe` | 24/7 screen+audio capture, local OCR, REST API | Rust (Screenpipe fork) |
| 17 | `indos-security` | Agent sandboxing (bubblewrap), capability model, immutable audit log | Rust |

### Tier 4 — Desktop Experience

| # | Repo | What |
|---|------|------|
| 18 | `indos-fragments` | A2UI component library: file browser, terminal, editor, monitor, charts, forms |
| 19 | `indos-terminal` | Embedded terminal (alacritty-core based) |
| 20 | `indos-browser` | Donut Browser integration + MCP profile switching |
| 21 | `indos-wallpapers` | Default visual assets |
| 22 | `indos-themes` | Dark, light, OLED, high-contrast theme definitions |
| 23 | `indos-welcome` | First-boot: AI tour, model download, API key setup ("The Moment") |

### Tier 5 — Docs & Community

| # | Repo | What |
|---|------|------|
| 24 | `indos-canvas` | Design system, UI spec, fragment protocol documentation |
| 25 | `indos-docs` | Documentation website (MDX/Astro) |
| 26 | `indos-website` | Marketing website — indos.dev |
| 27 | `.github` | Org-level README, CONTRIBUTING, templates |

---

## Fragment Architecture (A2UI + AG-UI + OpenUI + Tambo)

```
USER: "show me my files"
  │
  ▼
ORCHESTRATOR (JSON over Unix socket)
  │ classifies intent → "filesystem.browse"
  │ dispatches to Model Fabric
  ▼
LLM generates A2UI JSON:
  { "type": "fragment", "component": "file-browser",
    "props": { "path": "/home/user", "view": "grid" },
    "interactive": true }
  │
  ▼
SHELL (Iced + iced_layershell surface)
  │ looks up "file-browser" in FragmentRegistry
  │
  ├── Tier 1: FOUND in iced widgets → render native component (<50ms)
  │
  ├── Tier 2: FOUND in Tambo registry → render interactable component
  │   (persistent state, AI can read/update across conversation turns)
  │   Renders in embedded webview widget inside iced
  │
  └── Tier 3: NOT FOUND → OpenUI generative fallback
      LLM → HTML/CSS → sandboxed webview → ephemeral fragment
  │
  ▼
User interacts → AG-UI events → Unix socket → Orchestrator
```

**Three fragment tiers:**
- **Tier 1 — Native iced** — Pre-built Rust widgets. Fastest. System monitor, terminal, settings.
- **Tier 2 — Tambo interactable** — Schema-defined components with state. AI reads/updates across turns. File browser, code editor, forms.
- **Tier 3 — OpenUI generative** — LLM generates HTML/CSS on the fly. Ephemeral. Novel/one-off UI that doesn't exist in the registry.

---

## Privacy Architecture

```
User Input → Intent Classification (LOCAL, always)
  │
  ├── 🟢 GREEN ZONE (local-only) → Direct to Ollama
  │   Conversation, context memory, screen capture — never leaves device
  │
  ├── 🟡 YELLOW ZONE (needs API) → [OpenAI Privacy Filter] → PII stripped → API
  │   Complex reasoning, code generation, image generation
  │
  └── 🔴 RED ZONE (blocked) → User-marked "never send" — hardcoded exclusion
```

---

## Voice Pipeline

```
                    ┌─────────────────────────────────────────┐
                    │          CONVERSATION MODE               │
User speaks ──→ PipeWire ──→ Faster-Whisper (GPU, ~100ms)     │
                    │              ↓                            │
                    │     Transcribed text → Orchestrator       │
                    │              ↓                            │
                    │     LLM response → Piper TTS (<50ms)     │
                    │              ↓                            │
                    │     Speaker ← PipeWire                   │
                    │     Total: ~1-2s (fully offline)          │
                    └─────────────────────────────────────────┘

                    ┌─────────────────────────────────────────┐
                    │          DICTATION MODE                   │
Push-to-talk ──→ hyprwhspr ──→ types into active buffer        │
                    │     Waybar indicator shows recording      │
                    └─────────────────────────────────────────┘

                    ┌─────────────────────────────────────────┐
                    │          VISION MODE (optional)           │
Screen feed ──→ Vision Agents ──→ YOLO/Qwen-VL ──→ context    │
                    │     Realtime understanding, <30ms         │
                    └─────────────────────────────────────────┘
```

---

## Agent Harness Strategy

```
Tier 1 — Pre-installed (always available):
├── System Agent (built-in, handles OS: packages, files, services)
├── OpenCode (open-source, any model, default coding agent)
├── indos-mcp-server (OS tool exposure)
└── Sim Studio (visual workflow builder, self-hosted)

Tier 2 — Available with API keys:
├── Claude Code (deep reasoning, escalation agent)
├── Codex CLI (speed-optimized parallel execution)
└── Vision Agents (realtime video AI)

Tier 3 — Optional (user-installed):
├── Aider (git-native pair programming)
├── Crush (beautiful TUI, LSP-aware)
└── Any MCP-compatible agent
```

---

## The ISO: What Ships

### L1-L2: System Foundation
- CachyOS kernel (BORE, x86-64-v3)
- **Niri** compositor (scrollable tiling, layer-shell)
- **Waybar** (AI-aware status bar)
- systemd, PipeWire, NetworkManager
- pacman + IndOS custom repo
- chwd (hardware auto-detection)

### L3-L4: Desktop Layer
- indos-fragments (A2UI component library)
- indos-terminal, indos-themes, indos-wallpapers
- Donut Browser (stealth profiles, MCP)
- Firefox (traditional fallback)
- fuzzel (app launcher fallback)

### L5-L6: Intelligence
- indos-security (bubblewrap sandbox)
- indos-context-engine (LanceDB + nomic-embed)
- indos-screenpipe (optional, opt-in)
- indos-vision (optional, GPU-dependent)
- Context Hub CLI (`chub`)

### L7: Services
- Ollama (pre-installed, models download at first boot)
- llmfit (hardware scan → model recommendations)
- indos-model-fabric, indos-mcp-server
- indos-agent-harness (OpenCode pre-installed)
- indos-privacy (Privacy Filter)

### L8-L10: Shell
- indos-orchestrator (systemd, Cap'n Web)
- indos-voice (Faster-Whisper + Piper + hyprwhspr)
- **indos-shell** (the generative desktop)
- indos-welcome (first-boot experience)

---

## First-Boot: "The Moment"

```
1. Boot → IndOS logo → Niri starts → generative shell surface appears

2. "Hello. I'm IndOS. Setting up your local AI..."
   [llmfit scanning hardware...] → "You have 8GB VRAM — perfect for Llama 3.2 3B"
   [████████████░░░░░░] llama3.2 (2.0 GB)
   "While that downloads, let me learn about you."

3. "What's your name?" → "Pal"

4. "What do you do?"
   [Developer] [Designer] [Student] [Creator] [General]

5. "Languages?" (if Developer)
   [Python] [Rust] [JavaScript] [Go] [All]

6. "API keys?" (optional)
   [Add Anthropic] [Add OpenAI] [Skip — local only]

7. "Enable voice?" (if mic detected)
   [Yes — I want to talk to my OS] [No — keyboard only]

8. "Enable screen awareness?" (privacy-sensitive)
   [Yes — IndOS can see what I'm doing] [No — only when I ask]

9. Model ready. First ambient surface renders.
   User types: "show me my files"
   → File browser fragment appears
   → Jaw drops.
```

---

## Milestones

### M0: Scaffolding ✅ COMPLETE
- [x] Vision document, architecture, technology decisions
- [x] Niri ecosystem research (30 repos catalogued)
- [x] Component list with 58+ dependencies mapped
- [x] Core repo scaffolding (indos-shell, orchestrator, settings, voice, etc.)
- [x] Niri config, Waybar config, systemd service, session script

### M1: Bootable Prototype ✅ MOSTLY COMPLETE
- [x] archiso profile with CachyOS kernel + Niri
- [x] Ollama pre-installed on ISO (+ pre-pull in customize script)
- [x] indos-shell: Iced 0.14 + iced_layershell 0.18 layer-shell surface
- [x] Conversation loop: text input → Ollama streaming → text response → rendered
- [x] Tier 1 A2UI fragments: text block, terminal embed, system monitor
- [x] Waybar with AI modules configured
- [x] indos-orchestrator systemd service (NDJSON over Unix socket IPC)
- [x] IPC verified: status queries + chat messages working end-to-end
- [x] Release binaries: orchestrator 8.4MB, shell 20MB
- [ ] First-boot welcome flow (basic)
- [ ] llmfit hardware scan at first boot
- [ ] Boot and test in QEMU
- **Exit criteria:** Boot ISO → see shell → type question → get answer ← CLOSE

### M2: Voice + Context + Model Routing ✅ MOSTLY COMPLETE
- [x] indos-voice: Faster-Whisper STT subprocess + Piper TTS subprocess
- [x] Voice pipeline: full turn loop (mic → STT → orchestrator IPC → TTS → speaker)
- [x] Voice state machine: Idle/Listening/Transcribing/WaitingForResponse/Speaking
- [x] indos-context-engine: LanceDB + Ollama embeddings (nomic-embed-text)
- [x] Context engine wired into orchestrator (auto-remember every turn)
- [x] Conversation history with semantic search (vector recall)
- [x] Model routing: intent→tier (Small/Medium/Large/Code/Vision)
- [ ] hyprwhspr dictation integration
- [ ] indos-privacy: Privacy Filter for API calls
- [ ] indos-screenpipe: screen context daemon (opt-in)
- [ ] Ambient mode (passive contextual dashboard)
- **Exit criteria:** Voice conversation works offline, context recall works

### M3: Agent Integration 🟡 IN PROGRESS
- [x] Intent classification (keyword-based, 6 categories, 5/5 tests passing)
- [x] MCP tools: 7 tools (list_files, read_file, write_file, run_command, system_info, search_files, package_manager)
- [x] Safety guards: destructive command blocklist, system path write protection
- [x] Tool definitions injected into LLM system prompt
- [ ] Tool call parsing from LLM output + execution loop
- [ ] Agent harness: OpenCode + Claude Code + Codex CLI dispatch
- [ ] Sim Studio workflow integration
- [ ] indos-security: bubblewrap sandbox + capability model
- [ ] Session persistence (survive reboots)
- [ ] Donut Browser MCP profile switching
- **Exit criteria:** "Fix the auth bug" → correct agent dispatched → code edited

### M4: Full Desktop + Vision (Future)
- [ ] Multi-monitor fragment layout via Niri IPC
- [ ] indos-vision: Qwen-VL screen understanding
- [ ] Vision Agents realtime video integration
- [ ] Focus mode (immersive workspace)
- [ ] System management through conversation
- [ ] More fragments: code editor, charts, forms, media player
- [ ] ashell migration evaluation (Rust-native bar)
- [ ] Perry TS→native fragment compilation
- [ ] Fara-7B + UI-TARS computer use integration
- **Exit criteria:** Multi-monitor works, vision context improves responses

### M5: Distribution Release (Future)
- [ ] indos-installer (AI-guided, conversation-based)
- [ ] Hardware detection + auto-config (chwd)
- [ ] OTA updates (RAUC)
- [ ] Theming system (dark, light, OLED, high-contrast)
- [ ] Documentation website (indos-docs)
- [ ] indos-website (indos.dev)
- [ ] Alpha ISO release
- **Exit criteria:** Installs on real hardware, survives 24h daily-driver

---

## Current State (Updated)

| Component | Status | LOC |
|-----------|--------|-----|
| `indos-orchestrator/` | ✅ Built + tested | ~600 |
| `indos-shell/` | ✅ Built (Iced 0.14 + layer-shell) | ~440 |
| `indos-context-engine/` | ✅ Built (LanceDB + embeddings) | ~390 |
| `indos-voice/` | ✅ Built (STT + TTS + pipeline) | ~400 |
| `indos-iso/` | ✅ CachyOS profile ready | ~200 |
| `indos-settings/` | ✅ Niri + Waybar + SwayNC + session | — |
| `indos-privacy/` | Scaffolded | ~50 |
| `indos-security/` | Scaffolded | ~50 |
| `indos-canvas/` | Archived (merged into shell) | — |
| `VISION.md` | Complete | — |
| `ROADMAP.md` | Complete (this file) | — |
| `list` | 58 repos audited + categorized | — |
| **Total Rust** | **4,348 lines** | — |
| **Tests** | **5/5 passing** | — |
| **Git commits** | **11** | — |

---

## Niri Ecosystem Leverage

From [niri.md](file:///home/pal/Desktop/IndOS/niri.md) research:

| Resource | Stars | Use |
|----------|-------|-----|
| noctalia-shell | 6.3k | **Top design reference** — study animations, panels, compositor integration |
| DankMaterialShell | 6.1k | Material design patterns for tiling compositors |
| ashell | 904 | Future Waybar replacement (pure Rust, iced-rs) |
| cachyos-niri-settings | 68 | Upstream config reference (we replace this) |
| niri-animation-collection | 133 | Cherry-pick animation presets |
| niri-scratchpad | 121 | Reference for show/hide overlay patterns |
| Zirconium | 271 | Only other Niri-based OS (Fedora+bootc, not AI-first) |

**IndOS differentiators vs all Niri shells:**
- AI-driven generative UI (nobody else does this)
- Voice pipeline integration
- MCP tool protocol
- TUI fallback via Dioxus (all others are GUI-only)
- Capability-secured IPC via Cap'n Web

---

## Verification Plan

### Per-Milestone
- ISO boots in QEMU and real hardware
- Conversation works offline (Ollama + local model)
- Voice pipeline works with no internet
- Privacy Filter blocks PII in API requests
- No data leaves device in local-only mode
- Agent sandbox prevents unauthorized access

### Release Criteria (M5 Alpha)
- Installs on real hardware via USB
- First-boot completes in <5 minutes
- Voice conversation round-trip <2 seconds
- Survives 24h daily-driver use
- All fragment types render correctly
- Agent routing accuracy >90%
- Memory footprint <2GB idle (excluding model)
- Zero PII leaks to API providers

---

## Resolved Decisions

| Question | Decision |
|----------|----------|
| **Bar** | Waybar for M1–M3 (mature, works today). Evaluate ashell at M4. |
| **Fragment rendering** | Dioxus desktop mode (webview) for M1–M3. Blitz is not production-ready. UX > purity. |
| **Donut Browser AGPL** | Clean — standalone app, not linked. Bundle as-is. |
| **Vision Agents local** | Self-hosted mode works without Stream API keys. Verified. |
| **Cap'n Web** | **Dropped for Rust↔Rust IPC.** Cap'n Web is JS-only. Use JSON over Unix socket for M1, evaluate `tarpc` at M2. Keep Cap'n Web in list for future JS fragment ↔ orchestrator communication. |
| **indos-canvas vs indos-shell** | **indos-canvas archived.** Tauri prototype is superseded by Dioxus shell. One shell. |
| **Monorepo vs multi-repo** | **Cargo workspace monorepo now.** Split to separate repos only when contributors arrive. The 27-repo structure is the future org layout, not the M1 working structure. |

---

## Risk Mitigations

### ✅ R1: Layer-shell support — RESOLVED
**Solution:** Switched from Dioxus to **Iced + `iced_layershell`** (waycrate). Native layer-shell support, proven by ashell and COSMIC DE. No workaround needed.

### ✅ R2: Renderer maturity — RESOLVED
**Solution:** Iced uses wgpu for native rendering (not Blitz). Shell chrome is native iced widgets (fast, GPU-accelerated). Fragments that need HTML/CSS render in an embedded webview widget inside iced.

### ✅ R3: Fragment rendering — RESOLVED
**Solution:** Three-tier architecture:
- **Tier 1 — Native iced widgets:** Pre-built Rust components. <50ms. System monitor, terminal, settings.
- **Tier 2 — Tambo interactable:** Schema-defined components with persistent state. AI reads/updates across turns. Renders in embedded webview.
- **Tier 3 — OpenUI generative:** LLM → HTML/CSS for novel/one-off UI. Ephemeral. Renders in sandboxed webview.

### ✅ R4: Ollama cold start on first boot — RESOLVED
**Problem:** 2GB model download on first boot = bad first impression on slow internet.
**Solution:**
- Ship **Qwen2.5-0.5B** (~400MB GGUF) pre-seeded on the ISO in Ollama's model directory
- Also ship **nomic-embed-text** (~270MB) for vector search — both fit easily on a 4GB ISO
- Pre-seed: copy GGUF files to `/usr/share/ollama/models/` during ISO build, Ollama detects them instantly
- First boot: user chats immediately with the 0.5B model
- Background: `llmfit` scans hardware → recommends optimal model → `ollama pull` runs in background
- Tool: [llmfit](https://github.com/AlexsJones/llmfit) for hardware profiling → auto-selection

### ✅ R5: Memory budget — RESOLVED
**Problem:** Realistic minimum is 4-5GB RAM + 3-4GB VRAM.
**Solution:**
- **Quantization:** Ship all models as GGUF Q4_K_M (4-bit, minimal quality loss). Qwen2.5-0.5B Q4 = ~300MB RAM
- **mmap loading:** Ollama/llama.cpp uses `mmap` by default — OS loads only active model layers into RAM, not the full file
- **Lazy services:** Voice, Vision, Screenpipe are all `systemd` user services with `Type=dbus` — only start when first accessed
- **Tiered requirements:**
  | Tier | RAM | GPU | Experience |
  |------|-----|-----|-----------|
  | Minimal | 8GB | None | 0.5B model, CPU inference, no voice |
  | Recommended | 16GB | 4GB VRAM | 3B model, GPU inference, voice |
  | Full | 32GB | 8GB+ VRAM | 8B+ model, vision, screenpipe |
- Tools: [llama.cpp](https://github.com/ggerganov/llama.cpp) (mmap + quantization), [Ollama](https://github.com/ollama/ollama) (model lifecycle)

### ✅ R6: Voice latency — RESOLVED
**Problem:** LLM generation is the bottleneck (500-2000ms).
**Solution:**
- **Sentence-level streaming TTS:** Piper supports streaming output. Split LLM response at sentence boundaries → start TTS on first sentence while LLM generates the rest
- **Pipeline:** Ollama streaming API → sentence splitter → Piper TTS (per-sentence) → PipeWire output
- **Perceived latency:** First audio starts at ~800ms (Whisper 100ms + LLM first sentence ~500ms + Piper 100ms + buffer 100ms)
- **Actual full response:** 2-4s, but user hears the beginning within 1s — feels instant
- Tools: [Piper](https://github.com/rhasspy/piper) (streaming TTS), [Faster-Whisper](https://github.com/SYSTRAN/faster-whisper) (batched STT)

### ✅ R7: Crash recovery — RESOLVED
**Solution:**
- **[UWSM](https://github.com/nyyManni/uwsm)** (Universal Wayland Session Manager) — wraps Niri into a proper systemd user session with `graphical-session.target`. Manages lifecycle of all shell components
- **[wl-restart](https://github.com/Ferdi265/wl-restart)** — Wayland socket handover tool. If compositor crashes, wl-restart preserves the socket and hands it to the restarted instance. Apps survive compositor restarts
- **systemd services:**
  ```ini
  # indos-shell.service
  [Service]
  Restart=on-failure
  RestartSec=2
  StartLimitIntervalSec=0
  ```
- **Escape hatch:** `Mod+Return → alacritty` (already in Niri config). TTY fallback via `Ctrl+Alt+F2`

### ✅ R8: Notification system — RESOLVED
**Solution:**
- **M1:** [SwayNC](https://github.com/ErikReider/SwayNotificationCenter) — feature-rich notification center with layer-shell support, Do Not Disturb, notification groups, and CSS-themeable. Works on Niri out of the box
- **M2:** Inline conversation notifications — orchestrator posts notification messages directly to the conversation canvas as a fragment. SwayNC handles traditional app notifications (Firefox, etc.)
- **Alternative (Rust):** [Wired](https://github.com/Toqozz/wired-notify) — lightweight Rust notification daemon, highly customizable layouts. Consider for M4 pure-Rust migration
- **Niri config:** Add `spawn-at-startup "swaync"` and layer rules for notification surfaces

### ✅ R9: Update mechanism — RESOLVED
**Solution:**
- **M1-M3:** Standard `pacman` with IndOS custom repo (`indos-repo`). Orchestrator can run `checkupdates` → surface results as a conversation notification → user says "update" → `pacman -Syu` in sandboxed terminal fragment
- **M4:** Evaluate [RAUC](https://github.com/rauc/rauc) for A/B partition updates — available in Arch repos, supports GRUB/EFI, cryptographic verification, D-Bus API for integration with orchestrator
- **M5:** Full OTA with RAUC:
  - A/B root partitions (active + standby)
  - Delta updates via HTTP range requests
  - Automatic rollback on failed boot (GRUB + RAUC watchdog)
  - Orchestrator surfaces update status as ambient fragment
- Tools: [RAUC](https://github.com/rauc/rauc) (A/B OTA), `pacman` (rolling updates), `checkupdates` (safe update checking)

