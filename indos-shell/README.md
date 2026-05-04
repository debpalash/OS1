# indos-shell

The generative desktop shell for IndOS. This is the core product — a Wayland-native client that replaces the traditional desktop with an AI-driven conversation canvas.

## What it does

- Renders a full-screen conversation interface as the primary desktop
- Accepts natural language input and routes to the orchestrator
- Renders AI-generated UI fragments (file browsers, terminals, code editors, system monitors)
- Manages three modes: Ambient (passive), Conversation (active), Focus (immersive)

## Architecture

```
indos-shell
├── src/
│   ├── main.rs              # Entry point, Wayland client setup
│   ├── compositor.rs         # Wayland surface management
│   ├── canvas/
│   │   ├── mod.rs            # Conversation canvas layout
│   │   ├── input.rs          # Text input handling
│   │   ├── message.rs        # Message rendering
│   │   └── markdown.rs       # Markdown → rendered output
│   ├── fragments/
│   │   ├── mod.rs            # Fragment lifecycle manager
│   │   ├── renderer.rs       # WebView-based fragment renderer
│   │   └── registry.rs       # Available fragment types
│   ├── modes/
│   │   ├── ambient.rs        # Passive contextual dashboard
│   │   ├── conversation.rs   # Active chat mode
│   │   └── focus.rs          # Immersive workspace mode
│   ├── ipc.rs                # Communication with orchestrator
│   └── config.rs             # Shell configuration
├── Cargo.toml
└── README.md
```

## Building

```bash
cargo build --release
```

## Running (development)

Requires a Wayland compositor (cosmic-comp, hyprland, or sway):
```bash
cargo run
```

## Status

🚧 Scaffolding — not yet functional.
