//! Markdown rendering for conversation messages
//! Converts markdown text to renderable elements

pub struct MarkdownRenderer;

impl MarkdownRenderer {
    pub fn new() -> Self {
        Self
    }

    /// Parse markdown text into renderable blocks
    pub fn parse(&self, _text: &str) -> Vec<MarkdownBlock> {
        // TODO: Use pulldown-cmark to parse markdown
        // TODO: Convert to renderable blocks
        vec![]
    }
}

#[derive(Debug, Clone)]
pub enum MarkdownBlock {
    Paragraph(String),
    Heading { level: u8, text: String },
    CodeBlock { language: String, code: String },
    List(Vec<String>),
    Quote(String),
    HorizontalRule,
}
