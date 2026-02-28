//! Directory service — contacts, groups, and name resolution.
//!
//! Provides a channel-agnostic directory of users, groups,
//! and channels so other subsystems can resolve IDs to names.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Kind of directory entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DirectoryEntryKind {
    /// Individual user.
    User,
    /// Group conversation.
    Group,
    /// Broadcast channel (one-to-many).
    Channel,
}

/// A single entry in the directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryEntry {
    /// Entry kind.
    pub kind: DirectoryEntryKind,
    /// Platform-specific unique identifier.
    pub id: String,
    /// Display name.
    pub name: Option<String>,
    /// Handle / username (e.g. `@alice`).
    pub handle: Option<String>,
    /// Avatar / profile picture URL.
    pub avatar_url: Option<String>,
    /// Which channel this entry belongs to.
    pub channel_id: Option<String>,
}

/// In-memory directory of contacts and groups.
///
/// Thread-safe via `DashMap`. Intended to be wrapped in an
/// `Arc` and shared across tasks.
#[derive(Clone)]
pub struct Directory {
    entries: Arc<DashMap<String, DirectoryEntry>>,
}

impl Directory {
    /// Create a new, empty directory.
    pub fn new() -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
        }
    }

    /// Insert or update a directory entry.
    pub fn add_entry(&self, entry: DirectoryEntry) {
        self.entries.insert(entry.id.clone(), entry);
    }

    /// Look up an entry by its ID.
    pub fn get_entry(&self, id: &str) -> Option<DirectoryEntry> {
        self.entries.get(id).map(|r| r.value().clone())
    }

    /// Remove an entry.
    pub fn remove_entry(&self, id: &str) -> Option<DirectoryEntry> {
        self.entries.remove(id).map(|(_, v)| v)
    }

    /// Full-text search over name and handle fields (case-insensitive).
    pub fn search(&self, query: &str) -> Vec<DirectoryEntry> {
        let q = query.to_lowercase();
        self.entries
            .iter()
            .filter(|r| {
                let e = r.value();
                e.name.as_deref().map(|n| n.to_lowercase().contains(&q)).unwrap_or(false)
                    || e.handle.as_deref().map(|h| h.to_lowercase().contains(&q)).unwrap_or(false)
                    || e.id.to_lowercase().contains(&q)
            })
            .map(|r| r.value().clone())
            .collect()
    }

    /// Resolve an ID to a human-readable name.
    ///
    /// Falls back to handle, then to the raw ID.
    pub fn resolve_name(&self, id: &str) -> String {
        self.entries
            .get(id)
            .map(|r| {
                r.value()
                    .name
                    .clone()
                    .or_else(|| r.value().handle.clone())
                    .unwrap_or_else(|| id.to_string())
            })
            .unwrap_or_else(|| id.to_string())
    }

    /// List all entries of a particular kind.
    pub fn list_by_kind(&self, kind: DirectoryEntryKind) -> Vec<DirectoryEntry> {
        self.entries
            .iter()
            .filter(|r| r.value().kind == kind)
            .map(|r| r.value().clone())
            .collect()
    }

    /// List all entries belonging to a specific channel.
    pub fn list_by_channel(&self, channel_id: &str) -> Vec<DirectoryEntry> {
        self.entries
            .iter()
            .filter(|r| r.value().channel_id.as_deref() == Some(channel_id))
            .map(|r| r.value().clone())
            .collect()
    }

    /// Total number of entries.
    pub fn count(&self) -> usize {
        self.entries.len()
    }
}

impl Default for Directory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alice() -> DirectoryEntry {
        DirectoryEntry {
            kind: DirectoryEntryKind::User,
            id: "u1".into(),
            name: Some("Alice".into()),
            handle: Some("@alice".into()),
            avatar_url: None,
            channel_id: Some("tg".into()),
        }
    }

    fn bob() -> DirectoryEntry {
        DirectoryEntry {
            kind: DirectoryEntryKind::User,
            id: "u2".into(),
            name: Some("Bob".into()),
            handle: None,
            avatar_url: None,
            channel_id: Some("discord".into()),
        }
    }

    fn devs_group() -> DirectoryEntry {
        DirectoryEntry {
            kind: DirectoryEntryKind::Group,
            id: "g1".into(),
            name: Some("Developers".into()),
            handle: None,
            avatar_url: None,
            channel_id: Some("slack".into()),
        }
    }

    #[test]
    fn test_add_and_get() {
        let dir = Directory::new();
        dir.add_entry(alice());

        let entry = dir.get_entry("u1").expect("should exist");
        assert_eq!(entry.name.as_deref(), Some("Alice"));
    }

    #[test]
    fn test_remove() {
        let dir = Directory::new();
        dir.add_entry(alice());
        let removed = dir.remove_entry("u1");
        assert!(removed.is_some());
        assert!(dir.get_entry("u1").is_none());
    }

    #[test]
    fn test_search() {
        let dir = Directory::new();
        dir.add_entry(alice());
        dir.add_entry(bob());
        dir.add_entry(devs_group());

        let results = dir.search("ali");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "u1");

        let results = dir.search("dev");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, DirectoryEntryKind::Group);
    }

    #[test]
    fn test_resolve_name() {
        let dir = Directory::new();
        dir.add_entry(alice());

        assert_eq!(dir.resolve_name("u1"), "Alice");
        assert_eq!(dir.resolve_name("unknown"), "unknown");
    }

    #[test]
    fn test_list_by_kind() {
        let dir = Directory::new();
        dir.add_entry(alice());
        dir.add_entry(bob());
        dir.add_entry(devs_group());

        let users = dir.list_by_kind(DirectoryEntryKind::User);
        assert_eq!(users.len(), 2);
        let groups = dir.list_by_kind(DirectoryEntryKind::Group);
        assert_eq!(groups.len(), 1);
    }

    #[test]
    fn test_list_by_channel() {
        let dir = Directory::new();
        dir.add_entry(alice());
        dir.add_entry(bob());

        let tg = dir.list_by_channel("tg");
        assert_eq!(tg.len(), 1);
        assert_eq!(tg[0].id, "u1");
    }

    #[test]
    fn test_count() {
        let dir = Directory::new();
        assert_eq!(dir.count(), 0);
        dir.add_entry(alice());
        assert_eq!(dir.count(), 1);
    }
}
