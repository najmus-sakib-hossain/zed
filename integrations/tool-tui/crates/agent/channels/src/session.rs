//! Session management and conversation persistence.
//!
//! Tracks per-chat sessions with metadata, message counts,
//! and timestamps so agents can maintain context across
//! messages.

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// A single conversation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    /// Unique session identifier.
    pub session_id: String,
    /// Channel the session belongs to.
    pub channel_id: String,
    /// Chat / conversation ID on the platform.
    pub chat_id: String,
    /// When the session was created.
    pub created_at: DateTime<Utc>,
    /// Timestamp of the most recent activity.
    pub last_message_at: DateTime<Utc>,
    /// Total messages exchanged in this session.
    pub message_count: usize,
    /// Whether the session is currently active.
    pub active: bool,
    /// Arbitrary key-value metadata.
    pub metadata: HashMap<String, String>,
}

/// Manages sessions across all channels.
///
/// Thread-safe; can be wrapped in `Arc` for shared access.
#[derive(Clone)]
pub struct SessionManager {
    /// session_id → SessionData
    sessions: Arc<DashMap<String, SessionData>>,
    /// (channel_id, chat_id) → session_id for quick lookup
    lookup: Arc<DashMap<(String, String), String>>,
}

impl SessionManager {
    /// Create a new, empty session manager.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            lookup: Arc::new(DashMap::new()),
        }
    }

    /// Create a new session for a channel + chat pair.
    ///
    /// Returns the generated session ID.
    pub fn create_session(&self, channel_id: &str, chat_id: &str) -> String {
        let session_id = format!("sess-{}", uuid::Uuid::new_v4().as_simple());
        let now = Utc::now();
        let data = SessionData {
            session_id: session_id.clone(),
            channel_id: channel_id.to_string(),
            chat_id: chat_id.to_string(),
            created_at: now,
            last_message_at: now,
            message_count: 0,
            active: true,
            metadata: HashMap::new(),
        };
        self.sessions.insert(session_id.clone(), data);
        self.lookup
            .insert((channel_id.to_string(), chat_id.to_string()), session_id.clone());
        session_id
    }

    /// Get a session by ID.
    pub fn get_session(&self, session_id: &str) -> Option<SessionData> {
        self.sessions.get(session_id).map(|r| r.value().clone())
    }

    /// Find an active session for a channel + chat pair.
    pub fn find_session(&self, channel_id: &str, chat_id: &str) -> Option<SessionData> {
        self.lookup
            .get(&(channel_id.to_string(), chat_id.to_string()))
            .and_then(|sid| self.sessions.get(sid.value()).map(|r| r.value().clone()))
    }

    /// Get or create a session for a channel + chat pair.
    pub fn get_or_create(&self, channel_id: &str, chat_id: &str) -> SessionData {
        if let Some(session) = self.find_session(channel_id, chat_id) {
            return session;
        }
        let sid = self.create_session(channel_id, chat_id);
        self.get_session(&sid).expect("just created this session")
    }

    /// Update last activity timestamp and increment message count.
    pub fn record_message(&self, session_id: &str) {
        if let Some(mut entry) = self.sessions.get_mut(session_id) {
            entry.last_message_at = Utc::now();
            entry.message_count += 1;
        }
    }

    /// Set a metadata key on a session.
    pub fn set_metadata(&self, session_id: &str, key: &str, value: &str) {
        if let Some(mut entry) = self.sessions.get_mut(session_id) {
            entry.metadata.insert(key.to_string(), value.to_string());
        }
    }

    /// Get a metadata value from a session.
    pub fn get_metadata(&self, session_id: &str, key: &str) -> Option<String> {
        self.sessions.get(session_id).and_then(|r| r.metadata.get(key).cloned())
    }

    /// Mark a session as inactive (ended).
    pub fn end_session(&self, session_id: &str) {
        if let Some(mut entry) = self.sessions.get_mut(session_id) {
            entry.active = false;
        }
    }

    /// Remove a session entirely.
    pub fn remove_session(&self, session_id: &str) -> Option<SessionData> {
        if let Some((_, data)) = self.sessions.remove(session_id) {
            self.lookup.retain(|_, v| v != session_id);
            Some(data)
        } else {
            None
        }
    }

    /// List all active sessions.
    pub fn list_active(&self) -> Vec<SessionData> {
        self.sessions
            .iter()
            .filter(|r| r.value().active)
            .map(|r| r.value().clone())
            .collect()
    }

    /// List sessions for a specific channel.
    pub fn list_by_channel(&self, channel_id: &str) -> Vec<SessionData> {
        self.sessions
            .iter()
            .filter(|r| r.value().channel_id == channel_id)
            .map(|r| r.value().clone())
            .collect()
    }

    /// Total number of tracked sessions.
    pub fn count(&self) -> usize {
        self.sessions.len()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_get_session() {
        let mgr = SessionManager::new();
        let sid = mgr.create_session("tg", "chat-1");

        let sess = mgr.get_session(&sid).expect("should exist");
        assert_eq!(sess.channel_id, "tg");
        assert_eq!(sess.chat_id, "chat-1");
        assert!(sess.active);
        assert_eq!(sess.message_count, 0);
    }

    #[test]
    fn test_find_session() {
        let mgr = SessionManager::new();
        let sid = mgr.create_session("tg", "chat-1");

        let found = mgr.find_session("tg", "chat-1").expect("should find");
        assert_eq!(found.session_id, sid);

        assert!(mgr.find_session("tg", "chat-99").is_none());
    }

    #[test]
    fn test_get_or_create() {
        let mgr = SessionManager::new();

        let s1 = mgr.get_or_create("tg", "chat-1");
        let s2 = mgr.get_or_create("tg", "chat-1");
        assert_eq!(s1.session_id, s2.session_id);
        assert_eq!(mgr.count(), 1);
    }

    #[test]
    fn test_record_message() {
        let mgr = SessionManager::new();
        let sid = mgr.create_session("tg", "chat-1");
        mgr.record_message(&sid);
        mgr.record_message(&sid);

        let sess = mgr.get_session(&sid).expect("exists");
        assert_eq!(sess.message_count, 2);
    }

    #[test]
    fn test_metadata() {
        let mgr = SessionManager::new();
        let sid = mgr.create_session("tg", "chat-1");
        mgr.set_metadata(&sid, "lang", "en");

        let val = mgr.get_metadata(&sid, "lang");
        assert_eq!(val.as_deref(), Some("en"));

        assert!(mgr.get_metadata(&sid, "missing").is_none());
    }

    #[test]
    fn test_end_session() {
        let mgr = SessionManager::new();
        let sid = mgr.create_session("tg", "chat-1");

        mgr.end_session(&sid);
        let sess = mgr.get_session(&sid).expect("exists");
        assert!(!sess.active);

        assert!(mgr.list_active().is_empty());
    }

    #[test]
    fn test_remove_session() {
        let mgr = SessionManager::new();
        let sid = mgr.create_session("tg", "chat-1");
        assert_eq!(mgr.count(), 1);

        let removed = mgr.remove_session(&sid);
        assert!(removed.is_some());
        assert_eq!(mgr.count(), 0);
    }

    #[test]
    fn test_list_by_channel() {
        let mgr = SessionManager::new();
        mgr.create_session("tg", "c1");
        mgr.create_session("tg", "c2");
        mgr.create_session("discord", "c3");

        let tg = mgr.list_by_channel("tg");
        assert_eq!(tg.len(), 2);
    }
}
