//! Tool call parsing — extracts tool invocations from LLM output
//!
//! The LLM outputs tool calls in fenced blocks:
//! ```tool
//! {"tool": "list_files", "args": {"path": "/home/user"}}
//! ```
//!
//! This module parses those blocks, executes the tools, and returns results.

use crate::tools;
use indos_security::SecurityEngine;

/// A parsed tool call from LLM output
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub tool: String,
    pub args: serde_json::Value,
}

/// Result of parsing LLM output for tool calls
#[derive(Debug)]
pub struct ParsedResponse {
    /// Text segments (non-tool parts of the response)
    pub text: String,
    /// Extracted tool calls
    pub tool_calls: Vec<ToolCall>,
}

/// Parse LLM output, extracting tool calls and surrounding text
pub fn parse_tool_calls(response: &str) -> ParsedResponse {
    let mut text = String::new();
    let mut tool_calls = Vec::new();
    let mut in_tool_block = false;
    let mut tool_json = String::new();

    for line in response.lines() {
        let trimmed = line.trim();

        if trimmed == "```tool" {
            in_tool_block = true;
            tool_json.clear();
            continue;
        }

        if in_tool_block && trimmed == "```" {
            in_tool_block = false;
            // Try to parse the JSON
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&tool_json) {
                if let Some(tool_name) = val["tool"].as_str() {
                    tool_calls.push(ToolCall {
                        tool: tool_name.to_string(),
                        args: val["args"].clone(),
                    });
                }
            } else {
                tracing::warn!("Failed to parse tool call JSON: {}", tool_json.trim());
                text.push_str(&format!("[malformed tool call: {}]\n", tool_json.trim()));
            }
            tool_json.clear();
            continue;
        }

        if in_tool_block {
            tool_json.push_str(line);
            tool_json.push('\n');
        } else {
            text.push_str(line);
            text.push('\n');
        }
    }

    // Handle unclosed tool block
    if in_tool_block && !tool_json.is_empty() {
        text.push_str(&format!("[unclosed tool block: {}]\n", tool_json.trim()));
    }

    ParsedResponse {
        text: text.trim_end().to_string(),
        tool_calls,
    }
}

/// Execute all tool calls with security checks and format results
pub async fn execute_tool_calls(calls: &[ToolCall], security: &SecurityEngine) -> String {
    let mut results = String::new();

    for (i, call) in calls.iter().enumerate() {
        tracing::info!("Executing tool: {} ({})", call.tool, call.args);
        let result = tools::execute_tool_checked(&call.tool, &call.args, security, "system").await;

        results.push_str(&format!(
            "Tool `{}` {}:\n{}\n",
            call.tool,
            if result.success {
                "succeeded"
            } else {
                "failed"
            },
            result.output
        ));

        if i < calls.len() - 1 {
            results.push('\n');
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_tool_calls() {
        let parsed = parse_tool_calls("Hello, how can I help you today?");
        assert!(parsed.tool_calls.is_empty());
        assert_eq!(parsed.text, "Hello, how can I help you today?");
    }

    #[test]
    fn test_single_tool_call() {
        let input = r#"Let me list your files.
```tool
{"tool": "list_files", "args": {"path": "/home"}}
```
Here are the results."#;
        let parsed = parse_tool_calls(input);
        assert_eq!(parsed.tool_calls.len(), 1);
        assert_eq!(parsed.tool_calls[0].tool, "list_files");
        assert!(parsed.text.contains("Let me list your files."));
        assert!(parsed.text.contains("Here are the results."));
    }

    #[test]
    fn test_multiple_tool_calls() {
        let input = r#"I'll check your system.
```tool
{"tool": "system_info", "args": {}}
```
And list your files.
```tool
{"tool": "list_files", "args": {"path": "."}}
```
Done."#;
        let parsed = parse_tool_calls(input);
        assert_eq!(parsed.tool_calls.len(), 2);
        assert_eq!(parsed.tool_calls[0].tool, "system_info");
        assert_eq!(parsed.tool_calls[1].tool, "list_files");
    }

    #[test]
    fn test_malformed_json() {
        let input = "```tool\n{invalid json}\n```";
        let parsed = parse_tool_calls(input);
        assert!(parsed.tool_calls.is_empty());
        assert!(parsed.text.contains("malformed tool call"));
    }

    #[test]
    fn test_unclosed_tool_block() {
        // A tool fence that never closes should not yield a tool call and
        // should surface the leftover content as text for debugging.
        let input = "Here goes:\n```tool\n{\"tool\": \"list_files\", \"args\": {}}";
        let parsed = parse_tool_calls(input);
        assert!(parsed.tool_calls.is_empty());
        assert!(parsed.text.contains("unclosed tool block"));
    }

    #[test]
    fn test_tool_call_preserves_args() {
        let input = "```tool\n{\"tool\": \"read_file\", \"args\": {\"path\": \"/etc/hosts\"}}\n```";
        let parsed = parse_tool_calls(input);
        assert_eq!(parsed.tool_calls.len(), 1);
        assert_eq!(parsed.tool_calls[0].tool, "read_file");
        assert_eq!(parsed.tool_calls[0].args["path"], "/etc/hosts");
    }
}
