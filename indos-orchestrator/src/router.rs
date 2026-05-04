//! Agent router — dispatches intents to the right agent AND model
//!
//! Model routing strategy (inspired by Manifest):
//! - Simple chat/Q&A → small model (qwen2.5:0.5b) — fast, cheap
//! - Complex reasoning/coding → large model (qwen2.5:7b, codellama) — accurate
//! - System commands → no model needed, direct tool execution
//! - Vision tasks → vision model (qwen2.5-vl)

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

/// Select the best model for a given intent
pub fn select_model(intent: &Intent, available_models: &[String]) -> ModelSelection {
    let preferred = match intent.category {
        // Simple tasks — use smallest model
        IntentCategory::Conversation | IntentCategory::Voice => ModelTier::Small,

        // Information retrieval — medium model
        IntentCategory::Information => ModelTier::Medium,

        // Coding — needs reasoning
        IntentCategory::Coding => ModelTier::Code,

        // System — may need tool use understanding
        IntentCategory::Filesystem | IntentCategory::System => ModelTier::Medium,

        // Everything else — small is fine
        _ => ModelTier::Small,
    };

    // Find best available model for the tier
    let model = match preferred {
        ModelTier::Small => find_model(available_models, &[
            "qwen2.5:0.5b", "qwen2.5:1.5b", "phi3:mini", "gemma2:2b",
        ]),
        ModelTier::Medium => find_model(available_models, &[
            "qwen2.5:7b", "llama3.1:8b", "mistral:7b", "gemma2:9b",
        ]),
        ModelTier::Large => find_model(available_models, &[
            "qwen2.5:32b", "llama3.1:70b", "mixtral:8x7b",
        ]),
        ModelTier::Code => find_model(available_models, &[
            "qwen2.5-coder:7b", "codellama:7b", "deepseek-coder:6.7b", "qwen2.5:7b",
        ]),
        ModelTier::Vision => find_model(available_models, &[
            "qwen2.5-vl:7b", "llava:7b", "bakllava:7b",
        ]),
    };

    ModelSelection {
        model: model.unwrap_or_else(|| "qwen2.5:0.5b".to_string()),
        tier: preferred,
    }
}

/// Find the first available model from preferences
fn find_model(available: &[String], preferences: &[&str]) -> Option<String> {
    for pref in preferences {
        if available.iter().any(|m| m == pref) {
            return Some(pref.to_string());
        }
    }
    // Fallback: return first available model
    available.first().cloned()
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

#[derive(Debug, Clone)]
pub enum ModelTier {
    Small,   // <2B params — chat, simple Q&A
    Medium,  // 7-9B params — reasoning, info
    Large,   // 32B+ params — complex analysis
    Code,    // Code-specialized models
    Vision,  // Vision-language models
}

#[derive(Debug, Clone)]
pub struct ModelSelection {
    pub model: String,
    pub tier: ModelTier,
}
