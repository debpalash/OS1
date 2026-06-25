//! Terminal fragment — displays command output or embeds a terminal view
//!
//! Props:
//!   "command": string — command that was run
//!   "output": string — the output to display
//!   "exit_code": number (optional) — exit code
//!   "cwd": string (optional) — working directory
//!
//! For M1, this is a read-only display of command output.
//! M2 will embed an actual terminal emulator (alacritty/foot).

use iced::widget::{column, container, text};
use iced::{Element, Length};

pub fn render<'a, M: 'a + Clone>(props: &serde_json::Value) -> Element<'a, M> {
    let command = props
        .get("command")
        .and_then(|v| v.as_str())
        .unwrap_or("(no command)");

    let output = props.get("output").and_then(|v| v.as_str()).unwrap_or("");

    let exit_code = props.get("exit_code").and_then(|v| v.as_i64());

    let cwd = props.get("cwd").and_then(|v| v.as_str()).unwrap_or("~");

    // Header: prompt line
    let prompt = format!("{}$ {}", cwd, command);

    let exit_color = match exit_code {
        Some(0) => iced::Color::from_rgb(0.3, 0.9, 0.3),
        Some(_) => iced::Color::from_rgb(0.9, 0.3, 0.3),
        None => iced::Color::from_rgb(0.5, 0.5, 0.55),
    };

    let mut col = column![text(prompt)
        .size(13)
        .color(iced::Color::from_rgb(0.4, 0.8, 0.4)),]
    .spacing(4)
    .padding(12)
    .width(Length::Fill);

    // Output lines
    if !output.is_empty() {
        col = col.push(
            text(output.to_string())
                .size(13)
                .color(iced::Color::from_rgb(0.75, 0.75, 0.78)),
        );
    }

    // Exit code
    if let Some(code) = exit_code {
        let label = if code == 0 {
            "✓ exit 0".to_string()
        } else {
            format!("✗ exit {}", code)
        };
        col = col.push(text(label).size(11).color(exit_color));
    }

    container(col)
        .width(Length::Fill)
        .style(|_theme: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.04, 0.04, 0.06,
            ))),
            border: iced::Border {
                color: iced::Color::from_rgb(0.2, 0.2, 0.25),
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        })
        .into()
}
