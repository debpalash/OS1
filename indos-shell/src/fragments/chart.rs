//! Chart fragment — renders horizontal bar charts
//!
//! Props:
//!   "title": string
//!   "data": array of { "label": string, "value": number, "max": number }

use iced::widget::{column, container, row, text, Space};
use iced::{Element, Length};

pub fn render<'a, M: 'a + Clone>(props: &serde_json::Value) -> Element<'a, M> {
    let title = props
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Chart");
    let mut col = column![text(title.to_string())
        .size(16)
        .color(iced::Color::from_rgb(0.8, 0.8, 0.9))]
    .spacing(8)
    .padding(12)
    .width(Length::Fill);

    if let Some(data) = props.get("data").and_then(|v| v.as_array()) {
        for item in data {
            let label = item.get("label").and_then(|v| v.as_str()).unwrap_or("?");
            let val = item.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let max = item.get("max").and_then(|v| v.as_f64()).unwrap_or(100.0);

            let percentage = if max > 0.0 {
                (val / max).clamp(0.0, 1.0)
            } else {
                0.0
            };

            let label_col: Element<'a, M> = container(text(label.to_string()).size(14))
                .width(Length::Fixed(80.0))
                .into();

            let val_text: Element<'a, M> = container(text(format!("{:.1}", val)).size(14))
                .width(Length::Fixed(50.0))
                .into();

            let _bar_bg: Element<'a, M> =
                container(Space::new().width(Length::Fill).height(Length::Fixed(12.0)))
                    .width(Length::Fill)
                    .style(|_theme: &iced::Theme| container::Style {
                        background: Some(iced::Background::Color(iced::Color::from_rgb(
                            0.2, 0.2, 0.25,
                        ))),
                        border: iced::Border {
                            radius: 6.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .into();

            // The fill bar
            let fill_width = percentage as f32; // This is a ratio
            let color = if percentage > 0.9 {
                iced::Color::from_rgb(0.9, 0.3, 0.3)
            } else if percentage > 0.7 {
                iced::Color::from_rgb(0.9, 0.7, 0.2)
            } else {
                iced::Color::from_rgb(0.3, 0.8, 0.4)
            };

            let bar_fill: Element<'a, M> = container(
                Space::new()
                    .width(Length::FillPortion((fill_width * 100.0) as u16))
                    .height(Length::Fixed(12.0)),
            )
            .style(move |_theme: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(color)),
                border: iced::Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            })
            .into();

            let empty_fill: Element<'a, M> = container(
                Space::new()
                    .width(Length::FillPortion(((1.0 - fill_width) * 100.0) as u16))
                    .height(Length::Fixed(12.0)),
            )
            .into();

            let progress: Element<'a, M> = container(row![bar_fill, empty_fill])
                .width(Length::Fill)
                .style(|_theme: &iced::Theme| container::Style {
                    background: Some(iced::Background::Color(iced::Color::from_rgb(
                        0.2, 0.2, 0.25,
                    ))),
                    border: iced::Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .into();

            col = col.push(
                row![label_col, progress, val_text]
                    .spacing(12)
                    .align_y(iced::Alignment::Center),
            );
        }
    }

    container(col)
        .width(Length::Fill)
        .style(|_theme: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.12, 0.12, 0.15,
            ))),
            border: iced::Border {
                color: iced::Color::from_rgb(0.3, 0.3, 0.4),
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
}
