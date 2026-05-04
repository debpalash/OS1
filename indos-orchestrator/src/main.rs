//! IndOS Orchestrator — The Brain
//!
//! Central daemon that:
//! 1. Receives user intent (text, voice transcript, or system event)
//! 2. Classifies intent → determines what kind of task this is
//! 3. Routes to the appropriate agent (System, OpenCode, Claude Code, etc.)
//! 4. Manages sessions (conversation threads that survive reboots)
//! 5. Handles notifications (agent results, system events)
//! 6. Enforces privacy (all API-bound data goes through Privacy Filter)
//! 7. Enforces security (all agent actions go through Security Engine)
//!
//! Runs as a systemd service, communicates via unix socket.
//!
//! ```text
//! [indos-shell] ←→ [unix socket] ←→ [indos-orchestrator]
//!                                         │
//!                    ┌────────────────────┼────────────────────┐
//!                    │                    │                    │
//!              [model-fabric]    [agent-harness]        [mcp-server]
//!              (Ollama/APIs)     (OpenCode/Claude)      (OS tools)
//! ```

mod intent;
mod router;
mod session;

use anyhow::Result;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!("IndOS Orchestrator v{}", env!("CARGO_PKG_VERSION"));

    // Build async runtime
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        // Initialize subsystems
        tracing::info!("Initializing privacy filter...");
        // TODO: Init privacy filter

        tracing::info!("Initializing security engine...");
        // TODO: Init security engine

        tracing::info!("Initializing context engine...");
        // TODO: Init context engine

        tracing::info!("Starting unix socket listener...");
        // TODO: Bind to /run/indos/orchestrator.sock
        // TODO: Accept connections from indos-shell
        // TODO: Enter main event loop

        tracing::info!("Orchestrator ready.");

        // Block forever (systemd will manage lifecycle)
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("Shutting down...");

        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}
