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

use iced::widget::{column, container, row, scrollable, text, text_input, Column};
use iced::{Element, Length, Task, Theme};
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

                // Send to orchestrator
                let socket_path = shell.socket_path.clone();
                Task::perform(
                    send_to_orchestrator(socket_path, content),
                    |result| match result {
                        Ok(response) => Message::StreamDone(response),
                        Err(e) => Message::OrchestratorError(e.to_string()),
                    },
                )
            }

            Message::StreamChunk(chunk) => {
                shell.streaming_buffer.push_str(&chunk);
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
                let (prefix, style_color) = match msg.role {
                    MessageRole::System => ("◆ ", iced::Color::from_rgb(0.5, 0.5, 0.6)),
                    MessageRole::User => ("→ ", iced::Color::from_rgb(0.4, 0.8, 1.0)),
                    MessageRole::Assistant => ("◇ ", iced::Color::from_rgb(0.6, 1.0, 0.6)),
                };
                col.push(
                    text(format!("{}{}", prefix, msg.content))
                        .size(16)
                        .color(style_color),
                )
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

        // Main layout
        let content = column![header, conversation, input].spacing(4);

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

/// Send a message to the orchestrator and get the full response
async fn send_to_orchestrator(socket_path: PathBuf, content: String) -> Result<String, String> {
    let stream = UnixStream::connect(&socket_path)
        .await
        .map_err(|e| format!("Cannot connect to orchestrator: {}. Is it running?", e))?;

    let (reader, mut writer) = stream.into_split();

    // Send chat message
    let msg = ShellMessage::Chat {
        content,
        session_id: None,
    };
    let mut json = serde_json::to_string(&msg).map_err(|e| e.to_string())?;
    json.push('\n');
    writer
        .write_all(json.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    writer.flush().await.map_err(|e| e.to_string())?;

    // Read responses
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    let mut full_response = String::new();

    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .await
            .map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Ok(resp) = serde_json::from_str::<OrchestratorResponse>(trimmed) {
            match resp {
                OrchestratorResponse::Chunk { content } => {
                    full_response.push_str(&content);
                }
                OrchestratorResponse::Done { full_response: fr } => {
                    if full_response.is_empty() {
                        full_response = fr;
                    }
                    break;
                }
                OrchestratorResponse::Error { message } => {
                    return Err(message);
                }
                _ => {}
            }
        }
    }

    Ok(full_response)
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
