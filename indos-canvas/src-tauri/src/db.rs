use rusqlite::{Connection, Result, params};
use std::path::PathBuf;

use crate::{ConversationThread, Message};

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Result<Self> {
        let db_path = Self::db_path();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(&db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
        Ok(Database { conn })
    }

    fn db_path() -> PathBuf {
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("indos");
        data_dir.join("indos.db")
    }

    pub fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS threads (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                thread_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                fragments TEXT DEFAULT '[]',
                model TEXT,
                timestamp TEXT NOT NULL,
                FOREIGN KEY (thread_id) REFERENCES threads(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_messages_thread ON messages(thread_id);
            CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);"
        )?;
        Ok(())
    }

    pub fn save_thread(&self, thread: &ConversationThread) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO threads (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![thread.id, thread.title, thread.created_at, thread.updated_at],
        )?;
        Ok(())
    }

    pub fn list_threads(&self) -> Result<Vec<ConversationThread>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.title, t.created_at, t.updated_at, 
                    (SELECT COUNT(*) FROM messages WHERE thread_id = t.id) as msg_count
             FROM threads t ORDER BY t.updated_at DESC"
        )?;

        let threads = stmt.query_map([], |row| {
            Ok(ConversationThread {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
                message_count: row.get(4)?,
            })
        })?.filter_map(|r| r.ok()).collect();

        Ok(threads)
    }

    pub fn delete_thread(&self, thread_id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM messages WHERE thread_id = ?1", params![thread_id])?;
        self.conn.execute("DELETE FROM threads WHERE id = ?1", params![thread_id])?;
        Ok(())
    }

    pub fn save_message(&self, thread_id: &str, msg: &Message) -> Result<()> {
        let fragments_json = serde_json::to_string(&msg.fragments).unwrap_or_else(|_| "[]".into());
        
        self.conn.execute(
            "INSERT OR REPLACE INTO messages (id, thread_id, role, content, fragments, model, timestamp) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![msg.id, thread_id, msg.role, msg.content, fragments_json, msg.model, msg.timestamp],
        )?;

        // Update thread's updated_at
        self.conn.execute(
            "UPDATE threads SET updated_at = ?1 WHERE id = ?2",
            params![msg.timestamp, thread_id],
        )?;

        Ok(())
    }

    pub fn get_messages(&self, thread_id: &str) -> Result<Vec<Message>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, role, content, fragments, model, timestamp 
             FROM messages WHERE thread_id = ?1 ORDER BY timestamp ASC"
        )?;

        let messages = stmt.query_map(params![thread_id], |row| {
            let fragments_str: String = row.get(3)?;
            let fragments: Vec<crate::UIFragment> = serde_json::from_str(&fragments_str).unwrap_or_default();
            
            Ok(Message {
                id: row.get(0)?,
                role: row.get(1)?,
                content: row.get(2)?,
                fragments,
                model: row.get(4)?,
                timestamp: row.get(5)?,
            })
        })?.filter_map(|r| r.ok()).collect();

        Ok(messages)
    }
}
