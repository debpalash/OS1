//! Text Block fragment — renders styled text content
//!
//! Props:
//!   "content": string — the text to display
//!   "title": string (optional) — header above the text
//!   "style": "code" | "quote" | "info" | "warning" | "error" (optional)

use iced::widget::{column, container, text};
use iced::{Element, Length};

pub fn render<'a, M: 'a + Clone>(props: &serde_json::Value) -> Element<'a, M> {
    let content = props
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("(empty)");

    let title = props.get("title").and_then(|v| v.as_str());
    let style = props
        .get("style")
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    let (bg_color, text_color, border_color) = match style {
        "code" => (
            iced::Color::from_rgb(0.1, 0.1, 0.12),
            iced::Color::from_rgb(0.8, 0.9, 0.8),
            iced::Color::from_rgb(0.3, 0.3, 0.35),
        ),
        "quote" => (
            iced::Color::from_rgb(0.08, 0.1, 0.14),
            iced::Color::from_rgb(0.6, 0.7, 0.9),
            iced::Color::from_rgb(0.3, 0.4, 0.7),
        ),
        "warning" => (
            iced::Color::from_rgb(0.15, 0.12, 0.05),
            iced::Color::from_rgb(1.0, 0.85, 0.3),
            iced::Color::from_rgb(0.7, 0.6, 0.2),
        ),
        "error" => (
            iced::Color::from_rgb(0.15, 0.05, 0.05),
            iced::Color::from_rgb(1.0, 0.4, 0.4),
            iced::Color::from_rgb(0.7, 0.2, 0.2),
        ),
        _ => (
            // info
            iced::Color::from_rgb(0.1, 0.1, 0.14),
            iced::Color::from_rgb(0.75, 0.8, 0.85),
            iced::Color::from_rgb(0.2, 0.3, 0.4),
        ),
    };

    let mut col = column![].spacing(4).padding(12).width(Length::Fill);

    if let Some(t) = title {
        col = col.push(
            text(t.to_string())
                .size(15)
                .color(iced::Color::from_rgb(0.9, 0.9, 0.95)),
        );
    }

    col = col.push(text(content.to_string()).size(14).color(text_color));

    container(col)
        .width(Length::Fill)
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(bg_color)),
            border: iced::Border {
                color: border_color,
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        })
        .into()
}
