//! Ollama client — streaming conversation with local LLM
//!
//! Uses the Ollama HTTP API (default: localhost:11434) for:
//! - Model listing and health checks
//! - Streaming chat completions
//! - Embedding generation (for context engine)

use anyhow::Result;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

/// Ollama API client
pub struct OllamaClient {
    base_url: String,
    client: reqwest::Client,
    model: String,
}

/// A chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Ollama chat request
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

/// Ollama streaming response chunk
#[derive(Debug, Deserialize)]
pub struct ChatChunk {
    pub message: Option<ChatMessage>,
    pub done: bool,
}

/// Ollama model list response
#[derive(Debug, Deserialize)]
struct ModelListResponse {
    models: Vec<ModelInfo>,
}

#[derive(Debug, Deserialize)]
struct ModelInfo {
    name: String,
}

impl OllamaClient {
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: reqwest::Client::new(),
            model: model.to_string(),
        }
    }

    /// Check if Ollama is running and the model is available
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/api/tags", self.base_url);
        match self.client.get(&url).send().await {
            Ok(resp) => {
                let list: ModelListResponse = resp.json().await?;
                let available = list.models.iter().any(|m| m.name.starts_with(&self.model));
                if !available {
                    tracing::warn!(
                        "Model '{}' not found. Available: {:?}",
                        self.model,
                        list.models.iter().map(|m| &m.name).collect::<Vec<_>>()
                    );
                }
                Ok(available)
            }
            Err(e) => {
                tracing::error!("Ollama not reachable at {}: {}", self.base_url, e);
                Ok(false)
            }
        }
    }

    /// List all locally available model names
    pub async fn list_models(&self) -> Vec<String> {
        let url = format!("{}/api/tags", self.base_url);
        match self.client.get(&url).send().await {
            Ok(resp) => match resp.json::<ModelListResponse>().await {
                Ok(list) => list.models.into_iter().map(|m| m.name).collect(),
                Err(_) => vec![],
            },
            Err(_) => vec![],
        }
    }

    /// Send a chat message and stream the response, calling `on_chunk` for each token
    // convenience wrapper over chat_stream_with_model, part of public API surface
    #[allow(dead_code)]
    pub async fn chat_stream<F>(&self, messages: &[ChatMessage], on_chunk: F) -> Result<String>
    where
        F: FnMut(&str),
    {
        self.chat_stream_with_model(&self.model, messages, on_chunk)
            .await
    }

    /// Like chat_stream but allows overriding the model per-request
    pub async fn chat_stream_with_model<F>(
        &self,
        model: &str,
        messages: &[ChatMessage],
        mut on_chunk: F,
    ) -> Result<String>
    where
        F: FnMut(&str),
    {
        let url = format!("{}/api/chat", self.base_url);
        let req = ChatRequest {
            model: model.to_string(),
            messages: messages.to_vec(),
            stream: true,
        };

        let resp = self.client.post(&url).json(&req).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Ollama error {}: {}", status, body);
        }

        let mut stream = resp.bytes_stream();
        let mut full_response = String::new();

        while let Some(chunk_result) = stream.next().await {
            let bytes = chunk_result?;
            let text = String::from_utf8_lossy(&bytes);

            // Ollama sends newline-delimited JSON
            for line in text.lines() {
                if line.is_empty() {
                    continue;
                }
                if let Ok(chunk) = serde_json::from_str::<ChatChunk>(line) {
                    if let Some(msg) = &chunk.message {
                        on_chunk(&msg.content);
                        full_response.push_str(&msg.content);
                    }
                    if chunk.done {
                        break;
                    }
                }
            }
        }

        Ok(full_response)
    }

    pub fn model_name(&self) -> &str {
        &self.model
    }
}
