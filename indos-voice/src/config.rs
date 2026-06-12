//! Voice pipeline configuration
//!
//! Loaded from `~/.config/indos/voice.toml`; every field is optional and
//! falls back to the defaults below, so a partial file is fine.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VoiceConfig {
    /// STT engine: "faster-whisper" (default) or "whisper-cpp"
    pub stt_engine: String,

    /// STT model size: "tiny", "base", "small", "medium", "large-v3"
    pub stt_model: String,

    /// TTS engine: "piper" (default)
    pub tts_engine: String,

    /// TTS voice model
    pub tts_voice: String,

    /// TTS speech rate (0.5 = slow, 1.0 = normal, 2.0 = fast)
    pub tts_rate: f32,

    /// Enable voice activation detection (VAD)
    pub vad_enabled: bool,

    /// VAD sensitivity (0.0 = very sensitive, 1.0 = requires loud speech)
    pub vad_threshold: f32,

    /// Audio input device (None = system default)
    pub input_device: Option<String>,

    /// Audio output device (None = system default)
    pub output_device: Option<String>,

    /// Wake phrase checked against the transcript (e.g., "hey os").
    /// None = respond to every utterance while hands-free.
    pub wake_word: Option<String>,

    /// Hands-free mode: continuously listen and answer out loud.
    /// false = only listen when triggered (SIGUSR1 / Mod+V).
    pub hands_free: bool,

    /// After an exchange, keep responding without the wake phrase for
    /// this many seconds (a natural back-and-forth window).
    pub conversation_window_secs: u64,

    /// Speak a short proactive briefing at the first login of the day.
    pub briefing: bool,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            stt_engine: "faster-whisper".into(),
            stt_model: "base".into(),
            tts_engine: "piper".into(),
            tts_voice: "en_US-lessac-medium".into(),
            tts_rate: 1.0,
            vad_enabled: true,
            vad_threshold: 0.5,
            input_device: None,
            output_device: None,
            wake_word: Some("hey os".into()),
            hands_free: true,
            conversation_window_secs: 45,
            briefing: true,
        }
    }
}

impl VoiceConfig {
    /// Load from ~/.config/indos/voice.toml, defaults on any failure.
    pub fn load() -> Self {
        let path = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("indos")
            .join("voice.toml");
        match std::fs::read_to_string(&path) {
            Ok(text) => match toml::from_str(&text) {
                Ok(cfg) => {
                    tracing::info!("Loaded voice config from {}", path.display());
                    cfg
                }
                Err(e) => {
                    tracing::warn!("Bad voice.toml ({e}), using defaults");
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }
}
