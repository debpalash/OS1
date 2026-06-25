//! indos-voiced — the OS 1 voice daemon.
//!
//! Hands-free loop: listen (VAD) → transcribe (Faster-Whisper) → gate on the
//! wake phrase → orchestrator → speak the reply (Piper). Also delivers a
//! short spoken briefing at the first login of the day.
//!
//! Designed to never crash-loop on a half-provisioned system: if STT/TTS or
//! the mic aren't available yet, it idles with status "off" and retries.
//!
//! Signals: SIGUSR1 opens a "hot" window — the next utterance is answered
//! without the wake phrase (bound to Mod+V in niri).

use anyhow::Result;
use indos_voice::audio;
use indos_voice::config::VoiceConfig;
use indos_voice::{Pipeline, SttEngine, TtsEngine};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Where waybar's voice-status module reads our state from.
fn status_path() -> PathBuf {
    let runtime = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/run/user/{}", unsafe { libc::getuid() }));
    PathBuf::from(runtime).join("indos").join("voice.state")
}

fn set_status(path: &PathBuf, state: &str) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, state);
}

/// Lowercase, strip punctuation, collapse whitespace — so "Hey, OS!" and
/// "hey os" compare equal.
fn normalize(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Make LLM output speakable: drop code blocks and markdown markup, end at
/// a sentence boundary within a sane length for TTS.
fn speakable(text: &str) -> String {
    let mut out = String::new();
    let mut in_code = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        out.push_str(line);
        out.push(' ');
    }
    let cleaned: String = out
        .replace(['*', '`', '#', '|'], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if cleaned.len() <= 600 {
        return cleaned;
    }
    // cut at the last sentence end before 600 chars
    let cut = cleaned[..600]
        .rfind(['.', '!', '?'])
        .map(|i| i + 1)
        .unwrap_or(600);
    cleaned[..cut].to_string()
}

/// Local facts for the briefing prompt — gathered without LLM tool calls so
/// even the smallest routed model produces a grounded greeting.
fn briefing_facts() -> String {
    let mut facts = Vec::new();
    let out = std::process::Command::new("date")
        .arg("+%A, %B %d, %H:%M")
        .output();
    if let Ok(o) = out {
        facts.push(format!(
            "now: {}",
            String::from_utf8_lossy(&o.stdout).trim()
        ));
    }
    if let Ok(o) = std::process::Command::new("sh")
        .args(["-c", "df --output=pcent / | tail -1"])
        .output()
    {
        facts.push(format!(
            "root disk {} full",
            String::from_utf8_lossy(&o.stdout).trim()
        ));
    }
    if let Ok(up) = std::fs::read_to_string("/proc/uptime") {
        if let Some(secs) = up.split('.').next().and_then(|s| s.parse::<u64>().ok()) {
            facts.push(format!("system up {} min", secs / 60));
        }
    }
    facts.join("; ")
}

/// One spoken briefing per calendar day, tracked in a state file.
async fn maybe_briefing(pipeline: &Pipeline, cfg: &VoiceConfig, status: &PathBuf) {
    if !cfg.briefing {
        return;
    }
    let state_dir = dirs::state_dir()
        .or_else(dirs::data_dir)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("indos");
    let _ = std::fs::create_dir_all(&state_dir);
    let marker = state_dir.join("last_briefing");

    let today = std::process::Command::new("date")
        .arg("+%F")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    if today.is_empty()
        || std::fs::read_to_string(&marker)
            .map(|d| d.trim() == today)
            .unwrap_or(false)
    {
        return;
    }

    let prompt = format!(
        "[proactive briefing] Facts: {}. Greet me warmly by time of day and \
         give a one-or-two sentence start-of-session briefing from these \
         facts. Under 40 words. Plain prose, no markdown, no questions.",
        briefing_facts()
    );
    tracing::info!("Speaking daily briefing");
    match pipeline.send_to_orchestrator(&prompt).await {
        Ok(reply) if !reply.trim().is_empty() => {
            let _ = std::fs::write(&marker, &today);
            speak(pipeline, &speakable(&reply), status).await;
        }
        Ok(_) => tracing::warn!("Briefing: empty orchestrator reply"),
        Err(e) => tracing::warn!("Briefing skipped: {e}"),
    }
}

async fn speak(pipeline: &Pipeline, text: &str, status: &PathBuf) {
    if text.trim().is_empty() {
        return;
    }
    set_status(status, "speaking");
    let wav = "/tmp/indos_voiced_reply.wav";
    match pipeline.tts.synthesize_to_file(text, wav).await {
        Ok(()) => {
            if let Err(e) = audio::play_wav(wav, None).await {
                tracing::warn!("playback failed: {e}");
            }
        }
        Err(e) => tracing::warn!("TTS failed: {e}"),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cfg = VoiceConfig::load();
    let status = status_path();
    set_status(&status, "off");
    tracing::info!(
        "indos-voiced starting (hands_free={}, wake_word={:?}, briefing={})",
        cfg.hands_free,
        cfg.wake_word,
        cfg.briefing
    );

    // SIGUSR1 → hot window: next utterance bypasses the wake phrase.
    let hot_until = Arc::new(AtomicU64::new(0));
    {
        let hot = hot_until.clone();
        let window = cfg.conversation_window_secs.max(10);
        tokio::spawn(async move {
            let mut sig =
                match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::user_defined1())
                {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!("SIGUSR1 handler unavailable: {e}");
                        return;
                    }
                };
            while sig.recv().await.is_some() {
                tracing::info!("SIGUSR1: voice hot window opened");
                hot.store(now_secs() + window, Ordering::Relaxed);
            }
        });
    }

    // Engine init with graceful retry — the live ISO may not have
    // faster-whisper/piper until first-boot provisioning finishes.
    let pipeline = loop {
        let stt = SttEngine::new(&cfg.stt_model);
        let tts = TtsEngine::new(&cfg.tts_voice, cfg.tts_rate);
        let mut p = Pipeline::new(stt, tts);
        match p.init().await {
            Ok(()) => break p,
            Err(e) => {
                tracing::warn!("voice engines not ready ({e}); retrying in 60s");
                set_status(&status, "off");
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        }
    };
    tracing::info!("Voice engines ready");

    maybe_briefing(&pipeline, &cfg, &status).await;

    let wake = cfg.wake_word.as_deref().map(normalize);
    let mut last_exchange: u64 = 0;

    loop {
        if !cfg.hands_free && now_secs() >= hot_until.load(Ordering::Relaxed) {
            // Triggered-only mode: idle until SIGUSR1 opens the window.
            set_status(&status, "off");
            tokio::time::sleep(Duration::from_millis(500)).await;
            continue;
        }

        set_status(&status, "listening");
        let captured =
            match audio::capture_utterance(cfg.vad_threshold, cfg.input_device.as_deref(), 600)
                .await
            {
                Ok(Some(samples)) if !samples.is_empty() => samples,
                Ok(_) => continue,
                Err(e) => {
                    tracing::warn!("mic capture failed ({e}); retrying in 30s");
                    set_status(&status, "off");
                    tokio::time::sleep(Duration::from_secs(30)).await;
                    continue;
                }
            };

        set_status(&status, "processing");
        let transcript = match pipeline.stt.transcribe(&captured, audio::SAMPLE_RATE).await {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("STT failed: {e}");
                continue;
            }
        };
        let norm = normalize(&transcript);
        if norm.is_empty() {
            continue;
        }
        tracing::info!("Heard: \"{transcript}\"");

        // Wake gating: inside the conversation/hot window everything goes
        // through; otherwise the wake phrase must appear, and we strip it.
        let now = now_secs();
        let in_window = now < last_exchange + cfg.conversation_window_secs
            || now < hot_until.load(Ordering::Relaxed);
        let content = match (&wake, in_window) {
            (Some(w), false) => {
                if let Some(pos) = norm.find(w.as_str()) {
                    let after = norm[pos + w.len()..].trim().to_string();
                    if after.is_empty() {
                        speak(&pipeline, "Yes?", &status).await;
                        last_exchange = now_secs();
                        continue;
                    }
                    after
                } else {
                    tracing::debug!("no wake phrase, ignoring");
                    continue;
                }
            }
            _ => transcript.clone(),
        };

        match pipeline.send_to_orchestrator(&content).await {
            Ok(reply) => {
                last_exchange = now_secs();
                speak(&pipeline, &speakable(&reply), &status).await;
            }
            Err(e) => {
                tracing::warn!("orchestrator request failed: {e}");
                speak(
                    &pipeline,
                    "Sorry, I hit a snag talking to the brain. One sec.",
                    &status,
                )
                .await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize, speakable};

    #[test]
    fn normalize_lowercases_and_strips_punctuation() {
        assert_eq!(normalize("Hey, OS!"), "hey os");
        assert_eq!(normalize("hey os"), "hey os");
    }

    #[test]
    fn normalize_collapses_whitespace() {
        assert_eq!(
            normalize("  multiple   spaces\ttab\nline "),
            "multiple spaces tab line"
        );
        assert_eq!(normalize(""), "");
        assert_eq!(normalize("!!!"), "");
    }

    #[test]
    fn normalize_keeps_alphanumerics() {
        assert_eq!(normalize("Open file_2.txt"), "open file 2 txt");
    }

    #[test]
    fn speakable_strips_code_blocks_and_markdown() {
        let input = "Here is code:\n```rust\nfn main() {}\n```\nDone *now* `ok` #h |x";
        let out = speakable(input);
        assert!(!out.contains("fn main"));
        assert!(!out.contains('`'));
        assert!(!out.contains('*'));
        assert!(!out.contains('#'));
        assert!(!out.contains('|'));
        assert!(out.contains("Here is code"));
        assert!(out.contains("Done now ok"));
    }

    #[test]
    fn speakable_short_text_passes_through() {
        assert_eq!(speakable("Just a sentence."), "Just a sentence.");
    }

    #[test]
    fn speakable_truncates_long_text_at_sentence_boundary() {
        // First sentence ends well before 600 chars; second pushes over.
        let first = "A".repeat(400) + ".";
        let second = " ".to_string() + &"B".repeat(400) + ".";
        let input = format!("{first}{second}");
        let out = speakable(&input);
        assert!(out.len() <= 600);
        // Cut happens at the first sentence end, so no 'B' survives.
        assert!(!out.contains('B'));
        assert!(out.ends_with('.'));
    }
}
