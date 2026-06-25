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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sensible() {
        let cfg = VoiceConfig::default();
        assert_eq!(cfg.stt_engine, "faster-whisper");
        assert_eq!(cfg.stt_model, "base");
        assert_eq!(cfg.tts_engine, "piper");
        assert_eq!(cfg.tts_voice, "en_US-lessac-medium");
        assert_eq!(cfg.tts_rate, 1.0);
        assert!(cfg.vad_enabled);
        assert_eq!(cfg.vad_threshold, 0.5);
        assert!(cfg.input_device.is_none());
        assert!(cfg.output_device.is_none());
        assert_eq!(cfg.wake_word.as_deref(), Some("hey os"));
        assert!(cfg.hands_free);
        assert_eq!(cfg.conversation_window_secs, 45);
        assert!(cfg.briefing);
    }

    #[test]
    fn partial_toml_fills_missing_with_defaults() {
        // Only two fields set; the rest must fall back to defaults via
        // #[serde(default)] on the struct.
        let toml_src = r#"
            stt_model = "large-v3"
            hands_free = false
        "#;
        let cfg: VoiceConfig = toml::from_str(toml_src).unwrap();
        assert_eq!(cfg.stt_model, "large-v3");
        assert!(!cfg.hands_free);
        // Untouched fields keep defaults.
        assert_eq!(cfg.tts_voice, "en_US-lessac-medium");
        assert_eq!(cfg.conversation_window_secs, 45);
        assert_eq!(cfg.wake_word.as_deref(), Some("hey os"));
    }

    #[test]
    fn empty_toml_equals_defaults() {
        let cfg: VoiceConfig = toml::from_str("").unwrap();
        let def = VoiceConfig::default();
        assert_eq!(cfg.stt_engine, def.stt_engine);
        assert_eq!(cfg.tts_rate, def.tts_rate);
        assert_eq!(cfg.vad_threshold, def.vad_threshold);
        assert_eq!(cfg.briefing, def.briefing);
    }

    #[test]
    fn toml_round_trip_preserves_overrides() {
        let cfg = VoiceConfig {
            wake_word: Some("computer".into()),
            tts_rate: 1.5,
            input_device: Some("mic0".into()),
            ..VoiceConfig::default()
        };
        let text = toml::to_string(&cfg).unwrap();
        let back: VoiceConfig = toml::from_str(&text).unwrap();
        assert_eq!(back.wake_word.as_deref(), Some("computer"));
        assert_eq!(back.tts_rate, 1.5);
        assert_eq!(back.input_device.as_deref(), Some("mic0"));
    }
}
