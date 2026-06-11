# Contributing to IndOS

Thank you for your interest in contributing to IndOS — India's sovereign, AI-first operating system built on Arch Linux and open-source generative AI.

## Project Structure

IndOS is organized as a multi-repo project. Each repository has a specific responsibility:

- **Foundation repos** (kernel, iso, pkgbuilds, settings) — Follow Arch Linux packaging conventions
- **AI layer repos** (shell, orchestrator, model-fabric, mcp-server, agent-harness) — Rust crates
- **Desktop repos** (fragments, themes, welcome) — TypeScript + CSS

## Development Setup

### Prerequisites
- Arch Linux or CachyOS (recommended for development)
- Rust toolchain (stable, via rustup)
- Node.js 20+ (for fragment development)
- Ollama (for local model testing)

### Building
Each Rust repo can be built independently:
```bash
cd indos-shell
cargo build
```

### Testing
```bash
cargo test
```

### ISO Testing
```bash
cd indos-iso
sudo mkarchiso -v .
# Test in QEMU:
run_archiso -i out/indos-*.iso
```

## Code Style

- Rust: `cargo fmt` + `cargo clippy`
- TypeScript: Prettier
- Shell scripts: ShellCheck
- Commit messages: Conventional Commits (`feat:`, `fix:`, `chore:`, etc.)

## Submitting Changes

1. Fork the relevant repository
2. Create a feature branch (`feat/description`)
3. Make your changes with clear commit messages
4. Run tests and linting
5. Submit a Pull Request

## Architecture Decisions

Major architectural changes should be discussed in an issue before implementation.
Reference the [VISION.md](VISION.md) for the project's design philosophy.
