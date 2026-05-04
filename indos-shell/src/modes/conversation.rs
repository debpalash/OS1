//! Conversation mode — active AI interaction

pub struct ConversationMode {
    /// Whether the conversation is currently streaming a response
    pub streaming: bool,
}

impl ConversationMode {
    pub fn new() -> Self {
        Self { streaming: false }
    }
}
