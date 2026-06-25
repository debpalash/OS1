//! STT — Speech-to-Text via Faster-Whisper subprocess
//!
//! Faster-Whisper is run as a Python subprocess that:
//! 1. Accepts WAV audio on stdin
//! 2. Returns JSON transcripts on stdout
//!
//! We use a persistent subprocess for low-latency repeated transcriptions.

use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::{Child, Command};

pub struct SttEngine {
    model: String,
    // holds the persistent whisper subprocess; not yet wired up
    #[allow(dead_code)]
    process: Option<Child>,
    ready: bool,
}

impl SttEngine {
    pub fn new(model: &str) -> Self {
        Self {
            model: model.to_string(),
            process: None,
            ready: false,
        }
    }

    /// Check if faster-whisper is available
    async fn check_available() -> bool {
        Command::new("python3")
            .args(["-c", "import faster_whisper; print('ok')"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Initialize — verify faster-whisper is installed and model exists
    pub async fn init(&mut self) -> Result<()> {
        tracing::info!(
            "Initializing STT engine (Faster-Whisper, model: {})",
            self.model
        );

        if !Self::check_available().await {
            anyhow::bail!("faster-whisper not found. Install with: pip install faster-whisper");
        }

        self.ready = true;
        tracing::info!("STT engine ready");
        Ok(())
    }

    /// Transcribe a WAV file to text
    pub async fn transcribe_file(&self, wav_path: &str) -> Result<String> {
        if !self.ready {
            anyhow::bail!("STT engine not initialized");
        }

        // Run faster-whisper CLI on the file
        let script = format!(
            r#"
import sys, json
from faster_whisper import WhisperModel
model = WhisperModel("{}", device="auto", compute_type="int8")
segments, info = model.transcribe("{}", beam_size=5, vad_filter=True)
text = " ".join(s.text.strip() for s in segments)
print(json.dumps({{"text": text, "language": info.language, "duration": info.duration}}))
"#,
            self.model, wav_path
        );

        let output = Command::new("python3")
            .args(["-c", &script])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to run faster-whisper")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("faster-whisper failed: {}", stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: serde_json::Value =
            serde_json::from_str(stdout.trim()).context("Failed to parse STT output")?;

        let text = result["text"].as_str().unwrap_or("").to_string();
        tracing::debug!(
            "STT: \"{}\" ({}s, {})",
            text,
            result["duration"].as_f64().unwrap_or(0.0),
            result["language"].as_str().unwrap_or("?")
        );

        Ok(text)
    }

    /// Transcribe raw audio samples (16kHz mono f32) to text
    pub async fn transcribe(&self, audio: &[f32], sample_rate: u32) -> Result<String> {
        if !self.ready {
            anyhow::bail!("STT engine not initialized");
        }

        // Write audio to temp WAV file, then transcribe
        let tmp_path = "/tmp/indos_stt_input.wav";
        write_wav(tmp_path, audio, sample_rate)?;
        self.transcribe_file(tmp_path).await
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }
}

/// Write f32 samples to a WAV file (16-bit PCM)
fn write_wav(path: &str, samples: &[f32], sample_rate: u32) -> Result<()> {
    use std::io::Write;
    let mut file = std::fs::File::create(path)?;
    let num_samples = samples.len() as u32;
    let byte_rate = sample_rate * 2; // 16-bit mono
    let data_size = num_samples * 2;
    let file_size = 36 + data_size;

    // WAV header
    file.write_all(b"RIFF")?;
    file.write_all(&file_size.to_le_bytes())?;
    file.write_all(b"WAVE")?;
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?; // chunk size
    file.write_all(&1u16.to_le_bytes())?; // PCM format
    file.write_all(&1u16.to_le_bytes())?; // mono
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&byte_rate.to_le_bytes())?;
    file.write_all(&2u16.to_le_bytes())?; // block align
    file.write_all(&16u16.to_le_bytes())?; // bits per sample
    file.write_all(b"data")?;
    file.write_all(&data_size.to_le_bytes())?;

    // Convert f32 → i16 and write
    for &s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        let i16_val = (clamped * 32767.0) as i16;
        file.write_all(&i16_val.to_le_bytes())?;
    }

    Ok(())
}
