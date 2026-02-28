//! # Session Management System
//!
//! Provides persistent session management for DX CLI conversations.
//! Sessions track chat history, agent state, and metadata with support
//! for CRUD operations, compaction, repair, and export.
//!
//! ## Architecture
//!
//! ```text
//! SessionManager ─── SessionStorage (trait)
//!       │                    │
//!       ├─ DashMap cache     ├─ FileSessionStorage (default)
//!       ├─ CRUD ops          │    ├─ Atomic writes (temp + rename)
//!       └─ Key generation    │    ├─ Automatic backups
//!                            │    └─ Compression for large sessions
//!                            └─ (future: SQLite, etc.)
//! ```

pub mod compaction;
pub mod repair;
pub mod storage;
pub mod transcript;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use self::storage::FileSessionStorage;

/// Session management errors
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Compaction error: {0}")]
    CompactionError(String),
    #[error("Repair error: {0}")]
    RepairError(String),
    #[error("Export error: {0}")]
    ExportError(String),
}

/// Role of a message sender
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// User message
    User,
    /// Assistant/agent response
    Assistant,
    /// System prompt
    System,
    /// Tool/function call
    Tool,
}

/// A single message in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique message ID
    pub id: String,
    /// Role of the sender
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Token count (if known)
    #[serde(default)]
    pub token_count: Option<u32>,
    /// Tool call ID (if this is a tool response)
    #[serde(default)]
    pub tool_call_id: Option<String>,
    /// Tool name (if this is a tool call)
    #[serde(default)]
    pub tool_name: Option<String>,
    /// Metadata key-value pairs
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Session status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    /// Active session
    Active,
    /// Agent is processing
    Processing,
    /// Session is idle
    Idle,
    /// Session has been compacted
    Compacted,
    /// Session is archived
    Archived,
}

/// A conversation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session key (UUID v4)
    pub key: String,
    /// Optional human-readable title
    #[serde(default)]
    pub title: Option<String>,
    /// Agent/model ID used
    pub agent_id: String,
    /// Model name used
    #[serde(default)]
    pub model: Option<String>,
    /// Session status
    pub status: SessionStatus,
    /// Messages in the session
    pub messages: Vec<Message>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Total token count
    #[serde(default)]
    pub total_tokens: u64,
    /// Number of compactions performed
    #[serde(default)]
    pub compaction_count: u32,
    /// Session metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// Tags for organizing sessions
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Session {
    /// Create a new session with generated key
    pub fn new(agent_id: &str) -> Self {
        let key = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        Self {
            key,
            title: None,
            agent_id: agent_id.to_string(),
            model: None,
            status: SessionStatus::Active,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            total_tokens: 0,
            compaction_count: 0,
            metadata: HashMap::new(),
            tags: Vec::new(),
        }
    }

    /// Create a new session with a specific key
    pub fn with_key(key: &str, agent_id: &str) -> Self {
        let now = Utc::now();
        Self {
            key: key.to_string(),
            title: None,
            agent_id: agent_id.to_string(),
            model: None,
            status: SessionStatus::Active,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            total_tokens: 0,
            compaction_count: 0,
            metadata: HashMap::new(),
            tags: Vec::new(),
        }
    }

    /// Add a message to the session
    pub fn add_message(&mut self, role: MessageRole, content: &str) -> &Message {
        let msg = Message {
            id: uuid::Uuid::new_v4().to_string(),
            role,
            content: content.to_string(),
            timestamp: Utc::now(),
            token_count: None,
            tool_call_id: None,
            tool_name: None,
            metadata: HashMap::new(),
        };
        self.messages.push(msg);
        self.updated_at = Utc::now();
        self.messages.last().unwrap()
    }

    /// Get message count
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Compute approximate byte size
    pub fn byte_size(&self) -> usize {
        serde_json::to_vec(self).map(|v| v.len()).unwrap_or(0)
    }

    /// Generate a title from the first user message if none set
    pub fn auto_title(&mut self) {
        if self.title.is_some() {
            return;
        }
        if let Some(first_user_msg) = self.messages.iter().find(|m| m.role == MessageRole::User) {
            let title: String =
                first_user_msg.content.chars().take(80).collect::<String>().trim().to_string();
            if !title.is_empty() {
                self.title = Some(if first_user_msg.content.len() > 80 {
                    format!("{}...", title)
                } else {
                    title
                });
            }
        }
    }
}

/// Session listing filter options
#[derive(Debug, Clone, Default)]
pub struct SessionFilter {
    /// Filter by agent ID
    pub agent_id: Option<String>,
    /// Filter by status
    pub status: Option<SessionStatus>,
    /// Filter by tag
    pub tag: Option<String>,
    /// Created after this time
    pub created_after: Option<DateTime<Utc>>,
    /// Created before this time
    pub created_before: Option<DateTime<Utc>>,
    /// Maximum results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

/// Session storage trait for pluggable backends
pub trait SessionStorage: Send + Sync {
    /// Save a session
    fn save(&self, session: &Session) -> Result<(), SessionError>;
    /// Load a session by key
    fn load(&self, key: &str) -> Result<Session, SessionError>;
    /// Delete a session
    fn delete(&self, key: &str) -> Result<(), SessionError>;
    /// List session keys (with optional filters)
    fn list(&self, filter: &SessionFilter) -> Result<Vec<Session>, SessionError>;
    /// Check if a session exists
    fn exists(&self, key: &str) -> Result<bool, SessionError>;
    /// Clear all sessions
    fn clear(&self) -> Result<usize, SessionError>;
}

/// Session manager with in-memory cache and pluggable storage
pub struct SessionManager {
    /// In-memory cache for fast lookups
    cache: DashMap<String, Session>,
    /// Persistent storage backend
    storage: Arc<dyn SessionStorage>,
    /// Storage path for reference
    storage_path: PathBuf,
}

impl SessionManager {
    /// Create a new SessionManager with file-based storage
    pub fn new(storage_path: PathBuf) -> Result<Self, SessionError> {
        let file_storage = FileSessionStorage::new(storage_path.clone())?;
        let storage: Arc<dyn SessionStorage> = Arc::new(file_storage);

        let manager = Self {
            cache: DashMap::new(),
            storage,
            storage_path,
        };

        // Pre-warm cache from storage
        manager.warm_cache()?;

        Ok(manager)
    }

    /// Create with a custom storage backend
    pub fn with_storage(storage: Arc<dyn SessionStorage>, storage_path: PathBuf) -> Self {
        let manager = Self {
            cache: DashMap::new(),
            storage,
            storage_path,
        };
        let _ = manager.warm_cache();
        manager
    }

    /// Pre-warm cache from storage
    fn warm_cache(&self) -> Result<(), SessionError> {
        let sessions = self.storage.list(&SessionFilter::default())?;
        for session in sessions {
            self.cache.insert(session.key.clone(), session);
        }
        Ok(())
    }

    /// Create a new session
    pub fn create(&self, agent_id: &str) -> Result<Session, SessionError> {
        let session = Session::new(agent_id);
        self.storage.save(&session)?;
        self.cache.insert(session.key.clone(), session.clone());
        Ok(session)
    }

    /// Create a session with a specific key
    pub fn create_with_key(&self, key: &str, agent_id: &str) -> Result<Session, SessionError> {
        let session = Session::with_key(key, agent_id);
        self.storage.save(&session)?;
        self.cache.insert(session.key.clone(), session.clone());
        Ok(session)
    }

    /// Get a session by key
    pub fn get(&self, key: &str) -> Result<Session, SessionError> {
        // Check cache first
        if let Some(session) = self.cache.get(key) {
            return Ok(session.clone());
        }
        // Fall back to storage
        let session = self.storage.load(key)?;
        self.cache.insert(key.to_string(), session.clone());
        Ok(session)
    }

    /// Update a session (save changes)
    pub fn update(&self, session: &Session) -> Result<(), SessionError> {
        self.storage.save(session)?;
        self.cache.insert(session.key.clone(), session.clone());
        Ok(())
    }

    /// Delete a session
    pub fn delete(&self, key: &str) -> Result<(), SessionError> {
        self.storage.delete(key)?;
        self.cache.remove(key);
        Ok(())
    }

    /// List sessions with optional filters
    pub fn list(&self, filter: &SessionFilter) -> Result<Vec<Session>, SessionError> {
        let mut sessions: Vec<Session> =
            self.cache.iter().map(|entry| entry.value().clone()).collect();

        // Apply filters
        if let Some(ref agent_id) = filter.agent_id {
            sessions.retain(|s| &s.agent_id == agent_id);
        }
        if let Some(ref status) = filter.status {
            sessions.retain(|s| &s.status == status);
        }
        if let Some(ref tag) = filter.tag {
            sessions.retain(|s| s.tags.contains(tag));
        }
        if let Some(after) = filter.created_after {
            sessions.retain(|s| s.created_at > after);
        }
        if let Some(before) = filter.created_before {
            sessions.retain(|s| s.created_at < before);
        }

        // Sort by updated_at descending (most recent first)
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        // Apply pagination
        if let Some(offset) = filter.offset {
            if offset < sessions.len() {
                sessions = sessions[offset..].to_vec();
            } else {
                sessions.clear();
            }
        }
        if let Some(limit) = filter.limit {
            sessions.truncate(limit);
        }

        Ok(sessions)
    }

    /// Add a message to a session
    pub fn add_message(
        &self,
        key: &str,
        role: MessageRole,
        content: &str,
    ) -> Result<Message, SessionError> {
        let mut session = self.get(key)?;
        let msg = session.add_message(role, content).clone();
        session.auto_title();
        self.update(&session)?;
        Ok(msg)
    }

    /// Clear all sessions
    pub fn clear(&self) -> Result<usize, SessionError> {
        let count = self.storage.clear()?;
        self.cache.clear();
        Ok(count)
    }

    /// Check if a session exists
    pub fn exists(&self, key: &str) -> bool {
        self.cache.contains_key(key) || self.storage.exists(key).unwrap_or(false)
    }

    /// Get session count
    pub fn count(&self) -> usize {
        self.cache.len()
    }

    /// Get the storage path
    pub fn storage_path(&self) -> &PathBuf {
        &self.storage_path
    }

    /// Compact a session (delegates to compaction module)
    pub fn compact(&self, key: &str) -> Result<compaction::CompactionResult, SessionError> {
        let mut session = self.get(key)?;
        let result = compaction::compact_session(&mut session)?;
        self.update(&session)?;
        Ok(result)
    }

    /// Repair a session (delegates to repair module)
    pub fn repair(&self, key: &str) -> Result<repair::RepairResult, SessionError> {
        let mut session = self.get(key)?;
        let result = repair::repair_session(&mut session)?;
        self.update(&session)?;
        Ok(result)
    }

    /// Export a session (delegates to transcript module)
    pub fn export(
        &self,
        key: &str,
        format: transcript::ExportFormat,
    ) -> Result<String, SessionError> {
        let session = self.get(key)?;
        transcript::export_session(&session, format)
    }

    /// Get a preview of a session (lightweight metadata)
    pub fn preview(&self, key: &str) -> Result<serde_json::Value, SessionError> {
        let session = self.get(key)?;
        Ok(serde_json::json!({
            "session_key": session.key,
            "title": session.title,
            "agent_id": session.agent_id,
            "model": session.model,
            "status": session.status,
            "message_count": session.messages.len(),
            "total_tokens": session.total_tokens,
            "created_at": session.created_at.to_rfc3339(),
            "updated_at": session.updated_at.to_rfc3339(),
            "tags": session.tags,
        }))
    }

    /// Patch a session with partial updates
    pub fn patch(&self, key: &str, updates: &serde_json::Value) -> Result<Session, SessionError> {
        let mut session = self.get(key)?;

        if let Some(title) = updates.get("title").and_then(|v| v.as_str()) {
            session.title = Some(title.to_string());
        }
        if let Some(status) = updates.get("status").and_then(|v| v.as_str()) {
            session.status = match status {
                "active" => SessionStatus::Active,
                "processing" => SessionStatus::Processing,
                "idle" => SessionStatus::Idle,
                "compacted" => SessionStatus::Compacted,
                "archived" => SessionStatus::Archived,
                _ => session.status,
            };
        }
        if let Some(tags) = updates.get("tags").and_then(|v| v.as_array()) {
            session.tags = tags.iter().filter_map(|v| v.as_str().map(String::from)).collect();
        }
        if let Some(model) = updates.get("model").and_then(|v| v.as_str()) {
            session.model = Some(model.to_string());
        }
        if let Some(meta) = updates.get("metadata").and_then(|v| v.as_object()) {
            for (k, v) in meta {
                if let Some(val) = v.as_str() {
                    session.metadata.insert(k.clone(), val.to_string());
                }
            }
        }

        session.updated_at = Utc::now();
        self.update(&session)?;
        Ok(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new("test-agent");
        assert!(!session.key.is_empty());
        assert_eq!(session.agent_id, "test-agent");
        assert_eq!(session.status, SessionStatus::Active);
        assert!(session.messages.is_empty());
    }

    #[test]
    fn test_session_with_key() {
        let session = Session::with_key("my-key", "agent-1");
        assert_eq!(session.key, "my-key");
        assert_eq!(session.agent_id, "agent-1");
    }

    #[test]
    fn test_add_message() {
        let mut session = Session::new("test-agent");
        let msg = session.add_message(MessageRole::User, "Hello");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, "Hello");
        assert_eq!(session.message_count(), 1);
    }

    #[test]
    fn test_auto_title() {
        let mut session = Session::new("test-agent");
        session.add_message(MessageRole::User, "What is Rust?");
        session.auto_title();
        assert_eq!(session.title.as_deref(), Some("What is Rust?"));
    }

    #[test]
    fn test_auto_title_truncation() {
        let mut session = Session::new("test-agent");
        let long_msg = "a".repeat(200);
        session.add_message(MessageRole::User, &long_msg);
        session.auto_title();
        let title = session.title.unwrap();
        assert!(title.ends_with("..."));
        assert!(title.len() <= 84); // 80 chars + "..."
    }

    #[test]
    fn test_session_manager_crud() {
        let dir = tempfile::tempdir().unwrap();
        let manager = SessionManager::new(dir.path().to_path_buf()).unwrap();

        // Create
        let session = manager.create("test-agent").unwrap();
        let key = session.key.clone();

        // Get
        let loaded = manager.get(&key).unwrap();
        assert_eq!(loaded.agent_id, "test-agent");

        // List
        let sessions = manager.list(&SessionFilter::default()).unwrap();
        assert_eq!(sessions.len(), 1);

        // Add message
        let msg = manager.add_message(&key, MessageRole::User, "Hello").unwrap();
        assert_eq!(msg.content, "Hello");

        // Verify message persisted
        let updated = manager.get(&key).unwrap();
        assert_eq!(updated.message_count(), 1);

        // Delete
        manager.delete(&key).unwrap();
        assert!(!manager.exists(&key));
    }

    #[test]
    fn test_session_manager_clear() {
        let dir = tempfile::tempdir().unwrap();
        let manager = SessionManager::new(dir.path().to_path_buf()).unwrap();

        manager.create("agent-1").unwrap();
        manager.create("agent-2").unwrap();
        assert_eq!(manager.count(), 2);

        let cleared = manager.clear().unwrap();
        assert_eq!(cleared, 2);
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_session_filter() {
        let dir = tempfile::tempdir().unwrap();
        let manager = SessionManager::new(dir.path().to_path_buf()).unwrap();

        let mut s1 = manager.create("agent-1").unwrap();
        s1.tags.push("important".to_string());
        manager.update(&s1).unwrap();

        manager.create("agent-2").unwrap();

        // Filter by agent
        let filtered = manager
            .list(&SessionFilter {
                agent_id: Some("agent-1".to_string()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].agent_id, "agent-1");

        // Filter by tag
        let filtered = manager
            .list(&SessionFilter {
                tag: Some("important".to_string()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_session_patch() {
        let dir = tempfile::tempdir().unwrap();
        let manager = SessionManager::new(dir.path().to_path_buf()).unwrap();

        let session = manager.create("agent-1").unwrap();
        let key = session.key.clone();

        let updates = serde_json::json!({
            "title": "My Session",
            "status": "archived",
            "tags": ["tag1", "tag2"],
            "model": "gpt-4"
        });

        let patched = manager.patch(&key, &updates).unwrap();
        assert_eq!(patched.title.as_deref(), Some("My Session"));
        assert_eq!(patched.status, SessionStatus::Archived);
        assert_eq!(patched.tags, vec!["tag1", "tag2"]);
        assert_eq!(patched.model.as_deref(), Some("gpt-4"));
    }
}
