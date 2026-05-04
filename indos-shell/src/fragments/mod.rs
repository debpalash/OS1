//! Fragment lifecycle management
//!
//! Manages the registry of available A2UI components and their lifecycle.
//! Two types:
//! - **Generative** — ephemeral, created on demand, destroyed after use
//! - **Interactable** — persistent, AI can read/update across conversation turns

pub mod registry;
pub mod renderer;

/// Fragment lifecycle states
#[derive(Debug, Clone, PartialEq)]
pub enum FragmentState {
    /// Being created by the LLM
    Generating,
    /// Rendered and visible
    Active,
    /// User is interacting with it
    Focused,
    /// Minimized but still alive (interactable only)
    Background,
    /// Being destroyed
    Closing,
    /// Gone
    Destroyed,
}

/// A live fragment instance
pub struct FragmentInstance {
    pub id: String,
    pub component: String,
    pub state: FragmentState,
    pub interactive: bool,
    pub props: serde_json::Value,
}

impl FragmentInstance {
    pub fn new(id: String, component: String, interactive: bool) -> Self {
        Self {
            id,
            component,
            state: FragmentState::Generating,
            interactive,
            props: serde_json::Value::Null,
        }
    }
}
