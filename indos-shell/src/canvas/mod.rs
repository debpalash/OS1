//! Conversation canvas — message rendering and input handling

pub mod input;
pub mod markdown;
pub mod message;

/// The conversation canvas manages the primary UI surface
/// where messages are displayed and user input is captured.
pub struct ConversationCanvas {
    messages: Vec<message::Message>,
    input_buffer: String,
    scroll_offset: f32,
}

impl ConversationCanvas {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            input_buffer: String::new(),
            scroll_offset: 0.0,
        }
    }

    pub fn add_message(&mut self, msg: message::Message) {
        self.messages.push(msg);
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
}
