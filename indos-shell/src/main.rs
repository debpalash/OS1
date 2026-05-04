//! IndOS Shell — The Generative Desktop
//!
//! A Wayland-native shell running on Niri compositor that replaces the
//! traditional desktop paradigm with an AI-driven conversation canvas
//! that generates UI fragments on the fly using A2UI specification.
//!
//! Architecture:
//! ```text
//! ┌──────────────────────────────────────────────────┐
//! │  Niri (Wayland compositor, scrollable tiling)     │
//! │                                                   │
//! │  ┌──────────────────────────────────────────┐    │
//! │  │  IndOS Shell (layer-shell surface)        │    │
//! │  │                                           │    │
//! │  │  ┌─────────────┐  ┌──────────────────┐   │    │
//! │  │  │ Conversation │  │ Fragment          │   │    │
//! │  │  │ Canvas       │  │ Viewports (A2UI)  │   │    │
//! │  │  └─────────────┘  └──────────────────┘   │    │
//! │  └──────────────────────────────────────────┘    │
//! │                                                   │
//! │  ┌──────────┐  ┌──────────┐  ┌──────────────┐   │
//! │  │ Firefox   │  │ VS Code  │  │ Traditional   │   │
//! │  │ (scrolls) │  │ (scrolls)│  │ apps (Niri)   │   │
//! │  └──────────┘  └──────────┘  └──────────────┘   │
//! └──────────────────────────────────────────────────┘
//! ```

mod canvas;
mod config;
mod fragments;
mod ipc;
mod modes;
mod niri;

use anyhow::Result;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!("IndOS Shell v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Initializing generative desktop on Niri...");

    // Load configuration
    let config = config::ShellConfig::load()?;
    tracing::info!("Config loaded: {:?}", config.mode);

    // Initialize fragment registry
    let registry = fragments::registry::FragmentRegistry::new();
    tracing::info!(
        "Fragment registry: {} components available",
        registry.list().len()
    );

    // Build async runtime
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        // Step 1: Connect to Niri compositor via IPC
        let mut niri_conn = niri::NiriConnection::new();
        if let Err(e) = niri_conn.connect().await {
            tracing::warn!("Niri IPC not available (running outside Niri?): {}", e);
        } else {
            tracing::info!("Connected to Niri compositor");

            // Subscribe to compositor events for context awareness
            match niri_conn.subscribe_events().await {
                Ok(_events) => {
                    tracing::info!("Subscribed to Niri event stream");
                    // TODO: Spawn task to process events
                    // - WindowFocused → update orchestrator context
                    // - WindowOpened → offer contextual help
                    // - WindowTitleChanged → track what user is doing
                }
                Err(e) => tracing::warn!("Could not subscribe to Niri events: {}", e),
            }
        }

        // Step 2: Connect to IndOS orchestrator
        let mut orchestrator = ipc::OrchestratorConnection::new(
            config.orchestrator_socket.to_str().unwrap_or("/run/indos/orchestrator.sock"),
        );
        if let Err(e) = orchestrator.connect().await {
            tracing::warn!("Orchestrator not available: {}", e);
            tracing::info!("Running in standalone mode (no AI backend)");
        }

        // Step 3: Initialize conversation canvas
        let canvas = canvas::ConversationCanvas::new();
        tracing::info!("Conversation canvas ready ({} messages)", canvas.message_count());

        // Step 4: Initialize fragment renderer
        let renderer = fragments::renderer::FragmentRenderer::new();
        tracing::info!("Fragment renderer ready");

        // Step 5: Determine initial mode
        let mode = &config.mode;
        tracing::info!("Initial mode: {:?}", mode);

        tracing::info!("IndOS Shell ready. Entering main loop.");

        // TODO: Create layer-shell surface via wayland-client
        // TODO: Render conversation canvas using iced/wgpu
        // TODO: Enter Wayland event loop

        // Placeholder — block until terminated
        println!();
        println!("  ╔══════════════════════════════════════════════╗");
        println!("  ║         IndOS — Generative Desktop           ║");
        println!("  ║                                              ║");
        println!("  ║  Compositor:  Niri (scrollable tiling)       ║");
        println!("  ║  Fragments:   {} components registered       ║", registry.list().len());
        println!("  ║  Mode:        {:?}                    ║", mode);
        println!("  ║  Orchestrator: {}           ║",
            if orchestrator.is_connected() { "connected    " } else { "disconnected " });
        println!("  ║  Niri IPC:    {}           ║",
            if niri_conn.is_connected() { "connected    " } else { "disconnected " });
        println!("  ║                                              ║");
        println!("  ║  Wayland rendering: pending                  ║");
        println!("  ║  Press Ctrl+C to exit.                       ║");
        println!("  ╚══════════════════════════════════════════════╝");
        println!();

        tokio::signal::ctrl_c().await.ok();
        tracing::info!("Shutting down...");

        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}
