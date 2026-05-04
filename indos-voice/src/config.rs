//! Voice pipeline configuration

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Wake word (e.g., "hey indos"). None = push-to-talk only.
    pub wake_word: Option<String>,
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
            wake_word: None,
        }
    }
}
