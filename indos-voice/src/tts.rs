//! TTS — Text-to-Speech via Piper subprocess
//!
//! Piper is a fast, local neural TTS engine.
//! We call it as a subprocess: text on stdin → WAV on stdout.

use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::process::Command;

pub struct TtsEngine {
    /// Voice model name (e.g., "en_US-lessac-medium")
    voice: String,
    /// Speech rate multiplier
    rate: f32,
    /// Path to piper binary (auto-detected)
    piper_path: Option<String>,
    ready: bool,
}

impl TtsEngine {
    pub fn new(voice: &str, rate: f32) -> Self {
        Self {
            voice: voice.to_string(),
            rate,
            piper_path: None,
            ready: false,
        }
    }

    /// Find piper binary
    async fn find_piper() -> Option<String> {
        // Check common locations
        for path in &["piper", "/usr/bin/piper", "/usr/local/bin/piper"] {
            if Command::new(path)
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .await
                .map(|s| s.success())
                .unwrap_or(false)
            {
                return Some(path.to_string());
            }
        }
        None
    }

    /// Initialize — verify piper is installed
    pub async fn init(&mut self) -> Result<()> {
        tracing::info!("Initializing TTS engine (Piper, voice: {})", self.voice);

        match Self::find_piper().await {
            Some(path) => {
                tracing::info!("Found piper at: {}", path);
                self.piper_path = Some(path);
            }
            None => {
                tracing::warn!("Piper not found. Install with: pacman -S piper-tts");
                // Still mark as "ready" — we'll fail gracefully on synthesize
            }
        }

        // Check if voice model exists
        let model_dir = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("piper")
            .join("voices");

        if !model_dir.exists() {
            tracing::info!("Voice models dir not found, will download on first use");
        }

        self.ready = true;
        tracing::info!("TTS engine ready");
        Ok(())
    }

    /// Synthesize text to a WAV file
    pub async fn synthesize_to_file(&self, text: &str, output_path: &str) -> Result<()> {
        let piper = self.piper_path.as_deref()
            .ok_or_else(|| anyhow::anyhow!("Piper not installed"))?;

        let output = Command::new(piper)
            .args([
                "--model", &self.voice,
                "--output_file", output_path,
                "--length_scale", &format!("{:.2}", 1.0 / self.rate),
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to start piper")?;

        // Write text to piper's stdin
        let mut child = output;
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(text.as_bytes()).await?;
            stdin.shutdown().await?;
        }

        let result = child.wait_with_output().await?;
        if !result.status.success() {
            let stderr = String::from_utf8_lossy(&result.stderr);
            anyhow::bail!("Piper failed: {}", stderr);
        }

        tracing::debug!("TTS: synthesized {} chars → {}", text.len(), output_path);
        Ok(())
    }

    /// Synthesize text to raw audio samples (f32, 22050 Hz mono)
    pub async fn synthesize(&self, text: &str) -> Result<Vec<f32>> {
        let tmp_path = "/tmp/indos_tts_output.wav";
        self.synthesize_to_file(text, tmp_path).await?;

        // Read WAV file back as f32 samples
        let data = std::fs::read(tmp_path)?;
        let samples = parse_wav_to_f32(&data)?;
        Ok(samples)
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }
}

/// Parse WAV file bytes into f32 samples
fn parse_wav_to_f32(data: &[u8]) -> Result<Vec<f32>> {
    if data.len() < 44 {
        anyhow::bail!("WAV file too small");
    }

    // Skip to data chunk (byte 44 for standard WAV)
    let bits_per_sample = u16::from_le_bytes([data[34], data[35]]);
    let data_start = 44; // Standard WAV header size

    match bits_per_sample {
        16 => {
            let samples: Vec<f32> = data[data_start..]
                .chunks_exact(2)
                .map(|chunk| {
                    let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                    sample as f32 / 32768.0
                })
                .collect();
            Ok(samples)
        }
        _ => anyhow::bail!("Unsupported WAV bit depth: {}", bits_per_sample),
    }
}
