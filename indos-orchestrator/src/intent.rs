//! Intent classification — determines what kind of task the user wants
//!
//! Phase 1: keyword-based heuristics (instant, no LLM call)
//! Phase 2: will add LLM-based classification for ambiguous inputs

use serde::{Deserialize, Serialize};

/// Classified user intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    /// Intent category
    pub category: IntentCategory,

    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,

    /// Original user text
    pub raw_input: String,

    /// Extracted parameters
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntentCategory {
    /// File operations: browse, create, edit, delete, search
    Filesystem,

    /// System management: packages, services, settings, updates
    System,

    /// Coding: write code, debug, refactor, explain
    Coding,

    /// Information: search, explain, summarize
    Information,

    /// Communication: email, messages
    Communication,

    /// Media: play, convert, edit media files
    Media,

    /// Conversation: just chatting, no specific task
    Conversation,

    /// Voice: control voice pipeline
    Voice,

    /// Unknown: couldn't classify
    Unknown,
}

// Keyword sets for each category
const FS_KEYWORDS: &[&str] = &[
    "file",
    "folder",
    "directory",
    "ls",
    "list",
    "create",
    "delete",
    "move",
    "copy",
    "rename",
    "open",
    "save",
    "find",
    "search file",
    "browse",
    "tree",
    "cat",
    "read",
    "write",
    "edit",
    "path",
    "home",
    "desktop",
];

const SYS_KEYWORDS: &[&str] = &[
    "install",
    "update",
    "upgrade",
    "package",
    "pacman",
    "service",
    "systemd",
    "restart",
    "start",
    "stop",
    "enable",
    "disable",
    "status",
    "process",
    "kill",
    "memory",
    "cpu",
    "disk",
    "system",
    "info",
    "uptime",
    "reboot",
    "shutdown",
    "network",
    "wifi",
    "bluetooth",
    "volume",
    "brightness",
];

const CODE_KEYWORDS: &[&str] = &[
    "code",
    "function",
    "class",
    "debug",
    "compile",
    "build",
    "run",
    "test",
    "refactor",
    "git",
    "commit",
    "push",
    "pull",
    "branch",
    "error",
    "bug",
    "fix",
    "implement",
    "rust",
    "python",
    "javascript",
];

const INFO_KEYWORDS: &[&str] = &[
    "what is",
    "explain",
    "how to",
    "why",
    "tell me",
    "describe",
    "summarize",
    "compare",
    "difference",
    "meaning",
    "definition",
];

const MEDIA_KEYWORDS: &[&str] = &[
    "play",
    "music",
    "video",
    "audio",
    "image",
    "photo",
    "screenshot",
    "record",
    "stream",
    "convert",
    "resize",
];

const VOICE_KEYWORDS: &[&str] = &[
    "voice",
    "speak",
    "listen",
    "mute",
    "unmute",
    "dictation",
    "speech",
    "microphone",
    "mic",
];

/// Classify user input into an intent (keyword-based heuristic)
pub fn classify(input: &str) -> Intent {
    let lower = input.to_lowercase();

    let mut best_category = IntentCategory::Conversation;
    let mut best_score = 0.0f32;

    let checks: Vec<(IntentCategory, &[&str])> = vec![
        (IntentCategory::Filesystem, FS_KEYWORDS),
        (IntentCategory::System, SYS_KEYWORDS),
        (IntentCategory::Coding, CODE_KEYWORDS),
        (IntentCategory::Information, INFO_KEYWORDS),
        (IntentCategory::Media, MEDIA_KEYWORDS),
        (IntentCategory::Voice, VOICE_KEYWORDS),
    ];

    for (category, keywords) in checks {
        let matches = keywords.iter().filter(|kw| lower.contains(*kw)).count();

        if matches > 0 {
            let score = matches as f32 / keywords.len() as f32;
            // Boost: more matches = more confidence
            let boosted = (score * 3.0).min(1.0);
            if boosted > best_score {
                best_score = boosted;
                best_category = category;
            }
        }
    }

    // If nothing matched well, it's conversation
    if best_score < 0.05 {
        best_category = IntentCategory::Conversation;
        best_score = 0.7; // Default confidence for chat
    }

    Intent {
        category: best_category,
        confidence: best_score,
        raw_input: input.to_string(),
        params: serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filesystem_intent() {
        let intent = classify("list files in my home directory");
        assert!(matches!(intent.category, IntentCategory::Filesystem));
    }

    #[test]
    fn test_system_intent() {
        let intent = classify("install htop");
        assert!(matches!(intent.category, IntentCategory::System));
    }

    #[test]
    fn test_code_intent() {
        let intent = classify("debug this rust function");
        assert!(matches!(intent.category, IntentCategory::Coding));
    }

    #[test]
    fn test_conversation_fallback() {
        let intent = classify("hello how are you");
        assert!(matches!(intent.category, IntentCategory::Conversation));
    }

    #[test]
    fn test_info_intent() {
        let intent = classify("what is wayland and how to use it");
        assert!(matches!(intent.category, IntentCategory::Information));
    }
}
