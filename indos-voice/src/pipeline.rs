//! Pipeline — connects Microphone → STT → Orchestrator → TTS → Speaker
//!
//! The pipeline runs as a loop:
//! 1. Listen for voice activity (VAD)
//! 2. Record until silence
//! 3. Transcribe via STT
//! 4. Send to orchestrator via Unix socket
//! 5. Get response
//! 6. Synthesize via TTS
//! 7. Play audio

use anyhow::Result;
use crate::stt::SttEngine;
use crate::tts::TtsEngine;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

/// Voice pipeline state machine
#[derive(Debug, Clone, PartialEq)]
pub enum PipelineState {
    Idle,
    Listening,
    Transcribing,
    WaitingForResponse,
    Speaking,
}

pub struct Pipeline {
    pub stt: SttEngine,
    pub tts: TtsEngine,
    pub state: PipelineState,
    socket_path: PathBuf,
}

impl Pipeline {
    pub fn new(stt: SttEngine, tts: TtsEngine) -> Self {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .unwrap_or_else(|_| format!("/run/user/{}", unsafe { libc::getuid() }));

        Self {
            stt,
            tts,
            state: PipelineState::Idle,
            socket_path: PathBuf::from(runtime_dir).join("indos/orchestrator.sock"),
        }
    }

    /// Initialize both engines
    pub async fn init(&mut self) -> Result<()> {
        self.stt.init().await?;
        self.tts.init().await?;
        Ok(())
    }

    /// Process a single voice turn: audio → text → orchestrator → text → audio
    pub async fn process_turn(&mut self, audio: &[f32], sample_rate: u32) -> Result<Vec<f32>> {
        // 1. Transcribe
        self.state = PipelineState::Transcribing;
        let transcript = self.stt.transcribe(audio, sample_rate).await?;

        if transcript.trim().is_empty() {
            tracing::debug!("Empty transcript, skipping");
            self.state = PipelineState::Idle;
            return Ok(vec![]);
        }

        tracing::info!("Voice: \"{}\"", transcript);

        // 2. Send to orchestrator
        self.state = PipelineState::WaitingForResponse;
        let response = self.send_to_orchestrator(&transcript).await?;

        if response.is_empty() {
            self.state = PipelineState::Idle;
            return Ok(vec![]);
        }

        tracing::info!("Response: \"{}\"", &response[..response.len().min(100)]);

        // 3. Synthesize response
        self.state = PipelineState::Speaking;
        let audio_out = self.tts.synthesize(&response).await?;

        self.state = PipelineState::Idle;
        Ok(audio_out)
    }

    /// Send text to orchestrator and get full response
    pub async fn send_to_orchestrator(&self, text: &str) -> Result<String> {
        let stream = UnixStream::connect(&self.socket_path).await?;
        let (reader, mut writer) = stream.into_split();

        // Send chat message
        let msg = serde_json::json!({
            "type": "chat",
            "content": text,
            "session_id": null
        });
        let mut line = serde_json::to_string(&msg)?;
        line.push('\n');
        writer.write_all(line.as_bytes()).await?;

        // Read response chunks until Done
        let mut buf_reader = BufReader::new(reader);
        let mut full_response = String::new();
        let mut line_buf = String::new();

        loop {
            line_buf.clear();
            let n = buf_reader.read_line(&mut line_buf).await?;
            if n == 0 {
                break; // Connection closed
            }

            if let Ok(resp) = serde_json::from_str::<serde_json::Value>(line_buf.trim()) {
                match resp["type"].as_str() {
                    Some("chunk") => {
                        if let Some(content) = resp["content"].as_str() {
                            full_response.push_str(content);
                        }
                    }
                    Some("done") => {
                        if let Some(fr) = resp["full_response"].as_str() {
                            if full_response.is_empty() {
                                full_response = fr.to_string();
                            }
                        }
                        break;
                    }
                    Some("error") => {
                        let msg = resp["message"].as_str().unwrap_or("Unknown error");
                        anyhow::bail!("Orchestrator error: {}", msg);
                    }
                    _ => {}
                }
            }
        }

        Ok(full_response)
    }
}
