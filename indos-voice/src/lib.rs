//! IndOS Voice — Local Voice I/O Pipeline
//!
//! Provides fully-offline voice conversation capabilities:
//! - STT: Faster-Whisper (GPU-accelerated, ~100ms latency)
//! - TTS: Piper (CPU-friendly, <50ms latency)
//! - Audio: PipeWire integration via cpal
//!
//! Architecture:
//! ```text
//! Microphone → PipeWire → Faster-Whisper (STT)
//!     → Transcribed text → Orchestrator → LLM
//!     → Response text → Piper (TTS) → Speaker
//! ```

mod config;
mod stt;
mod tts;
mod pipeline;

use anyhow::Result;

/// Voice pipeline state
pub struct VoicePipeline {
    config: config::VoiceConfig,
    active: bool,
}

impl VoicePipeline {
    pub fn new(config: config::VoiceConfig) -> Self {
        Self {
            config,
            active: false,
        }
    }

    /// Start listening for voice input
    pub async fn start(&mut self) -> Result<()> {
        tracing::info!("Voice pipeline starting...");
        tracing::info!("STT engine: {}", self.config.stt_engine);
        tracing::info!("TTS engine: {}", self.config.tts_engine);
        self.active = true;
        // TODO: Initialize audio capture
        // TODO: Start STT subprocess
        // TODO: Connect to orchestrator
        Ok(())
    }

    /// Process a voice utterance and return audio response
    pub async fn process_utterance(&self, _audio: &[f32]) -> Result<Vec<f32>> {
        // TODO: Send audio to Faster-Whisper
        // TODO: Get transcript
        // TODO: Send to orchestrator
        // TODO: Get response text
        // TODO: Send to Piper TTS
        // TODO: Return audio samples
        Ok(vec![])
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}
