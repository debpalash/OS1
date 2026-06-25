//! IndOS Privacy — Local PII Redaction Layer
//!
//! Mandatory filter that sits between user data and any API-bound request.
//! Uses regex-based pattern matching for instant PII detection + optional
//! model-based detection via Ollama for complex cases.
//!
//! Privacy zones:
//! - **Green Zone** (local): Data stays on device. No filtering needed.
//! - **Yellow Zone** (filtered): API-bound data. PII stripped.
//! - **Red Zone** (blocked): User-marked "never send" data. Hard exclusion.

use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Result of privacy filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterResult {
    /// The sanitized text (PII replaced with tokens)
    pub sanitized_text: String,

    /// PII entities that were detected and redacted
    pub redacted_entities: Vec<RedactedEntity>,

    /// Whether any PII was found
    pub had_pii: bool,

    /// Privacy zone classification
    pub zone: PrivacyZone,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactedEntity {
    /// Type of PII: "EMAIL", "PHONE", "SSN", "CREDIT_CARD", "IP", "API_KEY", etc.
    pub entity_type: String,

    /// Original text (stored locally for potential de-redaction)
    pub original: String,

    /// Replacement token used in sanitized text
    pub token: String,

    /// Position in original text
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PrivacyZone {
    /// Local only — no filtering needed
    Green,
    /// API-bound — PII must be stripped
    Yellow,
    /// Never send — data is blocked entirely
    Red,
}

/// PII pattern definition
struct PiiPattern {
    entity_type: &'static str,
    regex: Regex,
    token_prefix: &'static str,
}

/// The privacy filter engine
pub struct PrivacyFilter {
    /// Compiled PII detection patterns
    patterns: Vec<PiiPattern>,
    /// User-defined "never send" patterns
    red_zone_patterns: Vec<Regex>,
    /// Redaction counter (for unique tokens)
    counter: std::sync::atomic::AtomicUsize,
    ready: bool,
}

impl Default for PrivacyFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl PrivacyFilter {
    pub fn new() -> Self {
        let patterns = vec![
            // Email addresses
            PiiPattern {
                entity_type: "EMAIL",
                regex: Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap(),
                token_prefix: "[EMAIL",
            },
            // Phone numbers (various formats)
            PiiPattern {
                entity_type: "PHONE",
                regex: Regex::new(r"(?:\+?1[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}").unwrap(),
                token_prefix: "[PHONE",
            },
            // SSN
            PiiPattern {
                entity_type: "SSN",
                regex: Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap(),
                token_prefix: "[SSN",
            },
            // Credit card numbers (basic — 13-19 digits with optional separators)
            PiiPattern {
                entity_type: "CREDIT_CARD",
                regex: Regex::new(r"\b(?:\d{4}[-\s]?){3,4}\d{0,4}\b").unwrap(),
                token_prefix: "[CARD",
            },
            // IP addresses (v4)
            PiiPattern {
                entity_type: "IP_ADDRESS",
                regex: Regex::new(
                    r"\b(?:(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.){3}(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\b",
                )
                .unwrap(),
                token_prefix: "[IP",
            },
            // API keys / tokens (long alphanumeric strings that look like secrets)
            PiiPattern {
                entity_type: "API_KEY",
                regex: Regex::new(r"(?:sk|pk|api|key|token|secret|password)[-_]?[a-zA-Z0-9]{20,}")
                    .unwrap(),
                token_prefix: "[KEY",
            },
            // Bearer tokens
            PiiPattern {
                entity_type: "BEARER_TOKEN",
                regex: Regex::new(r"Bearer\s+[a-zA-Z0-9._-]{20,}").unwrap(),
                token_prefix: "[TOKEN",
            },
            // AWS-style keys
            PiiPattern {
                entity_type: "AWS_KEY",
                regex: Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
                token_prefix: "[AWS_KEY",
            },
            // Aadhaar number (Indian ID — 12 digits)
            PiiPattern {
                entity_type: "AADHAAR",
                regex: Regex::new(r"\b\d{4}\s?\d{4}\s?\d{4}\b").unwrap(),
                token_prefix: "[AADHAAR",
            },
            // PAN card (Indian tax ID)
            PiiPattern {
                entity_type: "PAN",
                regex: Regex::new(r"\b[A-Z]{5}\d{4}[A-Z]\b").unwrap(),
                token_prefix: "[PAN",
            },
        ];

        Self {
            patterns,
            red_zone_patterns: Vec::new(),
            counter: std::sync::atomic::AtomicUsize::new(0),
            ready: true,
        }
    }

    /// Add a user-defined "never send" pattern (Red Zone)
    pub fn add_red_zone_pattern(&mut self, pattern: &str) -> Result<()> {
        let regex = Regex::new(pattern)?;
        self.red_zone_patterns.push(regex);
        Ok(())
    }

    /// Filter text before sending to an API provider
    pub fn filter(&self, text: &str, zone: PrivacyZone) -> Result<FilterResult> {
        match zone {
            PrivacyZone::Green => {
                // Local only — pass through unmodified
                Ok(FilterResult {
                    sanitized_text: text.to_string(),
                    redacted_entities: vec![],
                    had_pii: false,
                    zone: PrivacyZone::Green,
                })
            }
            PrivacyZone::Red => {
                // Never send — block entirely
                anyhow::bail!("Data is in Red Zone — transmission blocked")
            }
            PrivacyZone::Yellow => {
                // Check for Red Zone patterns first
                for pattern in &self.red_zone_patterns {
                    if pattern.is_match(text) {
                        anyhow::bail!(
                            "Text matches Red Zone pattern '{}' — transmission blocked",
                            pattern.as_str()
                        );
                    }
                }
                // API-bound — run PII detection and redaction
                self.detect_and_redact(text)
            }
        }
    }

    /// Detect and redact PII using pattern matching
    fn detect_and_redact(&self, text: &str) -> Result<FilterResult> {
        let mut sanitized = text.to_string();
        let mut entities = Vec::new();

        for pattern in &self.patterns {
            let matches: Vec<_> = pattern.regex.find_iter(text).collect();
            for m in matches {
                let count = self
                    .counter
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let token = format!("{}_{:03}]", pattern.token_prefix, count);

                entities.push(RedactedEntity {
                    entity_type: pattern.entity_type.to_string(),
                    original: m.as_str().to_string(),
                    token: token.clone(),
                    start: m.start(),
                    end: m.end(),
                });

                // Replace in sanitized text (replace first occurrence to handle overlaps)
                sanitized = sanitized.replacen(m.as_str(), &token, 1);
            }
        }

        let had_pii = !entities.is_empty();
        if had_pii {
            tracing::info!(
                "Privacy: redacted {} PII entities ({})",
                entities.len(),
                entities
                    .iter()
                    .map(|e| e.entity_type.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        Ok(FilterResult {
            sanitized_text: sanitized,
            redacted_entities: entities,
            had_pii,
            zone: PrivacyZone::Yellow,
        })
    }

    /// De-redact: restore original PII from tokens (for local display)
    pub fn de_redact(&self, sanitized: &str, entities: &[RedactedEntity]) -> String {
        let mut restored = sanitized.to_string();
        for entity in entities {
            restored = restored.replace(&entity.token, &entity.original);
        }
        restored
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_redaction() {
        let filter = PrivacyFilter::new();
        let result = filter
            .filter("Contact me at john@example.com", PrivacyZone::Yellow)
            .unwrap();
        assert!(result.had_pii);
        assert!(!result.sanitized_text.contains("john@example.com"));
        assert!(result.sanitized_text.contains("[EMAIL"));
        assert_eq!(result.redacted_entities.len(), 1);
        assert_eq!(result.redacted_entities[0].entity_type, "EMAIL");
    }

    #[test]
    fn test_phone_redaction() {
        let filter = PrivacyFilter::new();
        let result = filter
            .filter("Call me at 555-123-4567", PrivacyZone::Yellow)
            .unwrap();
        assert!(result.had_pii);
        assert!(!result.sanitized_text.contains("555-123-4567"));
    }

    #[test]
    fn test_api_key_redaction() {
        let filter = PrivacyFilter::new();
        let result = filter
            .filter(
                "My key is sk-abc123def456ghi789jkl012mno345",
                PrivacyZone::Yellow,
            )
            .unwrap();
        assert!(result.had_pii);
        assert!(!result.sanitized_text.contains("sk-abc123"));
    }

    #[test]
    fn test_green_zone_passthrough() {
        let filter = PrivacyFilter::new();
        let result = filter
            .filter("john@example.com", PrivacyZone::Green)
            .unwrap();
        assert!(!result.had_pii);
        assert_eq!(result.sanitized_text, "john@example.com");
    }

    #[test]
    fn test_red_zone_blocks() {
        let filter = PrivacyFilter::new();
        let result = filter.filter("anything", PrivacyZone::Red);
        assert!(result.is_err());
    }

    #[test]
    fn test_de_redact() {
        let filter = PrivacyFilter::new();
        let result = filter
            .filter("Email: pal@indos.dev", PrivacyZone::Yellow)
            .unwrap();
        let restored = filter.de_redact(&result.sanitized_text, &result.redacted_entities);
        assert!(restored.contains("pal@indos.dev"));
    }

    #[test]
    fn test_multiple_pii() {
        let filter = PrivacyFilter::new();
        let result = filter
            .filter(
                "Email john@test.com, phone 555-111-2222, SSN 123-45-6789",
                PrivacyZone::Yellow,
            )
            .unwrap();
        assert!(result.had_pii);
        assert!(result.redacted_entities.len() >= 3);
        assert!(!result.sanitized_text.contains("john@test.com"));
        assert!(!result.sanitized_text.contains("123-45-6789"));
    }

    #[test]
    fn test_no_pii() {
        let filter = PrivacyFilter::new();
        let result = filter
            .filter("Hello world, how are you?", PrivacyZone::Yellow)
            .unwrap();
        assert!(!result.had_pii);
        assert_eq!(result.sanitized_text, "Hello world, how are you?");
    }

    #[test]
    fn test_red_zone_pattern() {
        let mut filter = PrivacyFilter::new();
        filter.add_red_zone_pattern(r"TOP SECRET").unwrap();
        let result = filter.filter("This is TOP SECRET data", PrivacyZone::Yellow);
        assert!(result.is_err());
    }
}
