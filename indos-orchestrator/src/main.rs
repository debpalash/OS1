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

mod agents;
mod intent;
mod ipc;
mod ollama;
mod router;
mod session;
mod tools;
mod tool_parser;
mod niri;

use anyhow::Result;
use indos_context_engine::{ContextEngine, MemorySource, new_memory};
use indos_privacy::{PrivacyFilter, PrivacyZone};
use indos_security::SecurityEngine;
use ipc::{IpcServer, OrchestratorMessage, SessionInfo, ShellMessage};
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
    let base = r#"You are IndOS, an AI-native operating system assistant. You ARE the desktop.

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
When asked to show something, describe it and generate a fragment if appropriate."#;

    format!("{}{}", base, tools::tools_prompt())
}

/// Shared orchestrator state
struct OrchestratorState {
    ollama: OllamaClient,
    sessions: SessionManager,
    context: ContextEngine,
    privacy: PrivacyFilter,
    security: SecurityEngine,
    /// Conversation history for the active session
    conversation: Vec<ChatMessage>,
    system_prompt: String,
    /// Active session ID for persistence
    session_id: String,
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

    // Initialize session manager and restore previous session
    let mut sessions = SessionManager::new();
    let (session_id, restored_messages) = sessions.load_or_create();
    let conversation = if restored_messages.is_empty() {
        tracing::info!("New session: {}", session_id);
        vec![ChatMessage {
            role: "system".into(),
            content: config.system_prompt.clone(),
        }]
    } else {
        tracing::info!("Restored session: {} ({} messages)", session_id, restored_messages.len());
        restored_messages
    };

    // Initialize state
    let privacy = PrivacyFilter::new();
    tracing::info!("✓ Privacy filter ready (zone: Yellow)");

    let audit_path = data_dir.join("audit.jsonl");
    let mut security = SecurityEngine::new(audit_path.to_str().unwrap_or("./audit.jsonl"));
    match security.init() {
        Ok(()) => tracing::info!("✓ Security engine ready"),
        Err(e) => tracing::warn!("⚠ Security engine init failed: {}", e),
    }

    let state = Arc::new(Mutex::new(OrchestratorState {
        ollama,
        sessions,
        context,
        privacy,
        security,
        conversation,
        system_prompt: config.system_prompt,
        session_id,
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

            // Classify intent and route
            let user_intent = intent::classify(&content);
            let agent_target = router::route(&user_intent);
            tracing::info!(
                "Intent: {:?} (confidence: {:.2}) → {:?}",
                user_intent.category, user_intent.confidence, agent_target
            );

            // Select model based on intent + available models
            let model_name = {
                let state_guard = state.lock().await;
                let available = state_guard.ollama.list_models().await;
                let selection = router::select_model(&user_intent, &available);
                tracing::info!("Model: {} (tier: {:?})", selection.model, selection.tier);
                selection.model
            };

            // Remember user message in context engine
            {
                let state = state.lock().await;
                let memory = new_memory(&content, MemorySource::Conversation);
                if let Err(e) = state.context.remember(memory).await {
                    tracing::debug!("Context remember failed: {}", e);
                }
            }

            // Run privacy filter before sending to LLM
            let filtered_content = {
                let state = state.lock().await;
                match state.privacy.filter(&content, PrivacyZone::Yellow) {
                    Ok(result) => {
                        if result.had_pii {
                            tracing::info!(
                                "Privacy: redacted {} entities before LLM",
                                result.redacted_entities.len()
                            );
                        }
                        result.sanitized_text
                    }
                    Err(e) => {
                        tracing::warn!("Privacy filter blocked message: {}", e);
                        let _ = resp_tx
                            .send(OrchestratorMessage::Error {
                                message: format!("Privacy: {}", e),
                            })
                            .await;
                        return Ok(());
                    }
                }
            };

            // CodingAgent: dispatch to external agent if available
            if let router::AgentTarget::CodingAgent(_) = agent_target {
                if let Some(agent) = agents::detect_agent().await {
                    tracing::info!("Dispatching to coding agent: {:?}", agent);
                    match agents::dispatch(&agent, &filtered_content, &resp_tx).await {
                        Ok(response) => {
                            tracing::info!("Agent response: {} chars", response.len());
                            {
                                let mut state = state.lock().await;
                                state.conversation.push(ChatMessage {
                                    role: "user".into(),
                                    content: filtered_content,
                                });
                                state.conversation.push(ChatMessage {
                                    role: "assistant".into(),
                                    content: response.clone(),
                                });
                            }
                            let _ = resp_tx
                                .send(OrchestratorMessage::Done {
                                    full_response: response,
                                })
                                .await;
                            return Ok(());
                        }
                        Err(e) => {
                            tracing::warn!("Agent dispatch failed, falling back to Ollama: {}", e);
                        }
                    }
                } else {
                    tracing::info!("No coding agent found, falling back to Ollama");
                }
            }

            // Add filtered message to conversation (sent to LLM)
            {
                let mut state = state.lock().await;
                state.conversation.push(ChatMessage {
                    role: "user".into(),
                    content: filtered_content,
                });
            }

            // Get conversation history snapshot
            let messages = {
                let state = state.lock().await;
                state.conversation.clone()
            };

            // Stream response from Ollama with the selected model
            let resp_tx_clone = resp_tx.clone();
            let model_for_stream = model_name.clone();
            let full_response = {
                let state_guard = state.lock().await;
                state_guard
                    .ollama
                    .chat_stream_with_model(&model_for_stream, &messages, |chunk| {
                        let tx = resp_tx_clone.clone();
                        let content = chunk.to_string();
                        let _ = tx.try_send(OrchestratorMessage::Chunk { content });
                    })
                    .await
            };

            match full_response {
                Ok(response) => {
                    tracing::info!("Assistant: {} chars", response.len());

                    // Parse response for tool calls
                    let parsed = tool_parser::parse_tool_calls(&response);

                    if !parsed.tool_calls.is_empty() {
                        tracing::info!("Found {} tool call(s)", parsed.tool_calls.len());

                        // Send the text portion to the shell
                        if !parsed.text.is_empty() {
                            let _ = resp_tx.try_send(OrchestratorMessage::Chunk {
                                content: parsed.text.clone(),
                            });
                        }

                        // Execute tools
                        let tool_output = {
                            let state = state.lock().await;
                            tool_parser::execute_tool_calls(&parsed.tool_calls, &state.security).await
                        };
                        tracing::info!("Tool output: {} chars", tool_output.len());

                        // Feed tool results back into conversation
                        {
                            let mut state = state.lock().await;
                            state.conversation.push(ChatMessage {
                                role: "assistant".into(),
                                content: response.clone(),
                            });
                            state.conversation.push(ChatMessage {
                                role: "user".into(),
                                content: format!("[Tool results]:\n{}", tool_output),
                            });
                        }

                        // Get updated conversation for follow-up
                        let messages = {
                            let state = state.lock().await;
                            state.conversation.clone()
                        };

                        // LLM generates follow-up with tool results
                        let resp_tx_clone2 = resp_tx.clone();
                        let model_for_followup = model_name.clone();
                        let followup = {
                            let state_guard = state.lock().await;
                            state_guard
                                .ollama
                                .chat_stream_with_model(&model_for_followup, &messages, |chunk| {
                                    let tx = resp_tx_clone2.clone();
                                    let content = chunk.to_string();
                                    let _ = tx.try_send(OrchestratorMessage::Chunk { content });
                                })
                                .await
                        };

                        match followup {
                            Ok(followup_response) => {
                                let mut state = state.lock().await;
                                state.conversation.push(ChatMessage {
                                    role: "assistant".into(),
                                    content: followup_response.clone(),
                                });
                                let memory = new_memory(&followup_response, MemorySource::Conversation);
                                if let Err(e) = state.context.remember(memory).await {
                                    tracing::debug!("Context remember failed: {}", e);
                                }
                                let sid = state.session_id.clone();
                                let msgs = state.conversation.clone();
                                if let Err(e) = state.sessions.save_session(&sid, &msgs) {
                                    tracing::warn!("Session save failed: {}", e);
                                }
                                let _ = resp_tx
                                    .send(OrchestratorMessage::Done {
                                        full_response: followup_response,
                                    })
                                    .await;
                            }
                            Err(e) => {
                                // Tool results are still useful even if follow-up fails
                                {
                                    let state = state.lock().await;
                                    let sid = state.session_id.clone();
                                    let msgs = state.conversation.clone();
                                    if let Err(e) = state.sessions.save_session(&sid, &msgs) {
                                        tracing::warn!("Session save failed: {}", e);
                                    }
                                }
                                let _ = resp_tx
                                    .send(OrchestratorMessage::Done {
                                        full_response: format!("{}\n\n{}", parsed.text, tool_output),
                                    })
                                    .await;
                                tracing::warn!("Follow-up generation failed: {}", e);
                            }
                        }
                    } else {
                        // No tool calls — standard response
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
                            let sid = state.session_id.clone();
                            let msgs = state.conversation.clone();
                            if let Err(e) = state.sessions.save_session(&sid, &msgs) {
                                tracing::warn!("Session save failed: {}", e);
                            }
                        }

                        let _ = resp_tx
                            .send(OrchestratorMessage::Done {
                                full_response: response,
                            })
                            .await;
                    }
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

        ShellMessage::ListSessions => {
            let state = state.lock().await;
            let sessions: Vec<SessionInfo> = state
                .sessions
                .list_sessions()
                .into_iter()
                .map(|(id, title, message_count)| SessionInfo {
                    id,
                    title,
                    message_count,
                })
                .collect();
            let count = sessions.len();
            let _ = resp_tx
                .send(OrchestratorMessage::SessionList { sessions })
                .await;
            let _ = resp_tx
                .send(OrchestratorMessage::Done {
                    full_response: format!("{} sessions", count),
                })
                .await;
        }

        ShellMessage::LoadSession { session_id } => {
            let mut state = state.lock().await;
            match state.sessions.load_session(&session_id) {
                Ok(messages) => {
                    let msg_count = messages.len();
                    state.conversation = if messages.is_empty() {
                        vec![ChatMessage {
                            role: "system".into(),
                            content: state.system_prompt.clone(),
                        }]
                    } else {
                        messages
                    };
                    state.session_id = session_id.clone();
                    tracing::info!("Switched to session {} ({} messages)", session_id, msg_count);
                    let _ = resp_tx
                        .send(OrchestratorMessage::Done {
                            full_response: format!("Loaded session {}", session_id),
                        })
                        .await;
                }
                Err(e) => {
                    let _ = resp_tx
                        .send(OrchestratorMessage::Error {
                            message: format!("Failed to load session: {}", e),
                        })
                        .await;
                    let _ = resp_tx
                        .send(OrchestratorMessage::Done {
                            full_response: String::new(),
                        })
                        .await;
                }
            }
        }
    }

    Ok(())
}
