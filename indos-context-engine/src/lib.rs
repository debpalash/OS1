//! IndOS Context Engine — Semantic Memory System
//!
//! Provides persistent, searchable context memory using:
//! - **LanceDB** — embedded vector database (zero-config, Rust-native)
//! - **Local embedding model** — nomic-embed-text via Ollama
//! - **Temporal patterns** — time-aware context retrieval
//!
//! What gets indexed:
//! - Conversation history (every message, searchable)
//! - File paths + summaries (what the user has opened)
//! - Terminal commands (what was run and when)
//! - Application context (what app was focused)
//! - Clipboard contents (with user consent)

use anyhow::Result;
use chrono::Utc;
use lancedb::connect;
use lancedb::query::{ExecutableQuery, QueryBase};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

/// A memory entry in the context engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique ID
    pub id: String,

    /// The text content to be embedded and searched
    pub content: String,

    /// Source of this memory
    pub source: MemorySource,

    /// When this memory was created (ISO8601)
    pub timestamp: String,

    /// Optional metadata as JSON string
    #[serde(default)]
    pub metadata: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemorySource {
    Conversation,
    FileAccess,
    TerminalCommand,
    ApplicationContext,
    Clipboard,
    ScreenCapture,
    UserNote,
    SystemEvent,
}

impl std::fmt::Display for MemorySource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemorySource::Conversation => write!(f, "conversation"),
            MemorySource::FileAccess => write!(f, "file_access"),
            MemorySource::TerminalCommand => write!(f, "terminal_command"),
            MemorySource::ApplicationContext => write!(f, "app_context"),
            MemorySource::Clipboard => write!(f, "clipboard"),
            MemorySource::ScreenCapture => write!(f, "screen_capture"),
            MemorySource::UserNote => write!(f, "user_note"),
            MemorySource::SystemEvent => write!(f, "system_event"),
        }
    }
}

/// Search result from context memory
#[derive(Debug, Clone)]
pub struct ContextResult {
    pub entry: MemoryEntry,
    pub relevance_score: f32,
}

/// Embedding dimension for nomic-embed-text
const EMBED_DIM: usize = 768;

/// The context engine manages all persistent memory
pub struct ContextEngine {
    /// LanceDB connection
    db: Option<lancedb::Connection>,

    /// Database path
    db_path: String,

    /// Ollama embedding endpoint
    ollama_url: String,

    /// Embedding model name
    embedding_model: String,

    /// HTTP client for Ollama
    http: reqwest::Client,

    /// Whether the engine is initialized
    ready: bool,
}

impl ContextEngine {
    pub fn new(db_path: &str, embedding_model: &str) -> Self {
        Self {
            db: None,
            db_path: db_path.to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            embedding_model: embedding_model.to_string(),
            http: reqwest::Client::new(),
            ready: false,
        }
    }

    /// Initialize LanceDB and create tables
    pub async fn init(&mut self) -> Result<()> {
        tracing::info!("Initializing context engine at {}", self.db_path);

        // Ensure directory exists
        if let Some(parent) = Path::new(&self.db_path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Connect to LanceDB (creates if not exists)
        let db = connect(&self.db_path).execute().await?;

        // Check if memories table exists, create if not
        let tables = db.table_names().execute().await?;
        if !tables.contains(&"memories".to_string()) {
            tracing::info!("Creating memories table...");
            self.create_memories_table(&db).await?;
        }

        self.db = Some(db);
        self.ready = true;
        tracing::info!("Context engine ready (embedding: {})", self.embedding_model);
        Ok(())
    }

    /// Create the memories table with schema
    async fn create_memories_table(&self, db: &lancedb::Connection) -> Result<()> {
        use arrow_array::{RecordBatch, RecordBatchIterator};
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("source", DataType::Utf8, false),
            Field::new("timestamp", DataType::Utf8, false),
            Field::new("metadata", DataType::Utf8, true),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    EMBED_DIM as i32,
                ),
                true,
            ),
        ]));

        // Create empty table with schema
        let batch = RecordBatch::new_empty(schema.clone());
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
        db.create_table("memories", Box::new(batches))
            .execute()
            .await?;

        tracing::info!("Memories table created (vector dim: {})", EMBED_DIM);
        Ok(())
    }

    /// Generate embedding via Ollama
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        #[derive(Serialize)]
        struct EmbedRequest<'a> {
            model: &'a str,
            input: &'a str,
        }

        #[derive(Deserialize)]
        struct EmbedResponse {
            embeddings: Vec<Vec<f32>>,
        }

        let resp = self
            .http
            .post(format!("{}/api/embed", self.ollama_url))
            .json(&EmbedRequest {
                model: &self.embedding_model,
                input: text,
            })
            .send()
            .await?
            .json::<EmbedResponse>()
            .await?;

        resp.embeddings
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No embedding returned"))
    }

    /// Store a new memory with embedding
    pub async fn remember(&self, entry: MemoryEntry) -> Result<()> {
        let db = self
            .db
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not initialized"))?;

        // Generate embedding
        let embedding = match self.embed(&entry.content).await {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(
                    "Embedding failed ({}), storing without vector: {}",
                    self.embedding_model,
                    e
                );
                vec![0.0f32; EMBED_DIM]
            }
        };

        // Build record batch
        use arrow_array::{
            ArrayRef, FixedSizeListArray, Float32Array, RecordBatch, RecordBatchIterator,
            StringArray,
        };
        use arrow_schema::{DataType, Field, Schema};

        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("source", DataType::Utf8, false),
            Field::new("timestamp", DataType::Utf8, false),
            Field::new("metadata", DataType::Utf8, true),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    EMBED_DIM as i32,
                ),
                true,
            ),
        ]));

        let values = Float32Array::from(embedding);
        let field = Arc::new(Field::new("item", DataType::Float32, true));
        let vector_array = FixedSizeListArray::new(field, EMBED_DIM as i32, Arc::new(values), None);

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(StringArray::from(vec![entry.id.as_str()])) as ArrayRef,
                Arc::new(StringArray::from(vec![entry.content.as_str()])),
                Arc::new(StringArray::from(vec![entry.source.to_string().as_str()])),
                Arc::new(StringArray::from(vec![entry.timestamp.as_str()])),
                Arc::new(StringArray::from(vec![entry.metadata.as_str()])),
                Arc::new(vector_array),
            ],
        )?;

        let table = db.open_table("memories").execute().await?;
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
        table.add(Box::new(batches)).execute().await?;

        tracing::debug!("Stored memory: {}", entry.id);
        Ok(())
    }

    /// Search memories by semantic similarity
    pub async fn recall(&self, query: &str, limit: usize) -> Result<Vec<ContextResult>> {
        let db = self
            .db
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not initialized"))?;

        // Embed query
        let query_embedding = self.embed(query).await?;

        // Vector search
        let table = db.open_table("memories").execute().await?;
        let results = table
            .vector_search(query_embedding)?
            .limit(limit)
            .execute()
            .await?;

        use arrow_array::cast::AsArray;
        use futures::TryStreamExt;

        let batches: Vec<_> = results.try_collect().await?;
        let mut context_results = Vec::new();

        for batch in &batches {
            let ids = batch.column_by_name("id").unwrap().as_string::<i32>();
            let contents = batch.column_by_name("content").unwrap().as_string::<i32>();
            let sources = batch.column_by_name("source").unwrap().as_string::<i32>();
            let timestamps = batch
                .column_by_name("timestamp")
                .unwrap()
                .as_string::<i32>();
            let distances = batch
                .column_by_name("_distance")
                .and_then(|c| c.as_any().downcast_ref::<arrow_array::Float32Array>());

            for i in 0..batch.num_rows() {
                let source = match sources.value(i) {
                    "conversation" => MemorySource::Conversation,
                    "file_access" => MemorySource::FileAccess,
                    "terminal_command" => MemorySource::TerminalCommand,
                    "app_context" => MemorySource::ApplicationContext,
                    "clipboard" => MemorySource::Clipboard,
                    _ => MemorySource::SystemEvent,
                };

                let distance = distances.map(|d| d.value(i)).unwrap_or(1.0);
                let score = 1.0 / (1.0 + distance); // Convert distance to relevance

                context_results.push(ContextResult {
                    entry: MemoryEntry {
                        id: ids.value(i).to_string(),
                        content: contents.value(i).to_string(),
                        source,
                        timestamp: timestamps.value(i).to_string(),
                        metadata: String::new(),
                    },
                    relevance_score: score,
                });
            }
        }

        Ok(context_results)
    }

    /// Get recent memories (last N)
    pub async fn recent(&self, limit: usize) -> Result<Vec<MemoryEntry>> {
        let db = self
            .db
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not initialized"))?;

        let table = db.open_table("memories").execute().await?;
        let results = table.query().limit(limit).execute().await?;

        use arrow_array::cast::AsArray;
        use futures::TryStreamExt;

        let batches: Vec<_> = results.try_collect().await?;
        let mut entries = Vec::new();

        for batch in &batches {
            let ids = batch.column_by_name("id").unwrap().as_string::<i32>();
            let contents = batch.column_by_name("content").unwrap().as_string::<i32>();
            let sources = batch.column_by_name("source").unwrap().as_string::<i32>();
            let timestamps = batch
                .column_by_name("timestamp")
                .unwrap()
                .as_string::<i32>();

            for i in 0..batch.num_rows() {
                let source = match sources.value(i) {
                    "conversation" => MemorySource::Conversation,
                    "file_access" => MemorySource::FileAccess,
                    "terminal_command" => MemorySource::TerminalCommand,
                    _ => MemorySource::SystemEvent,
                };

                entries.push(MemoryEntry {
                    id: ids.value(i).to_string(),
                    content: contents.value(i).to_string(),
                    source,
                    timestamp: timestamps.value(i).to_string(),
                    metadata: String::new(),
                });
            }
        }

        Ok(entries)
    }

    /// Count total memories
    pub async fn count(&self) -> Result<usize> {
        let db = self
            .db
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not initialized"))?;
        let table = db.open_table("memories").execute().await?;
        let count = table.count_rows(None).await?;
        Ok(count)
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }
}

/// Create a new memory entry with auto-generated ID and timestamp
pub fn new_memory(content: &str, source: MemorySource) -> MemoryEntry {
    MemoryEntry {
        id: uuid::Uuid::new_v4().to_string(),
        content: content.to_string(),
        source,
        timestamp: Utc::now().to_rfc3339(),
        metadata: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_source_display_strings() {
        assert_eq!(MemorySource::Conversation.to_string(), "conversation");
        assert_eq!(MemorySource::FileAccess.to_string(), "file_access");
        assert_eq!(
            MemorySource::TerminalCommand.to_string(),
            "terminal_command"
        );
        assert_eq!(MemorySource::ApplicationContext.to_string(), "app_context");
        assert_eq!(MemorySource::Clipboard.to_string(), "clipboard");
        assert_eq!(MemorySource::ScreenCapture.to_string(), "screen_capture");
        assert_eq!(MemorySource::UserNote.to_string(), "user_note");
        assert_eq!(MemorySource::SystemEvent.to_string(), "system_event");
    }

    #[test]
    fn memory_entry_serde_round_trip() {
        let entry = MemoryEntry {
            id: "abc-123".to_string(),
            content: "hello world".to_string(),
            source: MemorySource::TerminalCommand,
            timestamp: "2026-06-26T00:00:00+00:00".to_string(),
            metadata: r#"{"k":"v"}"#.to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let back: MemoryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, entry.id);
        assert_eq!(back.content, entry.content);
        assert_eq!(back.timestamp, entry.timestamp);
        assert_eq!(back.metadata, entry.metadata);
        assert_eq!(back.source.to_string(), "terminal_command");
    }

    #[test]
    fn memory_entry_metadata_defaults_when_absent() {
        // metadata carries #[serde(default)] so a payload without it parses.
        let json = r#"{
            "id": "id-1",
            "content": "no metadata here",
            "source": "Conversation",
            "timestamp": "2026-06-26T00:00:00+00:00"
        }"#;
        let entry: MemoryEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.metadata, "");
        assert!(matches!(entry.source, MemorySource::Conversation));
    }

    #[test]
    fn new_memory_populates_fields() {
        let entry = new_memory("a note", MemorySource::UserNote);
        assert_eq!(entry.content, "a note");
        assert!(entry.metadata.is_empty());
        assert!(matches!(entry.source, MemorySource::UserNote));
        // UUID v4 hyphenated form is 36 chars.
        assert_eq!(entry.id.len(), 36);
        // Timestamp is valid RFC3339.
        assert!(chrono::DateTime::parse_from_rfc3339(&entry.timestamp).is_ok());
    }

    #[test]
    fn new_memory_ids_are_unique() {
        let a = new_memory("x", MemorySource::SystemEvent);
        let b = new_memory("x", MemorySource::SystemEvent);
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn context_result_relevance_score_from_distance() {
        // Mirror the distance→relevance transform used in `recall`.
        let score = |distance: f32| 1.0 / (1.0 + distance);
        assert!((score(0.0) - 1.0).abs() < f32::EPSILON);
        assert!((score(1.0) - 0.5).abs() < f32::EPSILON);
        // Smaller distance must yield higher relevance.
        assert!(score(0.2) > score(2.0));
    }
}
