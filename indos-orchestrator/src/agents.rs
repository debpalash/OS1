//! Agent dispatch — spawn external coding agents (opencode, claude)
//!
//! Priority: opencode → claude → Ollama fallback
//! Protocol: pipe user message via stdin, stream stdout chunks back via IPC.

use crate::ipc::OrchestratorMessage;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum AgentKind {
    OpenCode,
    Claude,
    SimStudio,
}

impl AgentKind {
    pub fn binary(&self) -> &'static str {
        match self {
            AgentKind::OpenCode => "opencode",
            AgentKind::Claude => "claude",
            AgentKind::SimStudio => "sim-studio",
        }
    }

    pub fn args(&self, message: &str) -> Vec<String> {
        match self {
            AgentKind::OpenCode => vec!["--pipe".into(), message.into()],
            AgentKind::Claude => vec!["--print".into(), message.into()],
            AgentKind::SimStudio => vec!["--run".into(), message.into()],
        }
    }
}

/// Check if a binary exists on PATH.
async fn binary_exists(name: &str) -> bool {
    tokio::process::Command::new("which")
        .arg(name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Detect the best available coding agent.
pub async fn detect_agent() -> Option<AgentKind> {
    if binary_exists("opencode").await {
        return Some(AgentKind::OpenCode);
    }
    if binary_exists("claude").await {
        return Some(AgentKind::Claude);
    }
    if binary_exists("sim-studio").await {
        return Some(AgentKind::SimStudio);
    }
    None
}

/// Dispatch a message to an external coding agent.
///
/// Spawns the agent process, passes the user message as CLI args,
/// and streams stdout line-by-line as IPC chunks. Returns the full
/// concatenated response.
pub async fn dispatch(
    agent: &AgentKind,
    message: &str,
    resp_tx: &mpsc::Sender<OrchestratorMessage>,
) -> anyhow::Result<String> {
    let binary = agent.binary();
    let args = agent.args(message);

    tracing::info!("Dispatching to {} with {} arg(s)", binary, args.len());

    let mut child = tokio::process::Command::new(binary)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("failed to capture {} stdout", binary))?;

    let mut reader = BufReader::new(stdout).lines();
    let mut full_response = String::new();

    while let Some(line) = reader.next_line().await? {
        if !full_response.is_empty() {
            full_response.push('\n');
        }
        full_response.push_str(&line);
        let _ = resp_tx
            .send(OrchestratorMessage::Chunk {
                content: format!("{}\n", line),
            })
            .await;
    }

    let status = child.wait().await?;
    if !status.success() {
        let stderr = child.stderr.take();
        let mut err_msg = String::new();
        if let Some(se) = stderr {
            let mut err_reader = BufReader::new(se).lines();
            while let Some(line) = err_reader.next_line().await? {
                err_msg.push_str(&line);
                err_msg.push('\n');
            }
        }
        anyhow::bail!("{} exited with {}: {}", binary, status, err_msg.trim());
    }

    Ok(full_response)
}
