//! Intent classification — determines what kind of task the user wants

use serde::{Deserialize, Serialize};

/// Classified user intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    /// Intent category
    pub category: IntentCategory,

    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,

    /// Original user text
    pub raw_input: String,

    /// Extracted parameters
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntentCategory {
    /// File operations: browse, create, edit, delete, search
    Filesystem,

    /// System management: packages, services, settings, updates
    System,

    /// Coding: write code, debug, refactor, explain
    Coding,

    /// Information: search, explain, summarize
    Information,

    /// Communication: email, messages
    Communication,

    /// Media: play, convert, edit media files
    Media,

    /// Conversation: just chatting, no specific task
    Conversation,

    /// Voice: control voice pipeline
    Voice,

    /// Unknown: couldn't classify
    Unknown,
}

/// Classify user input into an intent
pub async fn classify(_input: &str) -> Intent {
    // TODO: Use local model to classify intent
    // TODO: Extract parameters
    Intent {
        category: IntentCategory::Conversation,
        confidence: 0.5,
        raw_input: _input.to_string(),
        params: serde_json::Value::Null,
    }
}
