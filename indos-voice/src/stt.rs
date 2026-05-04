//! STT — Speech-to-Text module (Faster-Whisper integration)

pub struct SttEngine {
    model: String,
    ready: bool,
}

impl SttEngine {
    pub fn new(model: &str) -> Self {
        Self {
            model: model.to_string(),
            ready: false,
        }
    }

    /// Initialize Faster-Whisper subprocess
    pub async fn init(&mut self) -> anyhow::Result<()> {
        tracing::info!("Initializing STT engine (Faster-Whisper, model: {})", self.model);
        // TODO: Check if faster-whisper is installed
        // TODO: Download model if needed
        // TODO: Start subprocess
        self.ready = true;
        Ok(())
    }

    /// Transcribe audio samples to text
    pub async fn transcribe(&self, _audio: &[f32]) -> anyhow::Result<String> {
        // TODO: Send audio to Faster-Whisper
        // TODO: Return transcript
        Ok(String::new())
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }
}
