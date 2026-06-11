//! IndOS MCP Tools — OS capabilities exposed to the LLM
//!
//! When the orchestrator receives a user intent that requires system interaction,
//! these tools execute the actual work. The LLM generates tool calls, we execute them.
//!
//! Tool categories:
//! - **filesystem**: ls, cat, write, search, tree
//! - **system**: exec, info, services, packages
//! - **web**: fetch URL to markdown (via Firecrawl or builtin)
//! - **fragments**: generate UI fragments for the shell

use anyhow::Result;
use indos_security::{AuditResult, SecurityEngine};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::process::Command;

/// A tool definition exposed to the LLM
#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Result of executing a tool
#[derive(Debug, Clone, Serialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fragment: Option<FragmentSpec>,
}

/// A UI fragment to render in the shell
#[derive(Debug, Clone, Serialize)]
pub struct FragmentSpec {
    pub kind: String,
    pub title: String,
    pub data: serde_json::Value,
}

/// Get all available tool definitions (for LLM system prompt)
pub fn available_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "list_files".into(),
            description: "List files and directories at a path".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Directory path to list"},
                    "show_hidden": {"type": "boolean", "default": false}
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "read_file".into(),
            description: "Read the contents of a file".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "File path to read"},
                    "max_lines": {"type": "integer", "default": 100}
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "write_file".into(),
            description: "Write content to a file (creates or overwrites)".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "content": {"type": "string"}
                },
                "required": ["path", "content"]
            }),
        },
        ToolDefinition {
            name: "run_command".into(),
            description: "Execute a shell command and return output".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Shell command to execute"},
                    "timeout_secs": {"type": "integer", "default": 30}
                },
                "required": ["command"]
            }),
        },
        ToolDefinition {
            name: "system_info".into(),
            description: "Get system information (CPU, RAM, disk, OS)".into(),
            parameters: serde_json::json!({"type": "object", "properties": {}}),
        },
        ToolDefinition {
            name: "search_files".into(),
            description: "Search for files by name pattern".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "Glob or name pattern"},
                    "directory": {"type": "string", "default": "."}
                },
                "required": ["pattern"]
            }),
        },
        ToolDefinition {
            name: "package_manager".into(),
            description: "Manage system packages (search, install, remove, list)".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {"type": "string", "enum": ["search", "install", "remove", "info", "list_installed"]},
                    "package": {"type": "string"}
                },
                "required": ["action"]
            }),
        },
    ]
}

/// Generate the tools section for the LLM system prompt
pub fn tools_prompt() -> String {
    let tools = available_tools();
    let mut prompt = String::from(
        "\n\nYou have access to these tools. Call them by outputting JSON in this format:\n\
         ```tool\n{\"tool\": \"<name>\", \"args\": {<parameters>}}\n```\n\n\
         Available tools:\n",
    );

    for tool in &tools {
        prompt.push_str(&format!("- **{}**: {}\n", tool.name, tool.description));
    }

    prompt
}

/// Execute a tool call and return the result
pub async fn execute_tool(name: &str, args: &serde_json::Value) -> ToolResult {
    match name {
        "list_files" => tool_list_files(args).await,
        "read_file" => tool_read_file(args).await,
        "write_file" => tool_write_file(args).await,
        "run_command" => tool_run_command(args).await,
        "system_info" => tool_system_info().await,
        "search_files" => tool_search_files(args).await,
        "package_manager" => tool_package_manager(args).await,
        _ => ToolResult {
            success: false,
            output: format!("Unknown tool: {}", name),
            fragment: None,
        },
    }
}

/// Execute a tool call with security checks.
/// Maps each tool to a security action+target, checks with SecurityEngine,
/// then dispatches to the actual implementation if allowed.
pub async fn execute_tool_checked(
    name: &str,
    args: &serde_json::Value,
    security: &SecurityEngine,
    agent_id: &str,
) -> ToolResult {
    let (action, target) = match name {
        "list_files" => ("read", args["path"].as_str().unwrap_or(".").to_string()),
        "read_file" => ("read", args["path"].as_str().unwrap_or("").to_string()),
        "write_file" => ("write", args["path"].as_str().unwrap_or("").to_string()),
        "search_files" => ("read", args["directory"].as_str().unwrap_or(".").to_string()),
        "run_command" => ("spawn", args["command"].as_str().unwrap_or("").to_string()),
        "package_manager" => ("spawn", format!("pacman {}", args["action"].as_str().unwrap_or(""))),
        "system_info" => ("read", "/proc".to_string()),
        _ => {
            return ToolResult {
                success: false,
                output: format!("Unknown tool: {}", name),
                fragment: None,
            };
        }
    };

    match security.check(agent_id, action, &target) {
        AuditResult::Allowed => execute_tool(name, args).await,
        AuditResult::Denied => ToolResult {
            success: false,
            output: format!("SECURITY DENIED: {} not allowed to {} '{}'", agent_id, action, target),
            fragment: None,
        },
        AuditResult::RequiresApproval => ToolResult {
            success: false,
            output: format!("APPROVAL REQUIRED: {} wants to {} '{}'", agent_id, action, target),
            fragment: None,
        },
        AuditResult::Error(e) => ToolResult {
            success: false,
            output: format!("Security error: {}", e),
            fragment: None,
        },
    }
}

// ═══════════════════════════════════════════════════════════════
// Tool implementations
// ═══════════════════════════════════════════════════════════════

async fn tool_list_files(args: &serde_json::Value) -> ToolResult {
    let path = args["path"].as_str().unwrap_or(".");
    let show_hidden = args["show_hidden"].as_bool().unwrap_or(false);

    let mut cmd = Command::new("ls");
    cmd.arg("-la").arg("--color=never");
    if !show_hidden {
        cmd.arg("--ignore=.*");
    }
    cmd.arg(path);

    match cmd.output().await {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            ToolResult {
                success: true,
                output: stdout,
                fragment: None,
            }
        }
        Ok(output) => ToolResult {
            success: false,
            output: String::from_utf8_lossy(&output.stderr).to_string(),
            fragment: None,
        },
        Err(e) => ToolResult {
            success: false,
            output: format!("Failed to list: {}", e),
            fragment: None,
        },
    }
}

async fn tool_read_file(args: &serde_json::Value) -> ToolResult {
    let path = args["path"].as_str().unwrap_or("");
    let max_lines = args["max_lines"].as_u64().unwrap_or(100) as usize;

    match tokio::fs::read_to_string(path).await {
        Ok(content) => {
            let truncated: String = content
                .lines()
                .take(max_lines)
                .collect::<Vec<_>>()
                .join("\n");
            let total_lines = content.lines().count();
            let note = if total_lines > max_lines {
                format!("\n\n[... truncated, showing {}/{} lines]", max_lines, total_lines)
            } else {
                String::new()
            };
            ToolResult {
                success: true,
                output: format!("{}{}", truncated, note),
                fragment: None,
            }
        }
        Err(e) => ToolResult {
            success: false,
            output: format!("Failed to read {}: {}", path, e),
            fragment: None,
        },
    }
}

async fn tool_write_file(args: &serde_json::Value) -> ToolResult {
    let path = args["path"].as_str().unwrap_or("");
    let content = args["content"].as_str().unwrap_or("");

    // Safety: don't write to system paths without confirmation
    if path.starts_with("/etc") || path.starts_with("/usr") || path.starts_with("/boot") {
        return ToolResult {
            success: false,
            output: format!("BLOCKED: Writing to system path '{}' requires confirmation", path),
            fragment: None,
        };
    }

    // Create parent dirs if needed
    if let Some(parent) = Path::new(path).parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }

    match tokio::fs::write(path, content).await {
        Ok(()) => ToolResult {
            success: true,
            output: format!("Wrote {} bytes to {}", content.len(), path),
            fragment: None,
        },
        Err(e) => ToolResult {
            success: false,
            output: format!("Failed to write {}: {}", path, e),
            fragment: None,
        },
    }
}

async fn tool_run_command(args: &serde_json::Value) -> ToolResult {
    let command = args["command"].as_str().unwrap_or("");
    let timeout = args["timeout_secs"].as_u64().unwrap_or(30);

    // Safety: block destructive commands
    let blocked = ["rm -rf /", "mkfs", "dd if=", "> /dev/sd", "shutdown", "reboot"];
    for b in &blocked {
        if command.contains(b) {
            return ToolResult {
                success: false,
                output: format!("BLOCKED: Dangerous command pattern detected: {}", b),
                fragment: None,
            };
        }
    }

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout),
        Command::new("bash").args(["-c", command]).output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = if stderr.is_empty() {
                stdout.to_string()
            } else {
                format!("{}\n[stderr]: {}", stdout, stderr)
            };
            ToolResult {
                success: output.status.success(),
                output: combined,
                fragment: None,
            }
        }
        Ok(Err(e)) => ToolResult {
            success: false,
            output: format!("Command failed: {}", e),
            fragment: None,
        },
        Err(_) => ToolResult {
            success: false,
            output: format!("Command timed out after {}s", timeout),
            fragment: None,
        },
    }
}

async fn tool_system_info() -> ToolResult {
    let mut info = String::new();

    // Hostname
    if let Ok(h) = tokio::fs::read_to_string("/etc/hostname").await {
        info.push_str(&format!("Hostname: {}\n", h.trim()));
    }

    // OS
    if let Ok(os) = tokio::fs::read_to_string("/etc/os-release").await {
        for line in os.lines() {
            if line.starts_with("PRETTY_NAME=") {
                info.push_str(&format!("OS: {}\n", line.trim_start_matches("PRETTY_NAME=").trim_matches('"')));
            }
        }
    }

    // Kernel
    if let Ok(output) = Command::new("uname").arg("-r").output().await {
        info.push_str(&format!("Kernel: {}\n", String::from_utf8_lossy(&output.stdout).trim()));
    }

    // CPU
    if let Ok(cpuinfo) = tokio::fs::read_to_string("/proc/cpuinfo").await {
        if let Some(model) = cpuinfo.lines().find(|l| l.starts_with("model name")) {
            if let Some(name) = model.split(':').nth(1) {
                info.push_str(&format!("CPU: {}\n", name.trim()));
            }
        }
    }

    // Memory
    if let Ok(meminfo) = tokio::fs::read_to_string("/proc/meminfo").await {
        let mut total = 0u64;
        let mut avail = 0u64;
        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                total = line.split_whitespace().nth(1).and_then(|v| v.parse().ok()).unwrap_or(0);
            }
            if line.starts_with("MemAvailable:") {
                avail = line.split_whitespace().nth(1).and_then(|v| v.parse().ok()).unwrap_or(0);
            }
        }
        let used = total.saturating_sub(avail);
        info.push_str(&format!(
            "Memory: {:.1} GB / {:.1} GB ({:.0}% used)\n",
            used as f64 / 1_048_576.0,
            total as f64 / 1_048_576.0,
            if total > 0 { used as f64 / total as f64 * 100.0 } else { 0.0 }
        ));
    }

    // Disk
    if let Ok(output) = Command::new("df").args(["-h", "--output=target,size,used,avail,pcent", "/"]).output().await {
        info.push_str(&format!("Disk:\n{}", String::from_utf8_lossy(&output.stdout)));
    }

    // GPU
    if let Ok(output) = Command::new("lspci").output().await {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("VGA") || line.contains("3D") {
                if let Some(name) = line.split(':').last() {
                    info.push_str(&format!("GPU: {}\n", name.trim()));
                }
            }
        }
    }

    // Uptime
    if let Ok(output) = Command::new("uptime").arg("-p").output().await {
        info.push_str(&format!("Uptime: {}\n", String::from_utf8_lossy(&output.stdout).trim()));
    }

    ToolResult {
        success: true,
        output: info,
        fragment: Some(FragmentSpec {
            kind: "system_monitor".into(),
            title: "System Information".into(),
            data: serde_json::json!({}),
        }),
    }
}

async fn tool_search_files(args: &serde_json::Value) -> ToolResult {
    let pattern = args["pattern"].as_str().unwrap_or("*");
    let directory = args["directory"].as_str().unwrap_or(".");

    let output = Command::new("find")
        .args([directory, "-name", pattern, "-maxdepth", "5", "-type", "f"])
        .output()
        .await;

    match output {
        Ok(out) if out.status.success() => {
            let files = String::from_utf8_lossy(&out.stdout).to_string();
            let count = files.lines().count();
            ToolResult {
                success: true,
                output: format!("Found {} files:\n{}", count, files),
                fragment: None,
            }
        }
        Ok(out) => ToolResult {
            success: false,
            output: String::from_utf8_lossy(&out.stderr).to_string(),
            fragment: None,
        },
        Err(e) => ToolResult {
            success: false,
            output: format!("Search failed: {}", e),
            fragment: None,
        },
    }
}

async fn tool_package_manager(args: &serde_json::Value) -> ToolResult {
    let action = args["action"].as_str().unwrap_or("search");
    let package = args["package"].as_str().unwrap_or("");

    let (cmd, cmd_args): (&str, Vec<&str>) = match action {
        "search" => ("pacman", vec!["-Ss", package]),
        "info" => ("pacman", vec!["-Qi", package]),
        "list_installed" => ("pacman", vec!["-Q", "--color=never"]),
        "install" => {
            return ToolResult {
                success: false,
                output: format!("CONFIRMATION REQUIRED: Install '{}'? (needs sudo)", package),
                fragment: None,
            };
        }
        "remove" => {
            return ToolResult {
                success: false,
                output: format!("CONFIRMATION REQUIRED: Remove '{}'? (needs sudo)", package),
                fragment: None,
            };
        }
        _ => {
            return ToolResult {
                success: false,
                output: format!("Unknown action: {}", action),
                fragment: None,
            };
        }
    };

    match Command::new(cmd).args(&cmd_args).output().await {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            ToolResult {
                success: output.status.success(),
                output: if stdout.is_empty() {
                    String::from_utf8_lossy(&output.stderr).to_string()
                } else {
                    stdout
                },
                fragment: None,
            }
        }
        Err(e) => ToolResult {
            success: false,
            output: format!("Package manager error: {}", e),
            fragment: None,
        },
    }
}
