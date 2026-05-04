use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{Emitter, State};

mod db;
mod models;
mod system;

// ── Application State ──────────────────────────────────────────────────────

pub struct AppState {
    pub db: Mutex<db::Database>,
    pub ollama_url: Mutex<String>,
}

// ── Data Types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: String, // "user" | "assistant" | "system"
    pub content: String,
    pub fragments: Vec<UIFragment>,
    pub timestamp: String,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIFragment {
    #[serde(rename = "type")]
    pub fragment_type: String,
    pub component: String,
    pub props: serde_json::Value,
    pub actions: Vec<FragmentAction>,
    pub persistence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FragmentAction {
    pub label: String,
    pub handler: String,
    pub args: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationThread {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub message_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub size: Option<u64>,
    pub modified_at: Option<String>,
}

// ── Tauri Commands: Chat ───────────────────────────────────────────────────

#[tauri::command]
async fn send_message(
    state: State<'_, AppState>,
    thread_id: String,
    content: String,
) -> Result<Message, String> {
    let ollama_url = state.ollama_url.lock().unwrap().clone();

    // Save user message
    let user_msg = Message {
        id: uuid::Uuid::new_v4().to_string(),
        role: "user".into(),
        content: content.clone(),
        fragments: vec![],
        timestamp: chrono::Utc::now().to_rfc3339(),
        model: None,
    };

    {
        let db = state.db.lock().unwrap();
        db.save_message(&thread_id, &user_msg).ok();
    }

    // Get conversation history for context
    let history = {
        let db = state.db.lock().unwrap();
        db.get_messages(&thread_id).unwrap_or_default()
    };

    // Build messages for Ollama
    let mut api_messages: Vec<serde_json::Value> = vec![serde_json::json!({
        "role": "system",
        "content": SYSTEM_PROMPT
    })];

    // Include last 20 messages for context
    for msg in history.iter().rev().take(20).rev() {
        api_messages.push(serde_json::json!({
            "role": msg.role,
            "content": msg.content
        }));
    }

    api_messages.push(serde_json::json!({
        "role": "user",
        "content": content
    }));

    // Call Ollama
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/api/chat", ollama_url))
        .json(&serde_json::json!({
            "model": "llama3.2",
            "messages": api_messages,
            "stream": false,
            "options": {
                "temperature": 0.7,
                "num_predict": 2048
            }
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to reach Ollama: {}. Is Ollama running?", e))?;

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;

    let assistant_content = body["message"]["content"]
        .as_str()
        .unwrap_or("I couldn't generate a response.")
        .to_string();

    let model_name = body["model"].as_str().map(|s| s.to_string());

    // Parse UI fragments from the response
    let fragments = parse_fragments(&assistant_content);

    let assistant_msg = Message {
        id: uuid::Uuid::new_v4().to_string(),
        role: "assistant".into(),
        content: assistant_content,
        fragments,
        timestamp: chrono::Utc::now().to_rfc3339(),
        model: model_name,
    };

    // Save assistant message
    {
        let db = state.db.lock().unwrap();
        db.save_message(&thread_id, &assistant_msg).ok();
    }

    Ok(assistant_msg)
}

#[tauri::command]
async fn send_message_streaming(
    state: State<'_, AppState>,
    window: tauri::Window,
    thread_id: String,
    content: String,
    model: Option<String>,
) -> Result<String, String> {
    use futures::StreamExt;

    let ollama_url = state.ollama_url.lock().unwrap().clone();
    let model_name = model.unwrap_or_else(|| "llama3.2".to_string());

    // Save user message
    let user_msg = Message {
        id: uuid::Uuid::new_v4().to_string(),
        role: "user".into(),
        content: content.clone(),
        fragments: vec![],
        timestamp: chrono::Utc::now().to_rfc3339(),
        model: None,
    };

    {
        let db = state.db.lock().unwrap();
        db.save_message(&thread_id, &user_msg).ok();
    }

    // Get conversation history
    let history = {
        let db = state.db.lock().unwrap();
        db.get_messages(&thread_id).unwrap_or_default()
    };

    let mut api_messages: Vec<serde_json::Value> = vec![serde_json::json!({
        "role": "system",
        "content": SYSTEM_PROMPT
    })];

    for msg in history.iter().rev().take(20).rev() {
        api_messages.push(serde_json::json!({
            "role": msg.role,
            "content": msg.content
        }));
    }

    api_messages.push(serde_json::json!({
        "role": "user",
        "content": content
    }));

    let msg_id = uuid::Uuid::new_v4().to_string();
    let msg_id_clone = msg_id.clone();

    // Spawn streaming task
    let window_clone = window.clone();
    let thread_id_clone = thread_id.clone();

    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/api/chat", ollama_url))
            .json(&serde_json::json!({
                "model": model_name,
                "messages": api_messages,
                "stream": true,
                "options": {
                    "temperature": 0.7,
                    "num_predict": 2048
                }
            }))
            .send()
            .await;

        match response {
            Ok(resp) => {
                let mut stream = resp.bytes_stream();
                let mut full_content = String::new();

                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(bytes) => {
                            let text = String::from_utf8_lossy(&bytes);
                            for line in text.lines() {
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
                                    if let Some(content) = json["message"]["content"].as_str() {
                                        full_content.push_str(content);
                                        let _ = window_clone.emit("stream-chunk", serde_json::json!({
                                            "msgId": msg_id_clone,
                                            "chunk": content,
                                            "done": json["done"].as_bool().unwrap_or(false)
                                        }));
                                    }

                                    if json["done"].as_bool().unwrap_or(false) {
                                        let fragments = parse_fragments(&full_content);
                                        let _ = window_clone.emit("stream-complete", serde_json::json!({
                                            "msgId": msg_id_clone,
                                            "content": full_content,
                                            "fragments": fragments,
                                            "model": json["model"].as_str()
                                        }));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            let _ = window_clone.emit("stream-error", serde_json::json!({
                                "msgId": msg_id_clone,
                                "error": format!("{}", e)
                            }));
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                let _ = window_clone.emit("stream-error", serde_json::json!({
                    "msgId": msg_id_clone,
                    "error": format!("Connection failed: {}. Is Ollama running?", e)
                }));
            }
        }
    });

    Ok(msg_id)
}

// ── Tauri Commands: Threads ────────────────────────────────────────────────

#[tauri::command]
fn create_thread(state: State<'_, AppState>, title: Option<String>) -> Result<ConversationThread, String> {
    let db = state.db.lock().unwrap();
    let thread = ConversationThread {
        id: uuid::Uuid::new_v4().to_string(),
        title: title.unwrap_or_else(|| "New Conversation".into()),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
        message_count: 0,
    };
    db.save_thread(&thread).map_err(|e| format!("{}", e))?;
    Ok(thread)
}

#[tauri::command]
fn list_threads(state: State<'_, AppState>) -> Result<Vec<ConversationThread>, String> {
    let db = state.db.lock().unwrap();
    db.list_threads().map_err(|e| format!("{}", e))
}

#[tauri::command]
fn get_thread_messages(state: State<'_, AppState>, thread_id: String) -> Result<Vec<Message>, String> {
    let db = state.db.lock().unwrap();
    db.get_messages(&thread_id).map_err(|e| format!("{}", e))
}

#[tauri::command]
fn delete_thread(state: State<'_, AppState>, thread_id: String) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    db.delete_thread(&thread_id).map_err(|e| format!("{}", e))
}

// ── Tauri Commands: Models ─────────────────────────────────────────────────

#[tauri::command]
async fn list_models(state: State<'_, AppState>) -> Result<Vec<OllamaModel>, String> {
    let ollama_url = state.ollama_url.lock().unwrap().clone();
    models::list_ollama_models(&ollama_url).await
}

#[tauri::command]
async fn check_ollama_status(state: State<'_, AppState>) -> Result<bool, String> {
    let ollama_url = state.ollama_url.lock().unwrap().clone();
    models::check_ollama(&ollama_url).await
}

// ── Tauri Commands: System ─────────────────────────────────────────────────

#[tauri::command]
fn get_system_info() -> Result<serde_json::Value, String> {
    Ok(system::get_system_info())
}

#[tauri::command]
fn list_directory(path: String) -> Result<Vec<serde_json::Value>, String> {
    system::list_directory(&path)
}

#[tauri::command]
fn read_file_content(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read {}: {}", path, e))
}

// ── Fragment Parser ────────────────────────────────────────────────────────

fn parse_fragments(content: &str) -> Vec<UIFragment> {
    let mut fragments = Vec::new();

    // Look for ```indos-ui code blocks
    let mut in_block = false;
    let mut block_content = String::new();

    for line in content.lines() {
        if line.trim().starts_with("```indos-ui") {
            in_block = true;
            block_content.clear();
        } else if in_block && line.trim() == "```" {
            in_block = false;
            if let Ok(fragment) = serde_json::from_str::<UIFragment>(&block_content) {
                fragments.push(fragment);
            }
        } else if in_block {
            block_content.push_str(line);
            block_content.push('\n');
        }
    }

    // Also detect implicit fragments from content patterns
    if fragments.is_empty() {
        // Check for file listing patterns
        if content.contains("```") {
            fragments.push(UIFragment {
                fragment_type: "code".into(),
                component: "code-block".into(),
                props: serde_json::json!({"detected": true}),
                actions: vec![],
                persistence: "session".into(),
            });
        }
    }

    fragments
}

// ── System Prompt ──────────────────────────────────────────────────────────

const SYSTEM_PROMPT: &str = r#"You are IndOS, an AI-first operating system assistant. You are the primary interface between the user and their computer.

Your capabilities:
- Help users manage files, applications, and system settings through natural conversation
- Generate rich UI fragments when appropriate (file browsers, code editors, system monitors, etc.)
- Execute system commands and report results
- Provide intelligent suggestions based on context

When the user asks to see files, system information, or needs interactive UI, you can emit structured UI fragments using this format:

```indos-ui
{
  "type": "grid|list|form|chart|terminal|code|media|card",
  "component": "component-name",
  "props": { ... },
  "actions": [{"label": "Button Text", "handler": "handlerName"}],
  "persistence": "ephemeral|session|pinned"
}
```

For example, when showing files:
```indos-ui
{
  "type": "grid",
  "component": "file-browser",
  "props": {"path": "/home/user", "items": [...]},
  "actions": [{"label": "Open", "handler": "openFile"}, {"label": "Delete", "handler": "deleteFile"}],
  "persistence": "session"
}
```

Be conversational, helpful, and proactive. You ARE the desktop — make the user feel like they're having a conversation with their computer.
Always be concise but informative. Suggest next actions when appropriate."#;

// ── App Entry ──────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db = db::Database::new().expect("Failed to initialize database");
    db.migrate().expect("Failed to run database migrations");

    let state = AppState {
        db: Mutex::new(db),
        ollama_url: Mutex::new("http://localhost:11434".into()),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            send_message,
            send_message_streaming,
            create_thread,
            list_threads,
            get_thread_messages,
            delete_thread,
            list_models,
            check_ollama_status,
            get_system_info,
            list_directory,
            read_file_content,
        ])
        .run(tauri::generate_context!())
        .expect("error while running IndOS");
}
