//! Agent router — dispatches intents to the right agent

use crate::intent::{Intent, IntentCategory};

/// Determines which agent should handle an intent
pub fn route(intent: &Intent) -> AgentTarget {
    match intent.category {
        IntentCategory::Filesystem => AgentTarget::System,
        IntentCategory::System => AgentTarget::System,
        IntentCategory::Coding => AgentTarget::coding_default(),
        IntentCategory::Information => AgentTarget::ModelDirect,
        IntentCategory::Communication => AgentTarget::System,
        IntentCategory::Media => AgentTarget::System,
        IntentCategory::Conversation => AgentTarget::ModelDirect,
        IntentCategory::Voice => AgentTarget::System,
        IntentCategory::Unknown => AgentTarget::ModelDirect,
    }
}

#[derive(Debug, Clone)]
pub enum AgentTarget {
    /// Handle directly with LLM (conversation, info queries)
    ModelDirect,

    /// Built-in system agent (files, packages, services)
    System,

    /// External coding agent
    CodingAgent(String),
}

impl AgentTarget {
    /// Default coding agent (OpenCode, fallback to model-direct)
    pub fn coding_default() -> Self {
        AgentTarget::CodingAgent("opencode".into())
    }
}
