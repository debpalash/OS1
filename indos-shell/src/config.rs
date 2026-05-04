//! Shell configuration

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfig {
    /// Current shell mode
    pub mode: ShellMode,

    /// Orchestrator socket path
    pub orchestrator_socket: PathBuf,

    /// Default model for conversation
    pub default_model: String,

    /// Enable ambient mode on idle
    pub ambient_enabled: bool,

    /// Fragment rendering backend
    pub fragment_backend: FragmentBackend,

    /// Theme
    pub theme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShellMode {
    Conversation,
    Ambient,
    Focus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FragmentBackend {
    WebView,
    Native,
    Hybrid,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            mode: ShellMode::Conversation,
            orchestrator_socket: PathBuf::from("/run/indos/orchestrator.sock"),
            default_model: "llama3.2".into(),
            ambient_enabled: true,
            fragment_backend: FragmentBackend::Hybrid,
            theme: "dark".into(),
        }
    }
}

impl ShellConfig {
    /// Load config from XDG config directory, falling back to defaults
    pub fn load() -> Result<Self> {
        let config_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("indos")
            .join("shell.toml");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: ShellConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = Self::default();
            // Create config directory and write defaults
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent).ok();
            }
            let content = toml::to_string_pretty(&config)?;
            std::fs::write(&config_path, content).ok();
            Ok(config)
        }
    }
}
