//! Text input handling for the conversation canvas

pub struct InputHandler {
    buffer: String,
    cursor_pos: usize,
    history: Vec<String>,
    history_index: Option<usize>,
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            cursor_pos: 0,
            history: Vec::new(),
            history_index: None,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.buffer.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    pub fn submit(&mut self) -> String {
        let text = self.buffer.clone();
        if !text.is_empty() {
            self.history.push(text.clone());
        }
        self.buffer.clear();
        self.cursor_pos = 0;
        self.history_index = None;
        text
    }

    pub fn current_text(&self) -> &str {
        &self.buffer
    }
}
