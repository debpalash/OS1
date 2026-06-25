# Contributing to OS 1

Thank you for your interest in contributing to OS 1 — India's sovereign, AI-first operating system built on Arch Linux and open-source generative AI.

## Project Structure

OS 1 is a **monorepo**. Everything lives in this single repository:

- **OS / ISO** (`indos-iso/`, `indos-settings/`, `assets/`) — archiso profile, Calamares installer config, niri/waybar/swaync desktop settings. Follows Arch Linux packaging conventions.
- **AI backend services** — a Cargo **workspace** at the repo root:
  - `indos-orchestrator` — intent classification, agent routing, Ollama conversation
  - `indos-voice` — local STT/TTS voice pipeline (`indos-voiced`)
  - `indos-context-engine` — LanceDB vector memory + embeddings
  - `indos-privacy` — local PII redaction
  - `indos-security` — agent sandboxing + capability model
- **GUI frontends** (standalone crates, *not* in the workspace):
  - `indos-shell` — iced/Wayland generative desktop shell
  - `indos-canvas` — Tauri app
- **Tooling** — `indos-dev` (ISO build + QEMU toolkit), `Makefile` (Rust task runner), `autotest/` (automated install/boot tests).

## Development Setup

### Prerequisites
- Arch Linux or CachyOS (recommended for development; required to build the ISO)
- Rust toolchain via rustup — the pinned channel is in `rust-toolchain.toml`
- Node.js 20+ (see `.nvmrc`) for `indos-canvas`
- Ollama (for local model testing)
- For ISO work: `archiso`, `qemu`, and root access

### Building the Rust code

The repo root is a Cargo workspace, so you can build/test everything from there:

```bash
make build      # build all crates (workspace + indos-shell)
make test       # run all workspace tests
make lint       # cargo fmt --check + clippy -D warnings
make fmt        # auto-format
```

Or use cargo directly:

```bash
cargo build --workspace            # backend services
cargo build -p indos-orchestrator  # a single crate
cd indos-shell && cargo build      # the shell (separate workspace root)
```

> Note: `indos-shell` and `indos-canvas` are intentionally excluded from the
> root workspace (their GUI dependency trees conflict with the backend's, e.g.
> `bytemuck` between iced and lancedb), so they keep their own `Cargo.lock`.

### Building & testing the ISO

Use the `indos-dev` toolkit (run `./indos-dev help` for all commands):

```bash
./indos-dev build          # incremental ISO build
./indos-dev test           # boot the latest ISO in QEMU
./indos-dev calamares      # validate the installer config
```

## Code Style

- **Rust:** `cargo fmt` + `cargo clippy` (config in `rustfmt.toml`; CI denies warnings)
- **TypeScript/JS/CSS:** Prettier (`.prettierrc`)
- **Shell scripts:** ShellCheck (`.shellcheckrc`)
- **Editors:** settings in `.editorconfig`
- **Commit messages:** Conventional Commits (`feat:`, `fix:`, `chore:`, etc.)

CI runs `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, and
ShellCheck on every pull request (`.github/workflows/ci.yml`). Run `make lint`
locally before pushing.

## Submitting Changes

1. Create a feature branch (`feat/description`)
2. Make your changes with clear, conventional commit messages
3. Run `make lint && make test` (and `./indos-dev` checks for ISO changes)
4. Open a Pull Request against `main`

## Architecture Decisions

Major architectural changes should be discussed in an issue before implementation.
See [VISION.md](VISION.md) for the project's design philosophy.
