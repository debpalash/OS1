//! IPC server — Unix socket listener for shell ↔ orchestrator communication
//!
//! Protocol: newline-delimited JSON over Unix domain socket
//! Socket path: $XDG_RUNTIME_DIR/indos/orchestrator.sock
//!
//! Messages:
//!   Shell → Orchestrator: { "type": "chat", "content": "...", "session_id": "..." }
//!   Orchestrator → Shell: { "type": "chunk", "content": "..." }
//!   Orchestrator → Shell: { "type": "done", "full_response": "..." }
//!   Orchestrator → Shell: { "type": "fragment", "fragment": {...} }

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;

/// Message from shell to orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ShellMessage {
    /// User chat message
    #[serde(rename = "chat")]
    Chat {
        content: String,
        session_id: Option<String>,
    },

    /// Request system status
    #[serde(rename = "status")]
    Status,

    /// Cancel current generation
    #[serde(rename = "cancel")]
    Cancel,

    /// List saved sessions
    #[serde(rename = "list_sessions")]
    ListSessions,

    /// Load a specific session by ID
    #[serde(rename = "load_session")]
    LoadSession { session_id: String },
}

/// Message from orchestrator to shell
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum OrchestratorMessage {
    /// Streaming token chunk
    #[serde(rename = "chunk")]
    Chunk { content: String },

    /// Generation complete
    #[serde(rename = "done")]
    Done { full_response: String },

    /// Error occurred
    #[serde(rename = "error")]
    Error { message: String },

    /// System status response
    #[serde(rename = "status")]
    Status {
        model: String,
        ollama_connected: bool,
        session_count: usize,
    },

    /// A2UI fragment to render
    #[serde(rename = "fragment")]
    Fragment { fragment: serde_json::Value },

    /// List of saved sessions
    #[serde(rename = "session_list")]
    SessionList { sessions: Vec<SessionInfo> },
}

/// Session metadata sent to the shell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub title: Option<String>,
    pub message_count: usize,
}

/// Get the socket path
pub fn socket_path() -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/run/user/{}", unsafe { libc::getuid() }));
    let dir = PathBuf::from(runtime_dir).join("indos");
    dir.join("orchestrator.sock")
}

/// IPC server that accepts connections from the shell
pub struct IpcServer {
    socket_path: PathBuf,
}

impl IpcServer {
    pub fn new() -> Self {
        Self {
            socket_path: socket_path(),
        }
    }

    /// Start listening for connections
    pub async fn listen(
        &self,
        handler: mpsc::Sender<(ShellMessage, mpsc::Sender<OrchestratorMessage>)>,
    ) -> Result<()> {
        // Create parent directory
        if let Some(parent) = self.socket_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Remove stale socket file
        if self.socket_path.exists() {
            tokio::fs::remove_file(&self.socket_path).await?;
        }

        let listener = UnixListener::bind(&self.socket_path)?;
        tracing::info!("IPC listening on {}", self.socket_path.display());

        // Set permissions so the user's shell can connect
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&self.socket_path, std::fs::Permissions::from_mode(0o700))?;
        }

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    tracing::info!("Shell connected");
                    let handler = handler.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, handler).await {
                            tracing::error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Accept error: {}", e);
                }
            }
        }
    }
}

/// Handle a single shell connection
async fn handle_connection(
    stream: UnixStream,
    handler: mpsc::Sender<(ShellMessage, mpsc::Sender<OrchestratorMessage>)>,
) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            tracing::info!("Shell disconnected");
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Parse incoming message
        let msg: ShellMessage = match serde_json::from_str(trimmed) {
            Ok(m) => m,
            Err(e) => {
                let err = OrchestratorMessage::Error {
                    message: format!("Invalid message: {}", e),
                };
                let mut resp = serde_json::to_string(&err)?;
                resp.push('\n');
                writer.write_all(resp.as_bytes()).await?;
                continue;
            }
        };

        // Create response channel
        let (resp_tx, mut resp_rx) = mpsc::channel::<OrchestratorMessage>(64);

        // Send to handler
        handler.send((msg, resp_tx)).await?;

        // Stream responses back to the shell
        while let Some(response) = resp_rx.recv().await {
            let is_done = matches!(response, OrchestratorMessage::Done { .. });
            let mut json = serde_json::to_string(&response)?;
            json.push('\n');
            writer.write_all(json.as_bytes()).await?;
            writer.flush().await?;
            if is_done {
                break;
            }
        }
    }

    Ok(())
}

/// IPC client (used by indos-shell to connect to orchestrator)
// part of the public API surface consumed by indos-shell, not wired up in-workspace
#[allow(dead_code)]
pub struct IpcClient {
    socket_path: PathBuf,
}

#[allow(dead_code)]
impl IpcClient {
    pub fn new() -> Self {
        Self {
            socket_path: socket_path(),
        }
    }

    pub fn from_path(path: &Path) -> Self {
        Self {
            socket_path: path.to_path_buf(),
        }
    }

    /// Send a message and receive streaming responses
    pub async fn send(&self, msg: &ShellMessage) -> Result<mpsc::Receiver<OrchestratorMessage>> {
        let stream = UnixStream::connect(&self.socket_path).await?;
        let (reader, mut writer) = stream.into_split();

        // Send the message
        let mut json = serde_json::to_string(msg)?;
        json.push('\n');
        writer.write_all(json.as_bytes()).await?;
        writer.flush().await?;

        // Spawn reader task
        let (tx, rx) = mpsc::channel(64);
        tokio::spawn(async move {
            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break,
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.is_empty() {
                            continue;
                        }
                        if let Ok(msg) = serde_json::from_str::<OrchestratorMessage>(trimmed) {
                            let is_done = matches!(msg, OrchestratorMessage::Done { .. });
                            let _ = tx.send(msg).await;
                            if is_done {
                                break;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(rx)
    }
}
