//! Thread and context management for conversations.
//!
//! Tracks threads (reply chains), parent messages, and
//! per-conversation reply modes across channels.

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Reply behaviour for outgoing messages.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReplyMode {
    /// Never quote/reply.
    Off,
    /// Reply to the first message in a conversation.
    First,
    /// Reply to every incoming message.
    #[default]
    All,
}

/// Contextual threading information attached to a message.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThreadContext {
    /// Thread / topic ID (platform-specific).
    pub thread_id: Option<String>,
    /// Parent message that started the thread.
    pub parent_message_id: Option<String>,
    /// Specific message being replied to.
    pub reply_to_id: Option<String>,
}

/// Internal bookkeeping for a single thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadInfo {
    /// Unique thread identifier.
    pub thread_id: String,
    /// ID of the channel this thread belongs to.
    pub channel_id: String,
    /// Chat / conversation the thread lives in.
    pub chat_id: String,
    /// Number of messages in the thread.
    pub message_count: usize,
    /// Unique participant IDs.
    pub participants: Vec<String>,
    /// When thread was created.
    pub created_at: DateTime<Utc>,
    /// Most recent activity.
    pub last_activity_at: DateTime<Utc>,
}

/// Thread manager — tracks open threads across channels.
///
/// Thread-safe via `DashMap`; clone-friendly.
#[derive(Clone)]
pub struct ThreadManager {
    threads: Arc<DashMap<String, ThreadInfo>>,
    /// Maps `chat_id` → latest `thread_id` for quick lookup.
    chat_threads: Arc<DashMap<String, String>>,
}

impl ThreadManager {
    /// Create a new, empty thread manager.
    pub fn new() -> Self {
        Self {
            threads: Arc::new(DashMap::new()),
            chat_threads: Arc::new(DashMap::new()),
        }
    }

    /// Create a new thread rooted at `parent_id`.
    ///
    /// Returns the generated thread ID.
    pub fn create_thread(&self, channel_id: &str, chat_id: &str, parent_id: &str) -> String {
        let thread_id = format!("thread-{}", uuid::Uuid::new_v4().as_simple());
        let now = Utc::now();
        let info = ThreadInfo {
            thread_id: thread_id.clone(),
            channel_id: channel_id.to_string(),
            chat_id: chat_id.to_string(),
            message_count: 1,
            participants: Vec::new(),
            created_at: now,
            last_activity_at: now,
        };
        self.threads.insert(thread_id.clone(), info);
        self.chat_threads.insert(chat_id.to_string(), thread_id.clone());
        let _ = parent_id; // referenced for future persistence
        thread_id
    }

    /// Retrieve a thread snapshot.
    pub fn get_thread(&self, thread_id: &str) -> Option<ThreadInfo> {
        self.threads.get(thread_id).map(|r| r.value().clone())
    }

    /// Get the latest thread for a given chat.
    pub fn get_chat_thread(&self, chat_id: &str) -> Option<ThreadInfo> {
        self.chat_threads
            .get(chat_id)
            .and_then(|tid| self.threads.get(tid.value()).map(|r| r.clone()))
    }

    /// Record a new message in an existing thread.
    pub fn add_message(&self, thread_id: &str, sender_id: &str) {
        if let Some(mut entry) = self.threads.get_mut(thread_id) {
            entry.message_count += 1;
            entry.last_activity_at = Utc::now();
            if !entry.participants.contains(&sender_id.to_string()) {
                entry.participants.push(sender_id.to_string());
            }
        }
    }

    /// List all threads for a channel.
    pub fn list_threads(&self, channel_id: &str) -> Vec<ThreadInfo> {
        self.threads
            .iter()
            .filter(|r| r.value().channel_id == channel_id)
            .map(|r| r.value().clone())
            .collect()
    }

    /// Remove a thread from tracking.
    pub fn remove_thread(&self, thread_id: &str) -> Option<ThreadInfo> {
        if let Some((_, info)) = self.threads.remove(thread_id) {
            // Clean up chat_threads mapping
            self.chat_threads.retain(|_, tid| tid != thread_id);
            Some(info)
        } else {
            None
        }
    }

    /// Number of tracked threads.
    pub fn count(&self) -> usize {
        self.threads.len()
    }
}

impl Default for ThreadManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Determine the reply-to ID for an outgoing message based on
/// the configured reply mode and the conversation context.
pub fn resolve_reply_target(mode: ReplyMode, ctx: &ThreadContext) -> Option<String> {
    match mode {
        ReplyMode::Off => None,
        ReplyMode::First => ctx.parent_message_id.clone(),
        ReplyMode::All => ctx.reply_to_id.clone().or_else(|| ctx.parent_message_id.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_get_thread() {
        let mgr = ThreadManager::new();
        let tid = mgr.create_thread("tg", "chat-1", "msg-0");

        let info = mgr.get_thread(&tid).expect("thread should exist");
        assert_eq!(info.channel_id, "tg");
        assert_eq!(info.chat_id, "chat-1");
        assert_eq!(info.message_count, 1);
    }

    #[test]
    fn test_add_message() {
        let mgr = ThreadManager::new();
        let tid = mgr.create_thread("tg", "chat-1", "msg-0");
        mgr.add_message(&tid, "alice");
        mgr.add_message(&tid, "bob");
        mgr.add_message(&tid, "alice");

        let info = mgr.get_thread(&tid).expect("thread");
        assert_eq!(info.message_count, 4); // 1 initial + 3 adds
        assert_eq!(info.participants.len(), 2);
    }

    #[test]
    fn test_chat_thread_lookup() {
        let mgr = ThreadManager::new();
        let tid = mgr.create_thread("tg", "chat-42", "msg-0");

        let info = mgr.get_chat_thread("chat-42").expect("exists");
        assert_eq!(info.thread_id, tid);
    }

    #[test]
    fn test_list_threads_per_channel() {
        let mgr = ThreadManager::new();
        mgr.create_thread("tg", "c1", "m1");
        mgr.create_thread("tg", "c2", "m2");
        mgr.create_thread("discord", "c3", "m3");

        let tg = mgr.list_threads("tg");
        assert_eq!(tg.len(), 2);
        let dc = mgr.list_threads("discord");
        assert_eq!(dc.len(), 1);
    }

    #[test]
    fn test_remove_thread() {
        let mgr = ThreadManager::new();
        let tid = mgr.create_thread("tg", "c1", "m1");
        assert_eq!(mgr.count(), 1);

        let removed = mgr.remove_thread(&tid);
        assert!(removed.is_some());
        assert_eq!(mgr.count(), 0);
    }

    #[test]
    fn test_resolve_reply_target() {
        let ctx = ThreadContext {
            thread_id: Some("t1".into()),
            parent_message_id: Some("p1".into()),
            reply_to_id: Some("r1".into()),
        };

        assert_eq!(resolve_reply_target(ReplyMode::Off, &ctx), None);
        assert_eq!(resolve_reply_target(ReplyMode::First, &ctx), Some("p1".into()));
        assert_eq!(resolve_reply_target(ReplyMode::All, &ctx), Some("r1".into()));

        let ctx_no_reply = ThreadContext {
            thread_id: None,
            parent_message_id: Some("p1".into()),
            reply_to_id: None,
        };
        assert_eq!(resolve_reply_target(ReplyMode::All, &ctx_no_reply), Some("p1".into()));
    }
}
