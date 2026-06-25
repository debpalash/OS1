# OS 1 — top-level developer task runner.
#
# Unifies the two build worlds:
#   * the Cargo workspace (backend services)         -> cargo --workspace
#   * the standalone indos-shell GUI crate           -> cargo (in indos-shell/)
#   * the ISO build + QEMU toolkit                    -> ./indos-dev
#
# Run `make` or `make help` for the list.

.DEFAULT_GOAL := help

# indos-shell is a separate workspace root (see root Cargo.toml), so it needs
# its own cargo invocations on top of the `--workspace` ones.
SHELL_CRATE := indos-shell

.PHONY: help build build-shell test lint fmt fmt-check clippy audit iso clean

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-12s\033[0m %s\n", $$1, $$2}'

build: ## Build all Rust crates (workspace + indos-shell)
	cargo build --workspace
	cd $(SHELL_CRATE) && cargo build

build-shell: ## Build just the indos-shell GUI crate
	cd $(SHELL_CRATE) && cargo build

test: ## Run all workspace tests
	cargo test --workspace

lint: fmt-check clippy ## Run all linters (fmt check + clippy)

fmt: ## Auto-format all Rust crates
	cargo fmt --all
	cd $(SHELL_CRATE) && cargo fmt --all

fmt-check: ## Check formatting (CI gate)
	cargo fmt --all --check
	cd $(SHELL_CRATE) && cargo fmt --all --check

clippy: ## Run clippy with warnings denied (CI gate)
	cargo clippy --workspace --all-targets -- -D warnings

audit: ## Check dependencies for known security advisories
	cargo audit

iso: ## Build the bootable ISO (delegates to ./indos-dev)
	./indos-dev build

clean: ## Remove Cargo build artifacts
	cargo clean
	cd $(SHELL_CRATE) && cargo clean
