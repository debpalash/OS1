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

    /// Audit log path
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

    /// Load capability definitions from config
    pub async fn init(&mut self) -> Result<()> {
        tracing::info!("Security engine initializing...");
        // TODO: Load agent capabilities from /etc/indos/security/
        // TODO: Initialize audit log
        self.ready = true;
        Ok(())
    }

    /// Check if an agent is allowed to perform an action
    pub async fn check(
        &self,
        agent_id: &str,
        action: &str,
        target: &str,
    ) -> Result<AuditResult> {
        tracing::debug!(
            "Security check: agent={} action={} target={}",
            agent_id,
            action,
            target
        );
        // TODO: Look up agent capabilities
        // TODO: Evaluate against rules
        // TODO: Write audit entry
        Ok(AuditResult::Allowed)
    }

    /// Register a new agent with capabilities
    pub fn register_agent(&mut self, capability: AgentCapability) {
        tracing::info!("Registered agent: {}", capability.agent_id);
        self.capabilities.push(capability);
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
