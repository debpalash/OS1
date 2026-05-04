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

pub mod config;
pub mod stt;
pub mod tts;
pub mod pipeline;

pub use pipeline::{Pipeline, PipelineState};
pub use stt::SttEngine;
pub use tts::TtsEngine;
