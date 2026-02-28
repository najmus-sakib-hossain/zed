//! SQLite-backed session store for the gateway.

use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;

/// Session record stored in SQLite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: String,
    pub user_id: String,
    pub channel: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
    pub is_active: bool,
}

/// SQLite session store
pub struct SessionStore {
    conn: Mutex<Connection>,
}

impl SessionStore {
    /// Open or create the session database
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;

        // Enable WAL mode for better concurrent performance
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                channel TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                expires_at TEXT,
                metadata TEXT NOT NULL DEFAULT '{}',
                is_active INTEGER NOT NULL DEFAULT 1
            );
            CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_active ON sessions(is_active);
            CREATE INDEX IF NOT EXISTS idx_sessions_channel ON sessions(channel);",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Open an in-memory database (for testing)
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                channel TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                expires_at TEXT,
                metadata TEXT NOT NULL DEFAULT '{}',
                is_active INTEGER NOT NULL DEFAULT 1
            );",
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Create a new session
    pub fn create_session(&self, session: &SessionRecord) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        conn.execute(
            "INSERT INTO sessions (id, user_id, channel, created_at, updated_at, expires_at, metadata, is_active)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                session.id,
                session.user_id,
                session.channel,
                session.created_at.to_rfc3339(),
                session.updated_at.to_rfc3339(),
                session.expires_at.map(|t| t.to_rfc3339()),
                session.metadata.to_string(),
                session.is_active as i32,
            ],
        )?;
        Ok(())
    }

    /// Get a session by ID
    pub fn get_session(&self, id: &str) -> Result<Option<SessionRecord>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, channel, created_at, updated_at, expires_at, metadata, is_active
             FROM sessions WHERE id = ?1",
        )?;

        let result = stmt.query_row(params![id], |row| {
            Ok(SessionRecord {
                id: row.get(0)?,
                user_id: row.get(1)?,
                channel: row.get(2)?,
                created_at: parse_datetime(&row.get::<_, String>(3)?),
                updated_at: parse_datetime(&row.get::<_, String>(4)?),
                expires_at: row.get::<_, Option<String>>(5)?.map(|s| parse_datetime(&s)),
                metadata: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
                is_active: row.get::<_, i32>(7)? != 0,
            })
        });

        match result {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Update session metadata and timestamp
    pub fn update_session(&self, id: &str, metadata: &serde_json::Value) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        conn.execute(
            "UPDATE sessions SET metadata = ?1, updated_at = ?2 WHERE id = ?3",
            params![metadata.to_string(), Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }

    /// Deactivate a session
    pub fn deactivate_session(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        conn.execute(
            "UPDATE sessions SET is_active = 0, updated_at = ?1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }

    /// List active sessions
    pub fn list_sessions(&self) -> Result<Vec<SessionRecord>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, channel, created_at, updated_at, expires_at, metadata, is_active
             FROM sessions WHERE is_active = 1 ORDER BY updated_at DESC",
        )?;

        let sessions = stmt
            .query_map([], |row| {
                Ok(SessionRecord {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    channel: row.get(2)?,
                    created_at: parse_datetime(&row.get::<_, String>(3)?),
                    updated_at: parse_datetime(&row.get::<_, String>(4)?),
                    expires_at: row.get::<_, Option<String>>(5)?.map(|s| parse_datetime(&s)),
                    metadata: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
                    is_active: row.get::<_, i32>(7)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(sessions)
    }

    /// Get sessions for a specific user
    pub fn get_user_sessions(&self, user_id: &str) -> Result<Vec<SessionRecord>> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        let mut stmt = conn.prepare(
            "SELECT id, user_id, channel, created_at, updated_at, expires_at, metadata, is_active
             FROM sessions WHERE user_id = ?1 AND is_active = 1 ORDER BY updated_at DESC",
        )?;

        let sessions = stmt
            .query_map(params![user_id], |row| {
                Ok(SessionRecord {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    channel: row.get(2)?,
                    created_at: parse_datetime(&row.get::<_, String>(3)?),
                    updated_at: parse_datetime(&row.get::<_, String>(4)?),
                    expires_at: row.get::<_, Option<String>>(5)?.map(|s| parse_datetime(&s)),
                    metadata: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
                    is_active: row.get::<_, i32>(7)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(sessions)
    }

    /// Count active sessions
    pub fn count(&self) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM sessions WHERE is_active = 1", [], |row| {
                row.get(0)
            })?;
        Ok(count as usize)
    }

    /// Clean up expired sessions
    pub fn cleanup_expired(&self) -> Result<usize> {
        let conn = self.conn.lock().map_err(|e| anyhow::anyhow!("{}", e))?;
        let now = Utc::now().to_rfc3339();
        let deleted = conn.execute(
            "UPDATE sessions SET is_active = 0 WHERE expires_at IS NOT NULL AND expires_at < ?1 AND is_active = 1",
            params![now],
        )?;
        Ok(deleted)
    }
}

fn parse_datetime(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_and_get_session() {
        let store = SessionStore::in_memory().expect("open");
        let session = SessionRecord {
            id: "sess-1".into(),
            user_id: "user-1".into(),
            channel: Some("telegram".into()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
            metadata: json!({"model": "gpt-4"}),
            is_active: true,
        };

        store.create_session(&session).expect("create");
        let retrieved = store.get_session("sess-1").expect("get").expect("exists");
        assert_eq!(retrieved.user_id, "user-1");
        assert_eq!(retrieved.channel, Some("telegram".into()));
    }

    #[test]
    fn test_list_sessions() {
        let store = SessionStore::in_memory().expect("open");

        for i in 0..3 {
            let session = SessionRecord {
                id: format!("sess-{}", i),
                user_id: "user-1".into(),
                channel: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
                expires_at: None,
                metadata: json!({}),
                is_active: true,
            };
            store.create_session(&session).expect("create");
        }

        let sessions = store.list_sessions().expect("list");
        assert_eq!(sessions.len(), 3);
    }

    #[test]
    fn test_deactivate_session() {
        let store = SessionStore::in_memory().expect("open");
        let session = SessionRecord {
            id: "sess-1".into(),
            user_id: "user-1".into(),
            channel: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
            metadata: json!({}),
            is_active: true,
        };

        store.create_session(&session).expect("create");
        assert_eq!(store.count().expect("count"), 1);

        store.deactivate_session("sess-1").expect("deactivate");
        assert_eq!(store.count().expect("count"), 0);
    }

    #[test]
    fn test_update_session_metadata() {
        let store = SessionStore::in_memory().expect("open");
        let session = SessionRecord {
            id: "sess-1".into(),
            user_id: "user-1".into(),
            channel: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            expires_at: None,
            metadata: json!({}),
            is_active: true,
        };

        store.create_session(&session).expect("create");
        store
            .update_session("sess-1", &json!({"model": "claude-4", "tokens": 1000}))
            .expect("update");

        let updated = store.get_session("sess-1").expect("get").expect("exists");
        assert_eq!(updated.metadata["model"], "claude-4");
    }
}
