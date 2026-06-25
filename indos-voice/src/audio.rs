//! Audio capture and playback via PipeWire subprocesses.
//!
//! Capture: `pw-record` streams raw s16le mono 16 kHz to stdout; we run a
//! simple energy (RMS) voice-activity detector over 30 ms chunks and return
//! one utterance at a time. Playback: `pw-play` on a WAV file.
//!
//! Subprocesses keep us off the audio-API treadmill: PipeWire ships on the
//! ISO, the tools handle routing/resampling, and a crash can't take the
//! daemon down.

use anyhow::{Context, Result};
use std::process::Stdio;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

pub const SAMPLE_RATE: u32 = 16_000;
const CHUNK_SAMPLES: usize = 480; // 30 ms at 16 kHz
const PRE_ROLL_CHUNKS: usize = 10; // 300 ms kept from before speech onset
const ONSET_CHUNKS: usize = 3; // 90 ms of speech to trigger
const SILENCE_END_CHUNKS: usize = 35; // ~1.05 s of silence ends the utterance
const MAX_UTTERANCE_SECS: usize = 15;

/// Map the 0.0..=1.0 config threshold onto an RMS floor. Speech over a
/// laptop mic is typically RMS 0.02–0.2; 0.5 → 0.02.
pub fn threshold_rms(vad_threshold: f32) -> f32 {
    0.005 + vad_threshold.clamp(0.0, 1.0) * 0.03
}

fn rms(chunk: &[f32]) -> f32 {
    if chunk.is_empty() {
        return 0.0;
    }
    (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32).sqrt()
}

/// Block until one utterance is captured, then return its samples.
///
/// Returns Ok(None) if the mic stream ends or produces no speech within
/// `max_wait_secs` — callers treat that as "nothing said" and loop.
pub async fn capture_utterance(
    vad_threshold: f32,
    input_device: Option<&str>,
    max_wait_secs: u64,
) -> Result<Option<Vec<f32>>> {
    let mut cmd = Command::new("pw-record");
    cmd.args([
        "--rate",
        &SAMPLE_RATE.to_string(),
        "--channels",
        "1",
        "--format",
        "s16",
        "-",
    ]);
    if let Some(dev) = input_device {
        cmd.args(["--target", dev]);
    }
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .context("failed to start pw-record (is PipeWire running?)")?;

    let mut stdout = child.stdout.take().context("pw-record has no stdout")?;
    let floor = threshold_rms(vad_threshold);
    let max_chunks = MAX_UTTERANCE_SECS * SAMPLE_RATE as usize / CHUNK_SAMPLES;
    let wait_deadline = std::time::Instant::now() + std::time::Duration::from_secs(max_wait_secs);

    let mut pre_roll: Vec<Vec<f32>> = Vec::with_capacity(PRE_ROLL_CHUNKS);
    let mut utterance: Vec<f32> = Vec::new();
    let mut speech_run = 0usize;
    let mut silence_run = 0usize;
    let mut in_speech = false;
    let mut byte_buf = vec![0u8; CHUNK_SAMPLES * 2];

    loop {
        if let Err(e) = stdout.read_exact(&mut byte_buf).await {
            tracing::debug!("mic stream ended: {e}");
            return Ok(None);
        }
        let chunk: Vec<f32> = byte_buf
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0)
            .collect();
        let level = rms(&chunk);

        if !in_speech {
            if std::time::Instant::now() > wait_deadline {
                return Ok(None);
            }
            pre_roll.push(chunk);
            if pre_roll.len() > PRE_ROLL_CHUNKS {
                pre_roll.remove(0);
            }
            if level > floor {
                speech_run += 1;
                if speech_run >= ONSET_CHUNKS {
                    in_speech = true;
                    silence_run = 0;
                    for c in pre_roll.drain(..) {
                        utterance.extend(c);
                    }
                }
            } else {
                speech_run = 0;
            }
        } else {
            utterance.extend(&chunk);
            if level > floor {
                silence_run = 0;
            } else {
                silence_run += 1;
                if silence_run >= SILENCE_END_CHUNKS {
                    break;
                }
            }
            if utterance.len() / CHUNK_SAMPLES >= max_chunks {
                tracing::debug!("utterance hit {MAX_UTTERANCE_SECS}s cap");
                break;
            }
        }
    }

    let _ = child.kill().await;
    Ok(Some(utterance))
}

/// Play a WAV file through the default output.
pub async fn play_wav(path: &str, output_device: Option<&str>) -> Result<()> {
    let mut cmd = Command::new("pw-play");
    if let Some(dev) = output_device {
        cmd.args(["--target", dev]);
    }
    let status = cmd
        .arg(path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .context("failed to start pw-play")?;
    if !status.success() {
        anyhow::bail!("pw-play exited with {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_rms_endpoints() {
        // 0.0 → bare floor; 1.0 → floor + full span.
        assert!((threshold_rms(0.0) - 0.005).abs() < f32::EPSILON);
        assert!((threshold_rms(1.0) - 0.035).abs() < 1e-6);
        // The documented midpoint: 0.5 → 0.02.
        assert!((threshold_rms(0.5) - 0.02).abs() < 1e-6);
    }

    #[test]
    fn threshold_rms_clamps_out_of_range() {
        // Values outside 0.0..=1.0 are clamped, never extrapolated.
        assert_eq!(threshold_rms(-5.0), threshold_rms(0.0));
        assert_eq!(threshold_rms(5.0), threshold_rms(1.0));
    }

    #[test]
    fn threshold_rms_is_monotonic() {
        assert!(threshold_rms(0.1) < threshold_rms(0.9));
    }

    #[test]
    fn rms_of_silence_is_zero() {
        assert_eq!(rms(&[]), 0.0);
        assert_eq!(rms(&[0.0, 0.0, 0.0]), 0.0);
    }

    #[test]
    fn rms_of_constant_signal_equals_amplitude() {
        // RMS of a constant ±a square wave is |a|.
        let signal = [0.5, -0.5, 0.5, -0.5];
        assert!((rms(&signal) - 0.5).abs() < 1e-6);
    }
}
