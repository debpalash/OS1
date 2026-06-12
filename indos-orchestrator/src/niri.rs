use serde_json::Value;
use std::process::Command;

/// Client to interact with Niri compositor via `niri msg` CLI
pub struct NiriClient;

impl NiriClient {
    /// List all outputs (monitors)
    pub fn list_outputs() -> Result<Value, String> {
        Self::run_niri_cmd(&["outputs"])
    }

    /// List all workspaces
    pub fn list_workspaces() -> Result<Value, String> {
        Self::run_niri_cmd(&["workspaces"])
    }

    /// Focus a specific window by ID
    pub fn focus_window(id: u64) -> Result<Value, String> {
        Self::run_niri_cmd(&["action", "focus-window", "--id", &id.to_string()])
    }

    /// Move a window to a specific output
    pub fn move_window_to_output(window_id: u64, output_name: &str) -> Result<Value, String> {
        // First focus the window
        Self::focus_window(window_id)?;
        // Then move it to the output
        Self::run_niri_cmd(&["action", "move-window-to-monitor", output_name])
    }

    fn run_niri_cmd(args: &[&str]) -> Result<Value, String> {
        let mut cmd = Command::new("niri");
        cmd.arg("msg").arg("-j");
        for arg in args {
            cmd.arg(arg);
        }

        let output = cmd.output().map_err(|e| format!("Failed to execute niri msg: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("niri msg failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            return Ok(serde_json::json!({ "status": "success" }));
        }

        serde_json::from_str(&stdout).map_err(|e| format!("Failed to parse niri output: {}", e))
    }
}
