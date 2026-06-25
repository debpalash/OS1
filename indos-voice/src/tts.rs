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

    /// Find the piper TTS binary. `--help` must mention "model" — the
    /// Arch `extra/piper` package is an unrelated gaming-mouse GUI, so a
    /// bare existence check would latch onto the wrong tool.
    async fn find_piper() -> Option<String> {
        for path in &["piper", "/usr/bin/piper", "/usr/local/bin/piper"] {
            if let Ok(out) = Command::new(path)
                .arg("--help")
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output()
                .await
            {
                if out.status.success() && String::from_utf8_lossy(&out.stdout).contains("--model")
                {
                    return Some(path.to_string());
                }
            }
        }
        None
    }

    /// Resolve a voice name like "en_US-lessac-medium" to its .onnx path.
    /// Accepts an explicit path unchanged; otherwise searches the user's
    /// data dir then the system voices baked into the ISO.
    fn resolve_voice(voice: &str) -> Option<std::path::PathBuf> {
        let p = std::path::Path::new(voice);
        if p.extension().map(|e| e == "onnx").unwrap_or(false) && p.exists() {
            return Some(p.to_path_buf());
        }
        let mut dirs_to_try = Vec::new();
        if let Some(d) = dirs::data_dir() {
            dirs_to_try.push(d.join("piper").join("voices"));
        }
        dirs_to_try.push(std::path::PathBuf::from("/usr/share/piper/voices"));
        for dir in dirs_to_try {
            let candidate = dir.join(format!("{voice}.onnx"));
            if candidate.exists() {
                return Some(candidate);
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

        match Self::resolve_voice(&self.voice) {
            Some(path) => tracing::info!("Voice model: {}", path.display()),
            None => tracing::warn!(
                "Voice model '{}' not found in ~/.local/share/piper/voices \
                 or /usr/share/piper/voices — synthesis will fail until it \
                 is downloaded (python3 -m piper.download_voices {})",
                self.voice,
                self.voice
            ),
        }

        self.ready = true;
        tracing::info!("TTS engine ready");
        Ok(())
    }

    /// Synthesize text to a WAV file
    pub async fn synthesize_to_file(&self, text: &str, output_path: &str) -> Result<()> {
        let piper = self
            .piper_path
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Piper not installed"))?;
        let model = Self::resolve_voice(&self.voice)
            .ok_or_else(|| anyhow::anyhow!("voice model '{}' not found", self.voice))?;

        let output = Command::new(piper)
            .args([
                "--model",
                &model.to_string_lossy(),
                "--output_file",
                output_path,
                "--length_scale",
                &format!("{:.2}", 1.0 / self.rate),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal 44-byte WAV header with the given bit depth, followed
    /// by `data` bytes for the data chunk.
    fn wav_with(bits_per_sample: u16, data: &[u8]) -> Vec<u8> {
        let mut buf = vec![0u8; 44];
        buf[34] = (bits_per_sample & 0xff) as u8;
        buf[35] = (bits_per_sample >> 8) as u8;
        buf.extend_from_slice(data);
        buf
    }

    #[test]
    fn parse_wav_rejects_too_small() {
        let err = parse_wav_to_f32(&[0u8; 10]).unwrap_err();
        assert!(err.to_string().contains("too small"));
    }

    #[test]
    fn parse_wav_rejects_unsupported_bit_depth() {
        let bytes = wav_with(24, &[]);
        let err = parse_wav_to_f32(&bytes).unwrap_err();
        assert!(err.to_string().contains("bit depth"));
    }

    #[test]
    fn parse_wav_decodes_16bit_samples() {
        // Two i16 samples: 0 and i16::MAX, little-endian.
        let mut data = Vec::new();
        data.extend_from_slice(&0i16.to_le_bytes());
        data.extend_from_slice(&i16::MAX.to_le_bytes());
        let bytes = wav_with(16, &data);
        let samples = parse_wav_to_f32(&bytes).unwrap();
        assert_eq!(samples.len(), 2);
        assert!((samples[0] - 0.0).abs() < f32::EPSILON);
        // i16::MAX / 32768 ≈ 0.99997.
        assert!((samples[1] - (i16::MAX as f32 / 32768.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn parse_wav_empty_data_yields_no_samples() {
        let bytes = wav_with(16, &[]);
        let samples = parse_wav_to_f32(&bytes).unwrap();
        assert!(samples.is_empty());
    }
}
