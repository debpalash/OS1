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
//!              [Ollama LLM]      [agent-harness]        [mcp-server]
//!              (local models)    (OpenCode/Claude)      (OS tools)
//! ```

mod intent;
mod ipc;
mod ollama;
mod router;
mod session;

use anyhow::Result;
use indos_context_engine::{ContextEngine, MemorySource, new_memory};
use ipc::{IpcServer, OrchestratorMessage, ShellMessage};
use ollama::{ChatMessage, OllamaClient};
use session::SessionManager;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing_subscriber::EnvFilter;

/// Orchestrator configuration
struct Config {
    ollama_url: String,
    ollama_model: String,
    embedding_model: String,
    system_prompt: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ollama_url: "http://localhost:11434".into(),
            ollama_model: "qwen2.5:0.5b".into(),
            embedding_model: "nomic-embed-text".into(),
            system_prompt: indos_system_prompt(),
        }
    }
}

/// The IndOS system prompt — defines how the OS talks
fn indos_system_prompt() -> String {
    r#"You are IndOS, an AI-native operating system assistant. You ARE the desktop.

Your role:
- You help the user interact with their Linux system through conversation
- You can browse files, manage packages, control services, and run commands
- You generate UI fragments (file browsers, system monitors, editors) when appropriate
- You are concise, helpful, and proactive
- You run on Niri (Wayland compositor) with an Iced-based shell

Personality:
- Direct and efficient — you're an OS, not a chatbot
- You anticipate what the user needs based on context
- You explain what you're doing before executing system commands
- You ask for confirmation before destructive operations

When asked to do something on the system, describe what you'll do, then do it.
When asked to show something, describe it and generate a fragment if appropriate."#
        .to_string()
}

/// Shared orchestrator state
struct OrchestratorState {
    ollama: OllamaClient,
    sessions: SessionManager,
    context: ContextEngine,
    /// Conversation history for the active session
    conversation: Vec<ChatMessage>,
    system_prompt: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    tracing::info!("IndOS Orchestrator v{}", env!("CARGO_PKG_VERSION"));

    let config = Config::default();

    // Initialize Ollama client
    let ollama = OllamaClient::new(&config.ollama_url, &config.ollama_model);
    match ollama.health_check().await {
        Ok(true) => tracing::info!("✓ Ollama connected, model '{}' ready", config.ollama_model),
        Ok(false) => tracing::warn!(
            "⚠ Ollama reachable but model '{}' not found — pull it with: ollama pull {}",
            config.ollama_model,
            config.ollama_model
        ),
        Err(e) => tracing::warn!(
            "⚠ Ollama not reachable ({}). Start with: systemctl start ollama",
            e
        ),
    }

    // Initialize context engine (LanceDB)
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("indos");
    let db_path = data_dir.join("context.lance");
    let mut context = ContextEngine::new(
        db_path.to_str().unwrap_or("./context.lance"),
        &config.embedding_model,
    );
    match context.init().await {
        Ok(()) => {
            let count = context.count().await.unwrap_or(0);
            tracing::info!("✓ Context engine ready ({} memories)", count);
        }
        Err(e) => tracing::warn!("⚠ Context engine failed to init: {}", e),
    }

    // Initialize state
    let state = Arc::new(Mutex::new(OrchestratorState {
        ollama,
        sessions: SessionManager::new(),
        context,
        conversation: vec![ChatMessage {
            role: "system".into(),
            content: config.system_prompt.clone(),
        }],
        system_prompt: config.system_prompt,
    }));

    // Create message handler channel
    let (handler_tx, mut handler_rx) =
        mpsc::channel::<(ShellMessage, mpsc::Sender<OrchestratorMessage>)>(32);

    // Spawn the message handler
    let state_clone = state.clone();
    tokio::spawn(async move {
        while let Some((msg, resp_tx)) = handler_rx.recv().await {
            let state = state_clone.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_message(msg, resp_tx, state).await {
                    tracing::error!("Handler error: {}", e);
                }
            });
        }
    });

    // Start IPC server
    let server = IpcServer::new();
    tracing::info!("Orchestrator ready. Waiting for shell connection...");
    server.listen(handler_tx).await?;

    Ok(())
}

/// Handle a single message from the shell
async fn handle_message(
    msg: ShellMessage,
    resp_tx: mpsc::Sender<OrchestratorMessage>,
    state: Arc<Mutex<OrchestratorState>>,
) -> Result<()> {
    match msg {
        ShellMessage::Chat {
            content,
            session_id: _,
        } => {
            tracing::info!("User: {}", content);

            // Remember user message in context engine
            {
                let state = state.lock().await;
                let memory = new_memory(&content, MemorySource::Conversation);
                if let Err(e) = state.context.remember(memory).await {
                    tracing::debug!("Context remember failed: {}", e);
                }
            }

            // Add user message to conversation
            {
                let mut state = state.lock().await;
                state.conversation.push(ChatMessage {
                    role: "user".into(),
                    content: content.clone(),
                });
            }

            // Get conversation history snapshot
            let messages = {
                let state = state.lock().await;
                state.conversation.clone()
            };

            // Stream response from Ollama
            let resp_tx_clone = resp_tx.clone();
            let full_response = {
                let state_guard = state.lock().await;
                state_guard
                    .ollama
                    .chat_stream(&messages, |chunk| {
                        let tx = resp_tx_clone.clone();
                        let content = chunk.to_string();
                        // Fire-and-forget the chunk send
                        let _ = tx.try_send(OrchestratorMessage::Chunk { content });
                    })
                    .await
            };

            match full_response {
                Ok(response) => {
                    tracing::info!("Assistant: {} chars", response.len());

                    // Add assistant message to conversation history + context
                    {
                        let mut state = state.lock().await;
                        state.conversation.push(ChatMessage {
                            role: "assistant".into(),
                            content: response.clone(),
                        });
                        let memory = new_memory(&response, MemorySource::Conversation);
                        if let Err(e) = state.context.remember(memory).await {
                            tracing::debug!("Context remember failed: {}", e);
                        }
                    }

                    let _ = resp_tx
                        .send(OrchestratorMessage::Done {
                            full_response: response,
                        })
                        .await;
                }
                Err(e) => {
                    tracing::error!("Ollama error: {}", e);
                    let _ = resp_tx
                        .send(OrchestratorMessage::Error {
                            message: format!("LLM error: {}", e),
                        })
                        .await;
                }
            }
        }

        ShellMessage::Status => {
            let state = state.lock().await;
            let connected = state.ollama.health_check().await.unwrap_or(false);
            let _ = resp_tx
                .send(OrchestratorMessage::Status {
                    model: state.ollama.model_name().to_string(),
                    ollama_connected: connected,
                    session_count: 1,
                })
                .await;
            // Send done so the shell knows the response is complete
            let _ = resp_tx
                .send(OrchestratorMessage::Done {
                    full_response: "status".into(),
                })
                .await;
        }

        ShellMessage::Cancel => {
            tracing::info!("Generation cancelled");
            let _ = resp_tx
                .send(OrchestratorMessage::Done {
                    full_response: "[cancelled]".into(),
                })
                .await;
        }
    }

    Ok(())
}
