//! Shell modes — different interaction paradigms

pub mod ambient;
pub mod conversation;
pub mod focus;

/// The three modes of IndOS shell
#[derive(Debug, Clone, PartialEq)]
pub enum ShellMode {
    /// Passive contextual dashboard. Shows relevant info without asking.
    /// Triggered: idle, or explicit switch.
    Ambient,

    /// Active conversation. User is talking to IndOS.
    /// Triggered: user types/speaks, or incoming notification.
    Conversation,

    /// Immersive workspace. Fragments fill the screen. Minimal chrome.
    /// Triggered: user enters deep work, or explicit switch.
    Focus,
}
