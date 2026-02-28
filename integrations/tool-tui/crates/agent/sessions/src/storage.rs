//! Session storage backends.

use anyhow::Result;
use async_trait::async_trait;
use rusqlite::Connection;
use std::sync::Mutex;
use tracing::info;

use crate::manager::Session;

/// Storage backend trait for session persistence
#[async_trait]
pub trait SessionStorage: Send + Sync {
    /// Save a session
    async fn save_session(&self, session: &Session) -> Result<()>;
    /// Load a session by ID
    async fn load_session(&self, session_id: &str) -> Result<Option<SessionRecord>>;
    /// List all sessions for a user
    async fn list_user_sessions(&self, user_id: &str) -> Result<Vec<SessionRecord>>;
    /// Delete a session
    async fn delete_session(&self, session_id: &str) -> Result<()>;
    /// Clean up old sessions
    async fn cleanup(&self, max_age_secs: u64) -> Result<usize>;
}

/// Serialized session record for storage
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub user_id: String,
    pub channel: String,
    pub chat_id: String,
    pub state: String,
    pub context_json: String,
    pub metadata_json: String,
    pub created_at: String,
    pub last_activity: String,
}

/// SQLite-backed session storage
pub struct SqliteSessionStorage {
    conn: Mutex<Connection>,
}

impl SqliteSessionStorage {
    /// Open or create a SQLite database for sessions
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA cache_size=-64000;",
        )?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                channel TEXT NOT NULL,
                chat_id TEXT NOT NULL,
                state TEXT NOT NULL DEFAULT 'active',
                context_json TEXT NOT NULL DEFAULT '[]',
                metadata_json TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL,
                last_activity TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_channel ON sessions(channel, chat_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_activity ON sessions(last_activity);",
        )?;
        info!("Session storage opened at {}", path);
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Create an in-memory storage (for testing)
    pub fn in_memory() -> Result<Self> {
        Self::open(":memory:")
    }
}

#[async_trait]
impl SessionStorage for SqliteSessionStorage {
    async fn save_session(&self, session: &Session) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock: {}", e))?;

        let context_json = serde_json::to_string(
            &session
                .context
                .messages()
                .iter()
                .map(|m| {
                    serde_json::json!({
                        "id": m.id,
                        "role": m.role,
                        "content": m.content,
                        "token_count": m.token_count,
                    })
                })
                .collect::<Vec<_>>(),
        )?;

        let metadata_json = serde_json::to_string(&session.metadata)?;

        conn.execute(
            "INSERT OR REPLACE INTO sessions (id, user_id, channel, chat_id, state, context_json, metadata_json, created_at, last_activity)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                session.id,
                session.user_id,
                session.channel,
                session.chat_id,
                format!("{:?}", session.state),
                context_json,
                metadata_json,
                session.created_at.to_rfc3339(),
                session.last_activity.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    async fn load_session(&self, session_id: &str) -> Result<Option<SessionRecord>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock: {}", e))?;

        let mut stmt = conn.prepare(
            "SELECT id, user_id, channel, chat_id, state, context_json, metadata_json, created_at, last_activity
             FROM sessions WHERE id = ?1",
        )?;

        let record = stmt
            .query_row(rusqlite::params![session_id], |row| {
                Ok(SessionRecord {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    channel: row.get(2)?,
                    chat_id: row.get(3)?,
                    state: row.get(4)?,
                    context_json: row.get(5)?,
                    metadata_json: row.get(6)?,
                    created_at: row.get(7)?,
                    last_activity: row.get(8)?,
                })
            })
            .ok();

        Ok(record)
    }

    async fn list_user_sessions(&self, user_id: &str) -> Result<Vec<SessionRecord>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock: {}", e))?;

        let mut stmt = conn.prepare(
            "SELECT id, user_id, channel, chat_id, state, context_json, metadata_json, created_at, last_activity
             FROM sessions WHERE user_id = ?1 ORDER BY last_activity DESC",
        )?;

        let records = stmt
            .query_map(rusqlite::params![user_id], |row| {
                Ok(SessionRecord {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    channel: row.get(2)?,
                    chat_id: row.get(3)?,
                    state: row.get(4)?,
                    context_json: row.get(5)?,
                    metadata_json: row.get(6)?,
                    created_at: row.get(7)?,
                    last_activity: row.get(8)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(records)
    }

    async fn delete_session(&self, session_id: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock: {}", e))?;
        conn.execute("DELETE FROM sessions WHERE id = ?1", rusqlite::params![session_id])?;
        Ok(())
    }

    async fn cleanup(&self, max_age_secs: u64) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("Lock: {}", e))?;
        let cutoff = chrono::Utc::now() - chrono::Duration::seconds(max_age_secs as i64);
        let deleted = conn.execute(
            "DELETE FROM sessions WHERE last_activity < ?1",
            rusqlite::params![cutoff.to_rfc3339()],
        )?;
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::{SessionConfig, SessionState};

    #[tokio::test]
    async fn test_sqlite_storage() {
        let storage = SqliteSessionStorage::in_memory().unwrap();

        let session = Session::new(
            "user1".into(),
            "telegram".into(),
            "chat1".into(),
            SessionConfig::default(),
        );
        let session_id = session.id.clone();

        storage.save_session(&session).await.unwrap();

        let loaded = storage.load_session(&session_id).await.unwrap();
        assert!(loaded.is_some());
        let record = loaded.unwrap();
        assert_eq!(record.user_id, "user1");
        assert_eq!(record.channel, "telegram");
    }

    #[tokio::test]
    async fn test_list_user_sessions() {
        let storage = SqliteSessionStorage::in_memory().unwrap();

        for i in 0..3 {
            let session = Session::new(
                "user1".into(),
                "telegram".into(),
                format!("chat{}", i),
                SessionConfig::default(),
            );
            storage.save_session(&session).await.unwrap();
        }

        let sessions = storage.list_user_sessions("user1").await.unwrap();
        assert_eq!(sessions.len(), 3);
    }
}
