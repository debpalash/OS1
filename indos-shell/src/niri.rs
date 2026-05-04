//! Niri compositor integration via IPC
//!
//! IndOS shell communicates with Niri compositor via its unix socket IPC.
//! Two communication patterns:
//!
//! 1. **Commands** — shell tells Niri what to do (focus window, move, resize)
//! 2. **Event stream** — Niri tells shell what happened (window opened, focus changed)
//!
//! The event stream is critical for context awareness:
//! "User switched to Firefox" → orchestrator adjusts context
//! "New terminal opened" → orchestrator offers to help
//!
//! ```text
//! ┌─────────────┐     unix socket     ┌──────────────┐
//! │ IndOS Shell  │ ←───────────────── │   Niri        │
//! │              │   event stream      │  compositor   │
//! │              │ ──────────────────→ │              │
//! │              │   niri msg commands │              │
//! └─────────────┘                     └──────────────┘
//! ```

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Events received from Niri's event stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NiriEvent {
    /// A new window was opened
    WindowOpened {
        id: u64,
        app_id: String,
        title: String,
    },

    /// A window was closed
    WindowClosed { id: u64 },

    /// Window focus changed
    WindowFocused { id: Option<u64> },

    /// Window title changed (e.g., terminal prompt, browser tab)
    WindowTitleChanged { id: u64, title: String },

    /// Active workspace changed
    WorkspaceChanged {
        monitor: String,
        workspace_id: u64,
    },

    /// Keyboard layout changed
    KeyboardLayoutChanged { name: String },
}

/// Connection to Niri compositor
pub struct NiriConnection {
    /// Path to Niri's IPC socket
    socket_path: String,

    /// Whether we're connected
    connected: bool,
}

impl NiriConnection {
    pub fn new() -> Self {
        // Niri socket is at $XDG_RUNTIME_DIR/niri/$NIRI_SOCKET or similar
        let runtime_dir =
            std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/1000".into());
        let socket_path = format!("{}/niri-socket", runtime_dir);

        Self {
            socket_path,
            connected: false,
        }
    }

    /// Connect to Niri's IPC socket
    pub async fn connect(&mut self) -> Result<()> {
        tracing::info!("Connecting to Niri IPC at {}", self.socket_path);
        // TODO: Connect to unix socket
        // TODO: Start event stream listener in background task
        self.connected = true;
        Ok(())
    }

    /// Send a command to Niri (via `niri msg` protocol)
    pub async fn send_command(&self, command: NiriCommand) -> Result<()> {
        tracing::debug!("Niri command: {:?}", command);
        // TODO: Serialize command and send via socket
        Ok(())
    }

    /// Subscribe to Niri's event stream
    /// Returns a channel that receives compositor events in real-time
    pub async fn subscribe_events(
        &self,
    ) -> Result<tokio::sync::mpsc::Receiver<NiriEvent>> {
        let (tx, rx) = tokio::sync::mpsc::channel(256);

        // TODO: Connect to Niri event stream socket
        // TODO: Spawn task that deserializes events and sends to channel
        // TODO: Handle reconnection on disconnect

        let _tx = tx; // suppress unused warning until implemented

        Ok(rx)
    }

    /// Get list of all open windows
    pub async fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        // TODO: Send "windows" request to Niri
        Ok(vec![])
    }

    /// Get the currently focused window
    pub async fn focused_window(&self) -> Result<Option<WindowInfo>> {
        // TODO: Send "focused-window" request to Niri
        Ok(None)
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

/// Commands that IndOS can send to Niri
#[derive(Debug, Clone)]
pub enum NiriCommand {
    /// Focus a specific window by ID
    FocusWindow { id: u64 },

    /// Move focus in a direction
    FocusDirection(Direction),

    /// Spawn a new window (run a command)
    Spawn { command: Vec<String> },

    /// Close a window
    CloseWindow { id: u64 },

    /// Switch to a workspace
    SwitchWorkspace { index: u64 },

    /// Move a window to a workspace
    MoveWindowToWorkspace { id: u64, workspace: u64 },

    /// Toggle the IndOS shell overlay
    ToggleShellOverlay,
}

#[derive(Debug, Clone)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// Information about an open window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: u64,
    pub app_id: String,
    pub title: String,
    pub is_focused: bool,
    pub workspace_id: u64,
}
