# Niri Ecosystem — Complete GitHub Repository Research

> 30 repositories catalogued from 3 pages of GitHub search results.
> Research conducted for IndOS project — compositor layer selection.

---

## Tier 1: Core Project (IndOS MUST track)

### [niri-wm/niri](https://github.com/niri-wm/niri) ⭐ 23.7k
**THE compositor.** Scrollable tiling Wayland compositor built on Smithay.
- **Language:** Rust
- **License:** GPL-3.0
- **Status:** Very active (updated 2 days ago)
- **Key features:** Infinite scrolling canvas, per-monitor workspaces, IPC via JSON socket, window rules, animations, screenshot support, layer-shell, gamescope, screen recording
- **IndOS role:** L1 — the foundational compositor layer. IndOS shell runs as a layer-shell client on top.

### [niri-wm/awesome-niri](https://github.com/niri-wm/awesome-niri) ⭐ 1.1k
**Official curated list** of Niri-compatible tools, configs, and resources.
- **Language:** Nix
- **Status:** Active (updated 6 days ago)
- **IndOS role:** Reference catalog. Must monitor for new ecosystem tools.

---

## Tier 2: Essential Ecosystem (IndOS should integrate or fork)

### [sodiboo/niri-flake](https://github.com/sodiboo/niri-flake) ⭐ 844
**NixOS flake** for Niri — packaging, configuration modules, home-manager integration.
- **Language:** Nix
- **Status:** Very active (updated yesterday)
- **IndOS relevance:** Reference for packaging patterns. Our `indos-pkgbuilds` should study how this structures the Niri config module system.

### [MalpenZibo/ashell](https://github.com/MalpenZibo/ashell) ⭐ 904
**Rust status bar** built with iced-rs, designed specifically for wlroots/Niri.
- **Language:** Rust (iced-rs)
- **Status:** Very active (updated 5 hours ago)
- **IndOS relevance:** **STRONG CANDIDATE** as Waybar alternative. Pure Rust, iced-based, Niri-native. Could be forked for our AI status bar instead of Waybar (which is C++/GTK). Shares the Smithay ecosystem DNA.

### [LawnGnome/niri-taskbar](https://github.com/LawnGnome/niri-taskbar) ⭐ 139
**Taskbar/dock** for Niri — shows running windows as clickable icons.
- **Language:** Rust
- **Status:** Last updated Dec 2025
- **IndOS relevance:** Could be integrated as a fragment or adapted for our Waybar module.

### [Vigintillionn/niri-sidebar](https://github.com/Vigintillionn/niri-sidebar) ⭐ 137
**Sidebar widget** for Niri — a vertical panel.
- **Language:** Rust
- **Status:** Updated Feb 2026
- **IndOS relevance:** Reference architecture for building layer-shell sidebars. IndOS shell will be similar — a layer-shell surface that slides in from the side.

### [gvolpe/niri-scratchpad](https://github.com/gvolpe/niri-scratchpad) ⭐ 121
**Scratchpad implementation** for Niri (drop-down terminal, quick-access windows).
- **Language:** Python
- **Status:** Updated Mar 2026
- **IndOS relevance:** Our shell's `--toggle` overlay is conceptually similar. Reference for how to implement show/hide with Niri IPC.

### [isaksamsten/niriswitcher](https://github.com/isaksamsten/niriswitcher) ⭐ 148 (archived)
**Window switcher** (Alt-Tab equivalent) for Niri.
- **Language:** Python
- **Status:** Archived Nov 2025
- **IndOS relevance:** Shows what was missing in Niri's defaults. The window switching UX is something IndOS should handle natively.

### [imiric/qml-niri](https://github.com/imiric/qml-niri) ⭐ 73
**QML bindings for Niri IPC** — enables Qt/QML widgets to communicate with Niri.
- **Language:** C++17 / Qt
- **Status:** Active (updated yesterday)
- **IndOS relevance:** Reference for Niri IPC protocol from non-Rust languages. Shows how to subscribe to events, send commands.

---

## Tier 3: Shell / Ricing Projects (Design Inspiration)

### [noctalia-dev/noctalia-shell](https://github.com/noctalia-dev/noctalia-shell) ⭐ 6.3k
**Full desktop shell** for Niri AND Hyprland using QuickShell (QML).
- **Language:** QML
- **Status:** Extremely active (updated 2 hours ago)
- **Key features:** Animated panels, app launcher, notifications, calendar, media controls, AI-themed aesthetics
- **IndOS relevance:** **TOP DESIGN REFERENCE.** This is the closest existing project to what IndOS is building — a complete shell layer on Niri. Study their QML layouts, animation timing, and compositor integration patterns. CachyOS ships a variant of this (see below).

### [AvengeMedia/DankMaterialShell](https://github.com/AvengeMedia/DankMaterialShell) ⭐ 6.1k
**Material Design shell** for Sway/Niri using QuickShell.
- **Language:** QML
- **Status:** Extremely active (updated 21 minutes ago)
- **IndOS relevance:** Design inspiration for Material-style UI patterns on Wayland tiling compositors.

### [snowarch/iNiR](https://github.com/snowarch/iNiR) ⭐ 947
**Ricing dotfiles** for Niri using QuickShell.
- **Language:** QML
- **Status:** Active (updated 3 days ago)
- **IndOS relevance:** Visual inspiration. Shows what a polished Niri desktop can look like.

### [XansiVA/nirimation](https://github.com/XansiVA/nirimation) ⭐ 184
**Animation presets** and ricing config for Niri.
- **Language:** Config files
- **Status:** Updated Feb 2026
- **IndOS relevance:** Copy animation curves and timing values for our `config.kdl`.

### [debuggyo/Exo](https://github.com/debuggyo/Exo) ⭐ 625
**Niri rice** with Python-based widgets and unixporn aesthetics.
- **Language:** Python
- **Status:** Updated Jan 2026
- **IndOS relevance:** Widget design patterns.

### [AyushKr2003/niri-caelestia-shell](https://github.com/AyushKr2003/niri-caelestia-shell) ⭐ 143
**Caelestia shell port** for Niri using QuickShell.
- **Language:** QML
- **Status:** Updated 11 days ago
- **IndOS relevance:** Another full shell implementation to study.

### [jgarza9788/niri-animation-collection](https://github.com/jgarza9788/niri-animation-collection) ⭐ 133
**Curated animation presets** for Niri config.
- **Language:** Shell
- **Status:** Updated 27 days ago
- **IndOS relevance:** Direct resource — cherry-pick animation configs for IndOS defaults.

---

## Tier 4: Configuration & Settings Tools

### [srinivasr/nirimod](https://github.com/srinivasr/nirimod) ⭐ 171
**GUI configurator** for Niri settings.
- **Language:** Python
- **Status:** Very active (updated 10 hours ago)
- **IndOS relevance:** Could be bundled as a "Niri settings" fragment. Or we build our own via conversation: "make my windows bigger" → orchestrator modifies niri config.

### [stefonarch/niri-settings](https://github.com/stefonarch/niri-settings) ⭐ 130
**Settings app** for Niri — GUI for editing config.kdl.
- **Language:** Python
- **Status:** Updated 14 days ago
- **IndOS relevance:** Same as nirimod — reference for which settings users commonly want to change.

### [CachyOS/cachyos-niri-settings](https://github.com/CachyOS/cachyos-niri-settings) ⭐ 68
**CachyOS's official Niri configuration package.**
- **Language:** Shell
- **Status:** Updated Dec 2025
- **IndOS relevance:** **CRITICAL REFERENCE.** Since IndOS is CachyOS-based, this is the upstream config we're effectively replacing with `indos-settings/niri/config.kdl`. Study for packaging patterns and default choices.

### [CachyOS/cachyos-niri-noctalia](https://github.com/CachyOS/cachyos-niri-noctalia) ⭐ 39
**CachyOS's Niri + Noctalia shell** integration package.
- **Language:** Config
- **Status:** Updated 29 days ago
- **IndOS relevance:** Shows how CachyOS integrates a third-party shell with Niri. IndOS will follow a similar pattern but with our own shell.

---

## Tier 5: Distribution / OS Projects Using Niri

### [zirconium-dev/zirconium](https://github.com/zirconium-dev/zirconium) ⭐ 271
**Atomic Linux desktop** using bootc + Niri — immutable Fedora-based OS with Niri as compositor.
- **Language:** Shell
- **Status:** Active (updated 2 days ago)
- **IndOS relevance:** **COMPETITOR / SISTER PROJECT.** Another OS project using Niri as the compositor! Key differences from IndOS: Zirconium is Fedora-based + bootc (immutable), we are Arch-based. They are a traditional desktop, we are AI-first. Study their bootc integration, packaging, and Niri session setup.

### [Sinomor/delta-shell](https://github.com/Sinomor/delta-shell) ⭐ 191
**Desktop shell** in TypeScript targeting Niri.
- **Language:** TypeScript
- **Status:** Updated 18 days ago
- **IndOS relevance:** Shows a web-tech approach to Niri shells. Validates our decision to use Dioxus (which can render HTML/CSS natively via Blitz) rather than pure Electron/webview.

---

## Tier 6: Dotfiles & Personal Configs (Community Reference)

### [saatvik333/niri-dotfiles](https://github.com/saatvik333/niri-dotfiles) ⭐ 149
Installer-based Niri dotfiles with Waybar, rofi, etc.

### [acaibowlz/niri-setup](https://github.com/acaibowlz/niri-setup) ⭐ 132
Clean Niri rice with terminal aesthetics.

### [shub39/dotfiles](https://github.com/shub39/dotfiles) ⭐ 401
Arch + Niri + QuickShell dotfiles.

### [EbadShelby/dotfiles](https://github.com/EbadShelby/dotfiles) ⭐ 151
Niri dotfiles with CSS-heavy styling.

### [dileep-kishore/nixos-config](https://github.com/dileep-kishore/nixos-config) ⭐ 76
NixOS + Niri + home-manager config.

### [Naxdy/niri](https://github.com/Naxdy/niri) ⭐ 53
Niri fork with custom patches.

### [linuxmobile/astal-bar](https://github.com/linuxmobile/astal-bar) ⭐ 31
Astal-based status bar for Niri, written in Lua.

---

## Key Takeaways for IndOS

### 1. Ecosystem Health ✅
Niri has a **thriving ecosystem** — 23.7k stars, 100+ pages of repos, active community building shells, bars, config tools, dotfiles. This validates the compositor choice.

### 2. Shell Layer Pattern
The dominant pattern is **QuickShell (QML)** for building desktop shells on Niri (Noctalia, DankMaterialShell, iNiR, Caelestia). IndOS is diverging from this by using **Dioxus (Rust)** for TUI+GUI hybrid capability — a unique differentiator.

### 3. Bars
Two strong options:
- **Waybar** (C++, GTK) — universal, mature, what most people use
- **ashell** (Rust, iced-rs) — Niri-native, same Rust ecosystem

> **Decision point:** Consider switching from Waybar to ashell for a pure-Rust stack, or keep Waybar for broader compatibility. Can support both.

### 4. CachyOS Integration
CachyOS already has `cachyos-niri-settings` and `cachyos-niri-noctalia`. IndOS can use these as upstream references and replace them with `indos-settings` packages.

### 5. Competition
**Zirconium** is the only other "OS project using Niri" found, but it's Fedora+bootc (immutable/atomic), not AI-first. IndOS is unique in combining Niri with a generative AI shell layer.

### 6. Missing Pieces (Opportunities)
The ecosystem lacks:
- **AI-aware shells** — nobody is doing LLM-driven UI generation on Niri (IndOS is first)
- **Voice integration** — no Niri + voice pipeline projects exist
- **MCP integration** — no Niri compositor with Model Context Protocol support
- **TUI fallback** — all shells are GUI-only (IndOS's Dioxus TUI mode is unique)
