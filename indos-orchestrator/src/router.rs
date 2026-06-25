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
        ModelTier::Small => find_model(
            available_models,
            &["qwen2.5:0.5b", "qwen2.5:1.5b", "phi3:mini", "gemma2:2b"],
        ),
        ModelTier::Medium => find_model(
            available_models,
            &["qwen2.5:7b", "llama3.1:8b", "mistral:7b", "gemma2:9b"],
        ),
        ModelTier::Large => find_model(
            available_models,
            &["qwen2.5:32b", "llama3.1:70b", "mixtral:8x7b"],
        ),
        ModelTier::Code => find_model(
            available_models,
            &[
                "qwen2.5-coder:7b",
                "codellama:7b",
                "deepseek-coder:6.7b",
                "qwen2.5:7b",
            ],
        ),
        ModelTier::Vision => find_model(
            available_models,
            &["qwen2.5-vl:7b", "llava:7b", "bakllava:7b"],
        ),
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
    // agent name is part of the domain model, not yet read
    #[allow(dead_code)]
    CodingAgent(String),
}

impl AgentTarget {
    /// Default coding agent (OpenCode, fallback to model-direct)
    pub fn coding_default() -> Self {
        AgentTarget::CodingAgent("opencode".into())
    }
}

// model tiers form the routing domain model; not all are constructed yet
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum ModelTier {
    Small,  // <2B params — chat, simple Q&A
    Medium, // 7-9B params — reasoning, info
    Large,  // 32B+ params — complex analysis
    Code,   // Code-specialized models
    Vision, // Vision-language models
}

#[derive(Debug, Clone)]
pub struct ModelSelection {
    pub model: String,
    pub tier: ModelTier,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn intent(category: IntentCategory) -> Intent {
        Intent {
            category,
            confidence: 1.0,
            raw_input: String::new(),
            params: serde_json::Value::Null,
        }
    }

    #[test]
    fn test_route_filesystem_and_system_go_to_system() {
        assert!(matches!(
            route(&intent(IntentCategory::Filesystem)),
            AgentTarget::System
        ));
        assert!(matches!(
            route(&intent(IntentCategory::System)),
            AgentTarget::System
        ));
    }

    #[test]
    fn test_route_conversation_and_unknown_go_model_direct() {
        assert!(matches!(
            route(&intent(IntentCategory::Conversation)),
            AgentTarget::ModelDirect
        ));
        assert!(matches!(
            route(&intent(IntentCategory::Unknown)),
            AgentTarget::ModelDirect
        ));
        assert!(matches!(
            route(&intent(IntentCategory::Information)),
            AgentTarget::ModelDirect
        ));
    }

    #[test]
    fn test_route_coding_uses_named_coding_agent() {
        match route(&intent(IntentCategory::Coding)) {
            AgentTarget::CodingAgent(name) => assert_eq!(name, "opencode"),
            other => panic!("expected coding agent, got {:?}", other),
        }
    }

    #[test]
    fn test_coding_default_is_opencode() {
        match AgentTarget::coding_default() {
            AgentTarget::CodingAgent(name) => assert_eq!(name, "opencode"),
            other => panic!("expected coding agent, got {:?}", other),
        }
    }

    #[test]
    fn test_select_model_picks_preferred_small_for_conversation() {
        let available = vec!["llama3.1:8b".to_string(), "qwen2.5:0.5b".to_string()];
        let sel = select_model(&intent(IntentCategory::Conversation), &available);
        assert_eq!(sel.model, "qwen2.5:0.5b");
        assert!(matches!(sel.tier, ModelTier::Small));
    }

    #[test]
    fn test_select_model_coding_prefers_coder_over_generic() {
        // Both a coder model and a generic 7b are present; the coder wins.
        let available = vec!["qwen2.5:7b".to_string(), "qwen2.5-coder:7b".to_string()];
        let sel = select_model(&intent(IntentCategory::Coding), &available);
        assert_eq!(sel.model, "qwen2.5-coder:7b");
        assert!(matches!(sel.tier, ModelTier::Code));
    }

    #[test]
    fn test_select_model_information_uses_medium_tier() {
        let available = vec!["qwen2.5:7b".to_string()];
        let sel = select_model(&intent(IntentCategory::Information), &available);
        assert_eq!(sel.model, "qwen2.5:7b");
        assert!(matches!(sel.tier, ModelTier::Medium));
    }

    #[test]
    fn test_select_model_falls_back_to_first_available_when_no_match() {
        // None of the preferred small-tier models are present, so the first
        // available model is used as a fallback.
        let available = vec!["some-random-model:3b".to_string()];
        let sel = select_model(&intent(IntentCategory::Conversation), &available);
        assert_eq!(sel.model, "some-random-model:3b");
    }

    #[test]
    fn test_select_model_defaults_when_nothing_available() {
        let available: Vec<String> = vec![];
        let sel = select_model(&intent(IntentCategory::Conversation), &available);
        assert_eq!(sel.model, "qwen2.5:0.5b");
    }
}
