//! IndOS Shell — The Generative Desktop
//!
//! An Iced layer-shell surface on Niri that replaces the traditional
//! desktop with an AI-driven conversation canvas. Connects to
//! indos-orchestrator via Unix socket for LLM conversation.
//!
//! Architecture:
//! ```text
//! ┌──────────────────────────────────────────────┐
//! │  Niri (Wayland compositor)                    │
//! │  ┌──────────────────────────────────────┐    │
//! │  │  IndOS Shell (iced layer-shell)       │    │
//! │  │  ┌────────────────────────────────┐  │    │
//! │  │  │ Conversation Canvas            │  │    │
//! │  │  │  [system] Welcome to IndOS     │  │    │
//! │  │  │  [user]   Show my files        │  │    │
//! │  │  │  [ai]     Here are your...     │  │    │
//! │  │  │  ┌─────────────────────────┐   │  │    │
//! │  │  │  │ Fragment: File Browser  │   │  │    │
//! │  │  │  └─────────────────────────┘   │  │    │
//! │  │  │  [input]  ________________|    │  │    │
//! │  │  └────────────────────────────────┘  │    │
//! │  └──────────────────────────────────────┘    │
//! └──────────────────────────────────────────────┘
//! ```

mod fragments;

use fragments::registry::{FragmentDescriptor, FragmentRegistry};
use iced::widget::{column, container, row, scrollable, text, text_input, Column};
use iced::{Element, Length, Task, Theme};
use iced::futures::SinkExt;
use iced::window;
use iced_layershell::actions::LayerShellCustomActionWithId;
use iced_layershell::build_pattern::daemon;
use iced_layershell::reexport::{Anchor, KeyboardInteractivity, Layer};
use iced_layershell::settings::{LayerShellSettings, Settings};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

/// IPC messages (must match orchestrator)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum ShellMessage {
    #[serde(rename = "chat")]
    Chat {
        content: String,
        session_id: Option<String>,
    },
    #[serde(rename = "status")]
    Status,
    #[serde(rename = "cancel")]
    Cancel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum OrchestratorResponse {
    #[serde(rename = "chunk")]
    Chunk { content: String },
    #[serde(rename = "done")]
    Done { full_response: String },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "status")]
    StatusResp {
        model: String,
        ollama_connected: bool,
        session_count: usize,
    },
    #[serde(rename = "fragment")]
    Fragment { fragment: serde_json::Value },
}

/// A message in the conversation
#[derive(Debug, Clone)]
struct ConversationMessage {
    role: MessageRole,
    content: String,
}

#[derive(Debug, Clone)]
enum MessageRole {
    System,
    User,
    Assistant,
    /// An inline fragment (rendered as a widget)
    Fragment(FragmentDescriptor),
}

/// App messages
#[derive(Debug, Clone)]
enum Message {
    /// User typed in the input field
    InputChanged(String),

    /// User pressed Enter to send
    Submit,

    /// Received a streaming chunk from orchestrator
    StreamChunk(String),

    /// Received a fragment from orchestrator
    FragmentReceived(FragmentDescriptor),

    /// Response complete
    StreamDone(String),

    /// Connection or LLM error
    OrchestratorError(String),

    /// Connected to orchestrator
    Connected,

    /// Disconnected
    Disconnected,
}

/// The IndOS Shell application state
struct IndOSShell {
    /// User input text
    input: String,

    /// Conversation history
    messages: Vec<ConversationMessage>,

    /// Currently streaming assistant response
    streaming_buffer: String,

    /// Whether we're waiting for a response
    is_generating: bool,

    /// Orchestrator connection status
    connected: bool,

    /// Socket path
    socket_path: PathBuf,

    /// Fragment registry (Tier 1 native components)
    fragment_registry: FragmentRegistry,
}

impl IndOSShell {
    fn new() -> Self {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| format!("/run/user/{}", unsafe { libc::getuid() }));
        let socket_path = PathBuf::from(runtime_dir).join("indos").join("orchestrator.sock");

        Self {
            input: String::new(),
            messages: vec![ConversationMessage {
                role: MessageRole::System,
                content: "Welcome to IndOS. I am your desktop. Ask me anything.".into(),
            }],
            streaming_buffer: String::new(),
            is_generating: false,
            connected: false,
            socket_path,
            fragment_registry: FragmentRegistry::new(),
        }
    }
}

fn update(shell: &mut IndOSShell, message: Message) -> Task<Message> {
        match message {
            Message::InputChanged(value) => {
                shell.input = value;
                Task::none()
            }

            Message::Submit => {
                let content = shell.input.trim().to_string();
                if content.is_empty() || shell.is_generating {
                    return Task::none();
                }

                // Add user message
                shell.messages.push(ConversationMessage {
                    role: MessageRole::User,
                    content: content.clone(),
                });
                shell.input.clear();
                shell.is_generating = true;
                shell.streaming_buffer.clear();

                // Stream from orchestrator
                let socket_path = shell.socket_path.clone();
                Task::run(
                    stream_from_orchestrator(socket_path, content),
                    |msg| msg,
                )
            }

            Message::StreamChunk(chunk) => {
                shell.streaming_buffer.push_str(&chunk);
                Task::none()
            }

            Message::FragmentReceived(descriptor) => {
                shell.messages.push(ConversationMessage {
                    role: MessageRole::Fragment(descriptor),
                    content: String::new(),
                });
                Task::none()
            }

            Message::StreamDone(response) => {
                shell.messages.push(ConversationMessage {
                    role: MessageRole::Assistant,
                    content: if shell.streaming_buffer.is_empty() {
                        response
                    } else {
                        shell.streaming_buffer.clone()
                    },
                });
                shell.streaming_buffer.clear();
                shell.is_generating = false;
                Task::none()
            }

            Message::OrchestratorError(err) => {
                shell.messages.push(ConversationMessage {
                    role: MessageRole::System,
                    content: format!("⚠ {}", err),
                });
                shell.is_generating = false;
                Task::none()
            }

            Message::Connected => {
                shell.connected = true;
                Task::none()
            }

            Message::Disconnected => {
                shell.connected = false;
                Task::none()
            }
        }
    }

fn view(shell: &IndOSShell, _window: window::Id) -> Element<Message> {
        // Build message list
        let messages: Column<Message> = shell
            .messages
            .iter()
            .fold(Column::new().spacing(8), |col, msg| {
                match &msg.role {
                    MessageRole::Fragment(descriptor) => {
                        // Render the fragment through the registry
                        col.push(shell.fragment_registry.render::<Message>(descriptor))
                    }
                    role => {
                        let (prefix, style_color) = match role {
                            MessageRole::System => ("◆ ", iced::Color::from_rgb(0.5, 0.5, 0.6)),
                            MessageRole::User => ("→ ", iced::Color::from_rgb(0.4, 0.8, 1.0)),
                            MessageRole::Assistant => ("◇ ", iced::Color::from_rgb(0.6, 1.0, 0.6)),
                            _ => unreachable!(),
                        };
                        col.push(
                            text(format!("{}{}", prefix, msg.content))
                                .size(16)
                                .color(style_color),
                        )
                    }
                }
            });

        // Add streaming buffer if generating
        let messages = if !shell.streaming_buffer.is_empty() {
            messages.push(
                text(format!("◇ {}▌", shell.streaming_buffer))
                    .size(16)
                    .color(iced::Color::from_rgb(0.6, 1.0, 0.6)),
            )
        } else if shell.is_generating {
            messages.push(
                text("◇ thinking...")
                    .size(16)
                    .color(iced::Color::from_rgb(0.4, 0.4, 0.5)),
            )
        } else {
            messages
        };

        // Scrollable message area
        let conversation = scrollable(messages.width(Length::Fill).padding(16))
            .height(Length::Fill);

        // Input bar
        let input = text_input("Ask IndOS anything...", &shell.input)
            .on_input(Message::InputChanged)
            .on_submit(Message::Submit)
            .padding(12)
            .size(16);

        // Status indicator
        let status = text(if shell.connected {
            "● Connected"
        } else {
            "○ Disconnected"
        })
        .size(12)
        .color(if shell.connected {
            iced::Color::from_rgb(0.3, 0.9, 0.3)
        } else {
            iced::Color::from_rgb(0.9, 0.3, 0.3)
        });

        let header = row![
            text("IndOS").size(18).color(iced::Color::from_rgb(0.6, 0.8, 1.0)),
            iced::widget::Space::new().width(Length::Fill),
            status,
        ]
        .padding(8);

        // Check if we should render ambient mode
        let is_ambient = shell.messages.len() <= 1 && shell.input.is_empty() && !shell.is_generating;

        // Main layout
        let content: Element<Message> = if is_ambient {
            // Ambient view
            let time_text = text(chrono::Local::now().format("%H:%M").to_string())
                .size(72)
                .color(iced::Color::from_rgb(0.9, 0.9, 0.95));
            let date_text = text(chrono::Local::now().format("%A, %B %d").to_string())
                .size(24)
                .color(iced::Color::from_rgb(0.6, 0.6, 0.7));
            let greeting = text("Good to see you. How can I help?")
                .size(18)
                .color(iced::Color::from_rgb(0.5, 0.7, 0.9));

            let ambient_center = column![
                time_text,
                date_text,
                iced::widget::Space::new().height(Length::Fixed(40.0)),
                greeting
            ]
            .align_x(iced::Alignment::Center)
            .spacing(8);

            let ambient_container = container(ambient_center)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill);

            column![header, ambient_container, input].spacing(4).into()
        } else {
            // Chat view
            column![header, conversation, input].spacing(4).into()
        };

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme: &Theme| container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb(
                    0.08, 0.08, 0.1,
                ))),
                ..Default::default()
            })
            .into()
    }

// TryInto impl required by iced_layershell
impl TryInto<LayerShellCustomActionWithId> for Message {
    type Error = Message;
    fn try_into(self) -> Result<LayerShellCustomActionWithId, Self::Error> {
        Err(self)
    }
}

/// Stream messages from the orchestrator, yielding Message variants as chunks arrive.
fn stream_from_orchestrator(
    socket_path: PathBuf,
    content: String,
) -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(64, async move |mut sender| {
        let stream = match UnixStream::connect(&socket_path).await {
            Ok(s) => s,
            Err(e) => {
                let _ = sender
                    .send(Message::OrchestratorError(format!(
                        "Cannot connect to orchestrator: {}. Is it running?",
                        e
                    )))
                    .await;
                return;
            }
        };

        let (reader, mut writer) = stream.into_split();

        // Send chat message
        let msg = ShellMessage::Chat {
            content,
            session_id: None,
        };
        let json = match serde_json::to_string(&msg) {
            Ok(mut j) => {
                j.push('\n');
                j
            }
            Err(e) => {
                let _ = sender.send(Message::OrchestratorError(e.to_string())).await;
                return;
            }
        };
        if let Err(e) = writer.write_all(json.as_bytes()).await {
            let _ = sender.send(Message::OrchestratorError(e.to_string())).await;
            return;
        }
        if let Err(e) = writer.flush().await {
            let _ = sender.send(Message::OrchestratorError(e.to_string())).await;
            return;
        }

        let _ = sender.send(Message::Connected).await;

        // Read responses as they arrive
        let mut reader = BufReader::new(reader);
        let mut line = String::new();
        let mut full_response = String::new();

        loop {
            line.clear();
            let n = match reader.read_line(&mut line).await {
                Ok(n) => n,
                Err(e) => {
                    let _ = sender.send(Message::OrchestratorError(e.to_string())).await;
                    return;
                }
            };
            if n == 0 {
                // Connection closed without Done — finalize with what we have
                let _ = sender.send(Message::StreamDone(full_response)).await;
                return;
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Ok(resp) = serde_json::from_str::<OrchestratorResponse>(trimmed) {
                match resp {
                    OrchestratorResponse::Chunk { content } => {
                        full_response.push_str(&content);
                        let _ = sender.send(Message::StreamChunk(content)).await;
                    }
                    OrchestratorResponse::Done { full_response: fr } => {
                        let final_text = if full_response.is_empty() {
                            fr
                        } else {
                            full_response
                        };
                        let _ = sender.send(Message::StreamDone(final_text)).await;
                        return;
                    }
                    OrchestratorResponse::Error { message } => {
                        let _ = sender.send(Message::OrchestratorError(message)).await;
                        return;
                    }
                    OrchestratorResponse::Fragment { fragment } => {
                        if let Ok(descriptor) =
                            serde_json::from_value::<FragmentDescriptor>(fragment)
                        {
                            let _ =
                                sender.send(Message::FragmentReceived(descriptor)).await;
                        }
                    }
                    _ => {}
                }
            }
        }
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("IndOS Shell v{}", env!("CARGO_PKG_VERSION"));

    let layer_settings = LayerShellSettings {
        size: Some((0, 0)),
        anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
        layer: Layer::Bottom,
        keyboard_interactivity: KeyboardInteractivity::OnDemand,
        exclusive_zone: -1,
        ..Default::default()
    };

    let settings = Settings {
        layer_settings: layer_settings,
        ..Default::default()
    };

    tracing::info!("Starting layer-shell surface...");

    daemon(IndOSShell::new, "indos-shell", update, view)
        .settings(settings)
        .theme(|_state: &IndOSShell, _window: window::Id| Theme::Dark)
        .run()
        .map_err(|e| e.into())
}
