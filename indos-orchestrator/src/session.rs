//! Session management — conversation threads that survive reboots

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A conversation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: usize,
    pub active: bool,
}

/// Session manager
pub struct SessionManager {
    sessions: Vec<Session>,
    active_session_id: Option<String>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
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
        };
        self.active_session_id = Some(session.id.clone());
        self.sessions.push(session);
        self.sessions.last().unwrap()
    }

    pub fn active_session(&self) -> Option<&Session> {
        self.active_session_id
            .as_ref()
            .and_then(|id| self.sessions.iter().find(|s| &s.id == id))
    }
}
