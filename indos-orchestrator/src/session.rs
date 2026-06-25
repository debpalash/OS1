//! Session management — conversation threads that survive reboots
//!
//! Sessions are persisted as JSON files in `$XDG_DATA_HOME/indos/sessions/`.
//! Each session stores its conversation history so it can be restored.

use crate::ollama::ChatMessage;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A conversation session (serializable)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: usize,
    pub active: bool,
    pub messages: Vec<ChatMessage>,
}

/// Session manager — handles create, save, load, list
pub struct SessionManager {
    sessions_dir: PathBuf,
    sessions: Vec<Session>,
    active_session_id: Option<String>,
}

impl SessionManager {
    pub fn new() -> Self {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("indos")
            .join("sessions");

        // Ensure dir exists
        let _ = std::fs::create_dir_all(&data_dir);

        Self {
            sessions_dir: data_dir,
            sessions: Vec::new(),
            active_session_id: None,
        }
    }

    /// Create a new session
    pub fn create_session(&mut self) -> &Session {
        let session = Session {
            id: uuid::Uuid::new_v4().to_string(),
            title: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            message_count: 0,
            active: true,
            messages: Vec::new(),
        };
        self.active_session_id = Some(session.id.clone());
        self.sessions.push(session);
        self.sessions.last().unwrap()
    }

    /// Save a session's conversation to disk
    pub fn save_session(&self, session_id: &str, messages: &[ChatMessage]) -> anyhow::Result<()> {
        if let Some(session) = self.sessions.iter().find(|s| s.id == session_id) {
            let mut session = session.clone();
            session.messages = messages.to_vec();
            session.message_count = messages.len();
            session.updated_at = Utc::now();

            // Auto-title from first user message
            if session.title.is_none() {
                session.title = messages.iter().find(|m| m.role == "user").map(|m| {
                    let title: String = m.content.chars().take(60).collect();
                    if m.content.len() > 60 {
                        format!("{}...", title)
                    } else {
                        title
                    }
                });
            }

            let path = self.sessions_dir.join(format!("{}.json", session_id));
            let json = serde_json::to_string_pretty(&session)?;
            std::fs::write(&path, json)?;
            tracing::debug!(
                "Session saved: {} ({} messages)",
                session_id,
                messages.len()
            );
        }
        Ok(())
    }

    /// Load a session from disk
    pub fn load_session(&mut self, session_id: &str) -> anyhow::Result<Vec<ChatMessage>> {
        let path = self.sessions_dir.join(format!("{}.json", session_id));
        let json = std::fs::read_to_string(&path)?;
        let session: Session = serde_json::from_str(&json)?;
        let messages = session.messages.clone();

        // Update in-memory
        self.active_session_id = Some(session.id.clone());
        if !self.sessions.iter().any(|s| s.id == session.id) {
            self.sessions.push(session);
        }

        tracing::info!(
            "Session loaded: {} ({} messages)",
            session_id,
            messages.len()
        );
        Ok(messages)
    }

    /// Load the most recent session (or create a new one)
    pub fn load_or_create(&mut self) -> (String, Vec<ChatMessage>) {
        // Find most recent session file
        if let Ok(entries) = std::fs::read_dir(&self.sessions_dir) {
            let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
            for entry in entries.flatten() {
                if entry.path().extension().is_some_and(|e| e == "json") {
                    if let Ok(meta) = entry.metadata() {
                        if let Ok(modified) = meta.modified() {
                            if newest.as_ref().is_none_or(|(_, t)| modified > *t) {
                                newest = Some((entry.path(), modified));
                            }
                        }
                    }
                }
            }

            if let Some((path, _)) = newest {
                if let Some(id) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(messages) = self.load_session(id) {
                        return (id.to_string(), messages);
                    }
                }
            }
        }

        // No existing session — create new
        let session = self.create_session();
        (session.id.clone(), Vec::new())
    }

    /// List all saved sessions
    pub fn list_sessions(&self) -> Vec<(String, Option<String>, usize)> {
        let mut sessions = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.sessions_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().is_some_and(|e| e == "json") {
                    if let Ok(json) = std::fs::read_to_string(entry.path()) {
                        if let Ok(session) = serde_json::from_str::<Session>(&json) {
                            sessions.push((session.id, session.title, session.message_count));
                        }
                    }
                }
            }
        }
        sessions
    }

    // session accessors, part of the manager's public API surface, not yet wired up
    #[allow(dead_code)]
    pub fn active_session(&self) -> Option<&Session> {
        self.active_session_id
            .as_ref()
            .and_then(|id| self.sessions.iter().find(|s| &s.id == id))
    }

    #[allow(dead_code)]
    pub fn active_session_id(&self) -> Option<&str> {
        self.active_session_id.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_serde_roundtrip() {
        // Sessions are persisted as JSON and restored on reboot, so the
        // round-trip must preserve all fields including the message history.
        let now = Utc::now();
        let session = Session {
            id: "sess-123".to_string(),
            title: Some("Greeting".to_string()),
            created_at: now,
            updated_at: now,
            message_count: 2,
            active: true,
            messages: vec![
                ChatMessage {
                    role: "user".to_string(),
                    content: "hi".to_string(),
                },
                ChatMessage {
                    role: "assistant".to_string(),
                    content: "hello".to_string(),
                },
            ],
        };

        let json = serde_json::to_string_pretty(&session).unwrap();
        let back: Session = serde_json::from_str(&json).unwrap();

        assert_eq!(back.id, "sess-123");
        assert_eq!(back.title.as_deref(), Some("Greeting"));
        assert_eq!(back.message_count, 2);
        assert!(back.active);
        assert_eq!(back.messages.len(), 2);
        assert_eq!(back.messages[0].role, "user");
        assert_eq!(back.messages[1].content, "hello");
    }

    #[test]
    fn test_session_with_no_title_serializes() {
        let now = Utc::now();
        let session = Session {
            id: "sess-empty".to_string(),
            title: None,
            created_at: now,
            updated_at: now,
            message_count: 0,
            active: false,
            messages: Vec::new(),
        };
        let json = serde_json::to_string(&session).unwrap();
        let back: Session = serde_json::from_str(&json).unwrap();
        assert!(back.title.is_none());
        assert!(back.messages.is_empty());
        assert!(!back.active);
    }
}
