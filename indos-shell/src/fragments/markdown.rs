//! Markdown fragment — renders rich text using Iced's rich_text
//!
//! Props:
//!   "content": string

use iced::widget::{column, container, text};
use iced::{Element, Length};

pub fn render<'a, M: 'a + Clone>(props: &serde_json::Value) -> Element<'a, M> {
    let content = props.get("content").and_then(|v| v.as_str()).unwrap_or("");

    // A simple basic text fallback since fully parsing markdown in Iced
    // requires walking the AST and mapping to rich_text chunks.
    // For this fragment, we just display it as text for now until a full
    // markdown parser is integrated.

    let col = column![text(content.to_string())
        .size(14)
        .color(iced::Color::from_rgb(0.9, 0.9, 0.9))]
    .padding(12)
    .width(Length::Fill);

    container(col)
        .width(Length::Fill)
        .style(|_theme: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.1, 0.1, 0.12,
            ))),
            border: iced::Border {
                color: iced::Color::from_rgb(0.3, 0.3, 0.35),
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        })
        .into()
}
