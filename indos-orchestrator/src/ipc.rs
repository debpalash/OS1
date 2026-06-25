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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_chat_message_tag_and_roundtrip() {
        let msg = ShellMessage::Chat {
            content: "hello".to_string(),
            session_id: Some("abc".to_string()),
        };
        let json = serde_json::to_string(&msg).unwrap();
        // The internally-tagged enum must emit a "type" discriminator the
        // shell relies on for framing.
        assert!(json.contains("\"type\":\"chat\""));
        assert!(json.contains("\"content\":\"hello\""));

        let back: ShellMessage = serde_json::from_str(&json).unwrap();
        match back {
            ShellMessage::Chat {
                content,
                session_id,
            } => {
                assert_eq!(content, "hello");
                assert_eq!(session_id.as_deref(), Some("abc"));
            }
            other => panic!("wrong variant: {:?}", other),
        }
    }

    #[test]
    fn test_shell_chat_message_null_session_id() {
        // session_id is optional; the shell may omit it.
        let parsed: ShellMessage =
            serde_json::from_str(r#"{"type":"chat","content":"hi"}"#).unwrap();
        match parsed {
            ShellMessage::Chat {
                content,
                session_id,
            } => {
                assert_eq!(content, "hi");
                assert!(session_id.is_none());
            }
            other => panic!("wrong variant: {:?}", other),
        }
    }

    #[test]
    fn test_shell_unit_variants_parse() {
        assert!(matches!(
            serde_json::from_str::<ShellMessage>(r#"{"type":"status"}"#).unwrap(),
            ShellMessage::Status
        ));
        assert!(matches!(
            serde_json::from_str::<ShellMessage>(r#"{"type":"cancel"}"#).unwrap(),
            ShellMessage::Cancel
        ));
        assert!(matches!(
            serde_json::from_str::<ShellMessage>(r#"{"type":"list_sessions"}"#).unwrap(),
            ShellMessage::ListSessions
        ));
    }

    #[test]
    fn test_shell_load_session_roundtrip() {
        let msg = ShellMessage::LoadSession {
            session_id: "sess-1".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"load_session\""));
        let back: ShellMessage = serde_json::from_str(&json).unwrap();
        match back {
            ShellMessage::LoadSession { session_id } => assert_eq!(session_id, "sess-1"),
            other => panic!("wrong variant: {:?}", other),
        }
    }

    #[test]
    fn test_orchestrator_status_serialization() {
        let msg = OrchestratorMessage::Status {
            model: "qwen2.5:7b".to_string(),
            ollama_connected: true,
            session_count: 3,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"status\""));
        assert!(json.contains("\"ollama_connected\":true"));
        assert!(json.contains("\"session_count\":3"));
    }

    #[test]
    fn test_orchestrator_chunk_and_done_roundtrip() {
        let chunk = OrchestratorMessage::Chunk {
            content: "tok".to_string(),
        };
        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains("\"type\":\"chunk\""));

        let done = OrchestratorMessage::Done {
            full_response: "all done".to_string(),
        };
        let json = serde_json::to_string(&done).unwrap();
        let back: OrchestratorMessage = serde_json::from_str(&json).unwrap();
        match back {
            OrchestratorMessage::Done { full_response } => assert_eq!(full_response, "all done"),
            other => panic!("wrong variant: {:?}", other),
        }
    }

    #[test]
    fn test_orchestrator_session_list_roundtrip() {
        let msg = OrchestratorMessage::SessionList {
            sessions: vec![SessionInfo {
                id: "id1".to_string(),
                title: Some("My chat".to_string()),
                message_count: 5,
            }],
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"session_list\""));
        let back: OrchestratorMessage = serde_json::from_str(&json).unwrap();
        match back {
            OrchestratorMessage::SessionList { sessions } => {
                assert_eq!(sessions.len(), 1);
                assert_eq!(sessions[0].id, "id1");
                assert_eq!(sessions[0].title.as_deref(), Some("My chat"));
                assert_eq!(sessions[0].message_count, 5);
            }
            other => panic!("wrong variant: {:?}", other),
        }
    }
}
