//! IndOS Security — Agent Sandboxing & Capability Model
//!
//! Provides:
//! - **Agent capabilities** — per-agent permission model (filesystem paths, network, processes)
//! - **Sandbox enforcement** — bubblewrap/namespace isolation for agent processes
//! - **Audit logging** — immutable log of all agent actions for forensics
//! - **Trust zones** — classify data sensitivity and enforce access boundaries
//!
//! Every agent action goes through this layer:
//! ```text
//! Agent wants to: delete /home/user/important.txt
//!     → Security checks capability: agent has READ on /home/user/ but NOT WRITE
//!     → Action DENIED
//!     → Audit log: "Agent 'opencode' attempted DELETE on /home/user/important.txt — DENIED"
//! ```

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Defines what an agent is allowed to do
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapability {
    /// Agent identifier
    pub agent_id: String,

    /// Filesystem access rules
    pub filesystem: Vec<FsRule>,

    /// Network access rules
    pub network: NetworkRule,

    /// Process execution rules
    pub process: ProcessRule,

    /// Whether the agent requires human approval for destructive actions
    pub requires_approval: bool,

    /// Maximum session duration (seconds). 0 = unlimited.
    pub max_session_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsRule {
    /// Path pattern (glob)
    pub path: String,

    /// Allowed operations
    pub permissions: Vec<FsPermission>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FsPermission {
    Read,
    Write,
    Delete,
    Execute,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRule {
    /// Allow outbound connections
    pub outbound: bool,

    /// Allowed domains (empty = all if outbound is true)
    pub allowed_domains: Vec<String>,

    /// Blocked domains (overrides allowed)
    pub blocked_domains: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRule {
    /// Can spawn child processes
    pub can_spawn: bool,

    /// Allowed commands (empty = any if can_spawn is true)
    pub allowed_commands: Vec<String>,

    /// Blocked commands (overrides allowed)
    pub blocked_commands: Vec<String>,
}

/// An immutable audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub agent_id: String,
    pub action: String,
    pub target: String,
    pub result: AuditResult,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditResult {
    Allowed,
    Denied,
    RequiresApproval,
    Error(String),
}

/// The security engine
pub struct SecurityEngine {
    /// Per-agent capability definitions
    capabilities: Vec<AgentCapability>,

    /// Audit log path (append-only JSONL)
    audit_path: String,

    ready: bool,
}

impl SecurityEngine {
    pub fn new(audit_path: &str) -> Self {
        Self {
            capabilities: Vec::new(),
            audit_path: audit_path.to_string(),
            ready: false,
        }
    }

    /// Initialize with default agent capabilities
    pub fn init(&mut self) -> Result<()> {
        tracing::info!("Security engine initializing...");

        // Ensure audit log directory exists
        if let Some(parent) = std::path::Path::new(&self.audit_path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Register default agents
        self.register_agent(AgentCapability::system_agent());
        self.register_agent(AgentCapability::coding_agent("opencode"));
        self.register_agent(AgentCapability::coding_agent("claude-code"));

        self.ready = true;
        tracing::info!("Security engine ready ({} agents registered)", self.capabilities.len());
        Ok(())
    }

    /// Check if an agent is allowed to perform an action
    pub fn check(
        &self,
        agent_id: &str,
        action: &str,
        target: &str,
    ) -> AuditResult {
        let result = self.evaluate(agent_id, action, target);

        // Write audit entry
        let entry = AuditEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            agent_id: agent_id.to_string(),
            action: action.to_string(),
            target: target.to_string(),
            result: result.clone(),
            details: None,
        };
        self.write_audit(&entry);

        if matches!(result, AuditResult::Denied) {
            tracing::warn!(
                "SECURITY DENIED: agent={} action={} target={}",
                agent_id, action, target
            );
        }

        result
    }

    /// Evaluate an action against capabilities
    fn evaluate(&self, agent_id: &str, action: &str, target: &str) -> AuditResult {
        let cap = match self.capabilities.iter().find(|c| c.agent_id == agent_id) {
            Some(c) => c,
            None => {
                tracing::warn!("Unknown agent: {} — denying by default", agent_id);
                return AuditResult::Denied;
            }
        };

        match action {
            "read" | "write" | "delete" | "execute" => {
                self.check_filesystem(cap, action, target)
            }
            "spawn" => {
                self.check_process(cap, target)
            }
            "network" => {
                self.check_network(cap, target)
            }
            _ => {
                tracing::debug!("Unknown action type: {} — allowing", action);
                AuditResult::Allowed
            }
        }
    }

    /// Check filesystem access
    fn check_filesystem(&self, cap: &AgentCapability, action: &str, path: &str) -> AuditResult {
        let needed_perm = match action {
            "read" => FsPermission::Read,
            "write" => FsPermission::Write,
            "delete" => FsPermission::Delete,
            "execute" => FsPermission::Execute,
            _ => return AuditResult::Denied,
        };

        for rule in &cap.filesystem {
            if path_matches(&rule.path, path) && rule.permissions.contains(&needed_perm) {
                if cap.requires_approval && matches!(needed_perm, FsPermission::Delete) {
                    return AuditResult::RequiresApproval;
                }
                return AuditResult::Allowed;
            }
        }

        AuditResult::Denied
    }

    /// Check process spawn permission
    fn check_process(&self, cap: &AgentCapability, command: &str) -> AuditResult {
        if !cap.process.can_spawn {
            return AuditResult::Denied;
        }

        // Check blocked commands
        for blocked in &cap.process.blocked_commands {
            if command.contains(blocked.as_str()) {
                return AuditResult::Denied;
            }
        }

        // Check allowed commands (empty = allow all)
        if !cap.process.allowed_commands.is_empty() {
            let cmd_name = command.split_whitespace().next().unwrap_or("");
            if !cap.process.allowed_commands.iter().any(|a| a == cmd_name) {
                return AuditResult::Denied;
            }
        }

        AuditResult::Allowed
    }

    /// Check network access
    fn check_network(&self, cap: &AgentCapability, domain: &str) -> AuditResult {
        if !cap.network.outbound {
            return AuditResult::Denied;
        }

        // Check blocked domains
        if cap.network.blocked_domains.iter().any(|d| domain.contains(d.as_str())) {
            return AuditResult::Denied;
        }

        // Check allowed domains (empty = allow all)
        if !cap.network.allowed_domains.is_empty() {
            if !cap.network.allowed_domains.iter().any(|d| domain.contains(d.as_str())) {
                return AuditResult::Denied;
            }
        }

        AuditResult::Allowed
    }

    /// Write audit entry to append-only log
    fn write_audit(&self, entry: &AuditEntry) {
        if let Ok(json) = serde_json::to_string(entry) {
            use std::io::Write;
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.audit_path)
            {
                let _ = writeln!(file, "{}", json);
            }
        }
    }

    /// Register a new agent with capabilities
    pub fn register_agent(&mut self, capability: AgentCapability) {
        tracing::info!("Registered agent: {}", capability.agent_id);
        self.capabilities.push(capability);
    }

    /// Generate bubblewrap sandbox command for agent isolation
    pub fn sandbox_command(&self, agent_id: &str, command: &str) -> Option<Vec<String>> {
        let cap = self.capabilities.iter().find(|c| c.agent_id == agent_id)?;

        let mut args = vec![
            "bwrap".to_string(),
            "--ro-bind".into(), "/usr".into(), "/usr".into(),
            "--ro-bind".into(), "/lib".into(), "/lib".into(),
            "--ro-bind".into(), "/lib64".into(), "/lib64".into(),
            "--ro-bind".into(), "/bin".into(), "/bin".into(),
            "--ro-bind".into(), "/etc/resolv.conf".into(), "/etc/resolv.conf".into(),
            "--proc".into(), "/proc".into(),
            "--dev".into(), "/dev".into(),
            "--tmpfs".into(), "/tmp".into(),
        ];

        // Mount filesystem paths based on capabilities
        for rule in &cap.filesystem {
            let bind_type = if rule.permissions.contains(&FsPermission::Write) {
                "--bind"
            } else {
                "--ro-bind"
            };
            // Expand ~ to home
            let path = rule.path.replace("~", &std::env::var("HOME").unwrap_or_default());
            let clean_path = path.trim_end_matches("/**");
            args.push(bind_type.into());
            args.push(clean_path.into());
            args.push(clean_path.into());
        }

        // Network isolation
        if !cap.network.outbound {
            args.push("--unshare-net".into());
        }

        // Add the actual command
        args.push("--".into());
        for part in command.split_whitespace() {
            args.push(part.into());
        }

        Some(args)
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }
}

/// Default capabilities for built-in agents
impl AgentCapability {
    /// System agent — full access (it IS the OS)
    pub fn system_agent() -> Self {
        Self {
            agent_id: "system".into(),
            filesystem: vec![FsRule {
                path: "/".into(),
                permissions: vec![
                    FsPermission::Read,
                    FsPermission::Write,
                    FsPermission::Delete,
                    FsPermission::Execute,
                ],
            }],
            network: NetworkRule {
                outbound: true,
                allowed_domains: vec![],
                blocked_domains: vec![],
            },
            process: ProcessRule {
                can_spawn: true,
                allowed_commands: vec![],
                blocked_commands: vec![],
            },
            requires_approval: true, // Still requires approval for destructive ops
            max_session_seconds: 0,
        }
    }

    /// Coding agent — access to user projects only
    pub fn coding_agent(agent_id: &str) -> Self {
        Self {
            agent_id: agent_id.into(),
            filesystem: vec![
                FsRule {
                    path: "~/projects/**".into(),
                    permissions: vec![
                        FsPermission::Read,
                        FsPermission::Write,
                        FsPermission::Execute,
                    ],
                },
                FsRule {
                    path: "/tmp/indos-*/**".into(),
                    permissions: vec![
                        FsPermission::Read,
                        FsPermission::Write,
                        FsPermission::Delete,
                    ],
                },
            ],
            network: NetworkRule {
                outbound: true,
                allowed_domains: vec![
                    "github.com".into(),
                    "api.anthropic.com".into(),
                    "api.openai.com".into(),
                ],
                blocked_domains: vec![],
            },
            process: ProcessRule {
                can_spawn: true,
                allowed_commands: vec![
                    "git".into(),
                    "cargo".into(),
                    "npm".into(),
                    "python".into(),
                    "node".into(),
                ],
                blocked_commands: vec![
                    "rm -rf /".into(),
                    "dd".into(),
                    "mkfs".into(),
                ],
            },
            requires_approval: true,
            max_session_seconds: 3600, // 1 hour max
        }
    }
}

/// Simple glob-like path matching
fn path_matches(pattern: &str, path: &str) -> bool {
    // Handle root pattern
    if pattern == "/" {
        return true;
    }

    let clean_pattern = pattern
        .replace("~", &std::env::var("HOME").unwrap_or_default())
        .replace("/**", "");

    path.starts_with(&clean_pattern)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_engine() -> SecurityEngine {
        let mut engine = SecurityEngine::new("/tmp/indos-test-audit.jsonl");
        engine.init().unwrap();
        engine
    }

    #[test]
    fn test_system_agent_read_allowed() {
        let engine = test_engine();
        let result = engine.check("system", "read", "/etc/hostname");
        assert!(matches!(result, AuditResult::Allowed));
    }

    #[test]
    fn test_system_agent_delete_needs_approval() {
        let engine = test_engine();
        let result = engine.check("system", "delete", "/home/user/file.txt");
        assert!(matches!(result, AuditResult::RequiresApproval));
    }

    #[test]
    fn test_unknown_agent_denied() {
        let engine = test_engine();
        let result = engine.check("unknown-agent", "read", "/etc/passwd");
        assert!(matches!(result, AuditResult::Denied));
    }

    #[test]
    fn test_coding_agent_blocked_command() {
        let engine = test_engine();
        let result = engine.check("opencode", "spawn", "rm -rf /");
        assert!(matches!(result, AuditResult::Denied));
    }

    #[test]
    fn test_coding_agent_allowed_command() {
        let engine = test_engine();
        let result = engine.check("opencode", "spawn", "git status");
        assert!(matches!(result, AuditResult::Allowed));
    }

    #[test]
    fn test_coding_agent_network_allowed() {
        let engine = test_engine();
        let result = engine.check("opencode", "network", "github.com");
        assert!(matches!(result, AuditResult::Allowed));
    }

    #[test]
    fn test_coding_agent_network_denied() {
        let engine = test_engine();
        let result = engine.check("opencode", "network", "evil.com");
        assert!(matches!(result, AuditResult::Denied));
    }

    #[test]
    fn test_sandbox_command_generation() {
        let engine = test_engine();
        let cmd = engine.sandbox_command("opencode", "git status");
        assert!(cmd.is_some());
        let args = cmd.unwrap();
        assert_eq!(args[0], "bwrap");
        assert!(args.contains(&"--".to_string()));
        assert!(args.contains(&"git".to_string()));
    }
}
