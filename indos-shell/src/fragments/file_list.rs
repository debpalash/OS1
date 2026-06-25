//! File List fragment — renders a directory listing
//!
//! Props:
//!   "path": string — the directory path
//!   "title": string (optional)

use iced::widget::{column, container, row, text};
use iced::{Element, Length};
use std::fs;

pub fn render<'a, M: 'a + Clone>(props: &serde_json::Value) -> Element<'a, M> {
    let path = props.get("path").and_then(|v| v.as_str()).unwrap_or(".");

    let title = props.get("title").and_then(|v| v.as_str()).unwrap_or(path);

    let mut col = column![text(title.to_string())
        .size(16)
        .color(iced::Color::from_rgb(0.6, 0.8, 1.0))]
    .spacing(4)
    .padding(12)
    .width(Length::Fill);

    match fs::read_dir(path) {
        Ok(entries) => {
            let mut list = column![].spacing(2);
            for entry in entries.filter_map(|e| e.ok()).take(20) {
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let prefix = if is_dir { "📁 " } else { "📄 " };
                let color = if is_dir {
                    iced::Color::from_rgb(0.4, 0.7, 1.0)
                } else {
                    iced::Color::from_rgb(0.8, 0.8, 0.8)
                };
                list =
                    list.push(row![text(prefix).color(color), text(name).color(color)].spacing(4));
            }
            col = col.push(list);
        }
        Err(e) => {
            col = col.push(
                text(format!("Failed to read dir: {}", e))
                    .color(iced::Color::from_rgb(1.0, 0.4, 0.4)),
            );
        }
    }

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
