//! Message types for the conversation canvas

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: Role,
    pub content: MessageContent,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    /// Plain text or markdown
    Text(String),

    /// A2UI fragment specification
    Fragment(FragmentSpec),

    /// Streaming text (in progress)
    Streaming { text: String, complete: bool },
}

/// A2UI-compatible fragment specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FragmentSpec {
    /// Component name from the registry (e.g., "file-browser", "terminal")
    pub component: String,

    /// Props to pass to the component
    pub props: serde_json::Value,

    /// Whether this fragment is interactable (persistent) or generative (ephemeral)
    pub interactive: bool,

    /// Fragment dimensions (optional, shell will auto-layout)
    pub width: Option<u32>,
    pub height: Option<u32>,
}
