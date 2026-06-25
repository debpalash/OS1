//! Code View fragment — renders source code
//!
//! Props:
//!   "code": string
//!   "language": string (optional)

use iced::widget::{column, container, scrollable, text};
use iced::{Element, Font, Length};

pub fn render<'a, M: 'a + Clone>(props: &serde_json::Value) -> Element<'a, M> {
    let code = props.get("code").and_then(|v| v.as_str()).unwrap_or("");
    let lang = props
        .get("language")
        .and_then(|v| v.as_str())
        .unwrap_or("text");

    let header = text(lang.to_string())
        .size(12)
        .color(iced::Color::from_rgb(0.5, 0.5, 0.6));

    let code_text = text(code.to_string())
        .font(Font::MONOSPACE)
        .size(13)
        .color(iced::Color::from_rgb(0.8, 0.9, 0.8));

    let col = column![header, scrollable(code_text).width(Length::Fill)]
        .spacing(8)
        .padding(12)
        .width(Length::Fill);

    container(col)
        .width(Length::Fill)
        .style(|_theme: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.08, 0.08, 0.1,
            ))),
            border: iced::Border {
                color: iced::Color::from_rgb(0.2, 0.3, 0.4),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into()
}
