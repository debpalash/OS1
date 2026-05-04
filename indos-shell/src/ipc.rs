//! IPC — Communication with the orchestrator

/// Connection to the IndOS orchestrator daemon
pub struct OrchestratorConnection {
    socket_path: String,
    connected: bool,
}

impl OrchestratorConnection {
    pub fn new(socket_path: &str) -> Self {
        Self {
            socket_path: socket_path.to_string(),
            connected: false,
        }
    }

    /// Connect to the orchestrator unix socket
    pub async fn connect(&mut self) -> anyhow::Result<()> {
        tracing::info!("Connecting to orchestrator at {}", self.socket_path);
        // TODO: Connect to unix socket
        self.connected = true;
        Ok(())
    }

    /// Send user input to the orchestrator
    pub async fn send_input(&self, _text: &str) -> anyhow::Result<()> {
        // TODO: Serialize and send via unix socket
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }
}
