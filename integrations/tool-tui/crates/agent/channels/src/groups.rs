//! Group operations and member management.
//!
//! Tracks groups, their members, admins, and provides
//! helpers for group lifecycle operations.

use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Information about a group / chat room.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupInfo {
    /// Platform-specific group ID.
    pub id: String,
    /// Group display name.
    pub name: String,
    /// Channel type (e.g. `"telegram"`, `"discord"`).
    pub channel_id: String,
    /// Ordered member IDs.
    pub members: Vec<String>,
    /// Admin / owner member IDs.
    pub admins: Vec<String>,
    /// Group description.
    pub description: Option<String>,
    /// When the group was first tracked.
    pub created_at: DateTime<Utc>,
    /// Last activity.
    pub last_activity_at: DateTime<Utc>,
}

/// Manages group membership across channels.
#[derive(Clone)]
pub struct GroupManager {
    groups: Arc<DashMap<String, GroupInfo>>,
}

impl GroupManager {
    /// Create a new, empty group manager.
    pub fn new() -> Self {
        Self {
            groups: Arc::new(DashMap::new()),
        }
    }

    /// Add or update a group.
    pub fn add_group(&self, info: GroupInfo) {
        self.groups.insert(info.id.clone(), info);
    }

    /// Get group info by ID.
    pub fn get_group(&self, id: &str) -> Option<GroupInfo> {
        self.groups.get(id).map(|r| r.value().clone())
    }

    /// Remove a group from tracking.
    pub fn remove_group(&self, id: &str) -> Option<GroupInfo> {
        self.groups.remove(id).map(|(_, v)| v)
    }

    /// Add a member to a group (no-op if already present).
    pub fn add_member(&self, group_id: &str, user_id: &str) -> Result<()> {
        let mut entry = self
            .groups
            .get_mut(group_id)
            .ok_or_else(|| anyhow::anyhow!("Group not found: {}", group_id))?;
        let group = entry.value_mut();
        if !group.members.contains(&user_id.to_string()) {
            group.members.push(user_id.to_string());
            group.last_activity_at = Utc::now();
        }
        Ok(())
    }

    /// Remove a member from a group.
    pub fn remove_member(&self, group_id: &str, user_id: &str) -> Result<()> {
        let mut entry = self
            .groups
            .get_mut(group_id)
            .ok_or_else(|| anyhow::anyhow!("Group not found: {}", group_id))?;
        let group = entry.value_mut();
        group.members.retain(|m| m != user_id);
        group.admins.retain(|a| a != user_id);
        group.last_activity_at = Utc::now();
        Ok(())
    }

    /// Promote a member to admin.
    pub fn promote_admin(&self, group_id: &str, user_id: &str) -> Result<()> {
        let mut entry = self
            .groups
            .get_mut(group_id)
            .ok_or_else(|| anyhow::anyhow!("Group not found: {}", group_id))?;
        let group = entry.value_mut();
        if !group.members.contains(&user_id.to_string()) {
            bail!("User '{}' is not a member of group '{}'", user_id, group_id);
        }
        if !group.admins.contains(&user_id.to_string()) {
            group.admins.push(user_id.to_string());
        }
        Ok(())
    }

    /// Demote an admin to regular member.
    pub fn demote_admin(&self, group_id: &str, user_id: &str) -> Result<()> {
        let mut entry = self
            .groups
            .get_mut(group_id)
            .ok_or_else(|| anyhow::anyhow!("Group not found: {}", group_id))?;
        entry.value_mut().admins.retain(|a| a != user_id);
        Ok(())
    }

    /// Check if a user is a member of a group.
    pub fn is_member(&self, group_id: &str, user_id: &str) -> bool {
        self.groups
            .get(group_id)
            .map(|r| r.members.contains(&user_id.to_string()))
            .unwrap_or(false)
    }

    /// Check if a user is an admin of a group.
    pub fn is_admin(&self, group_id: &str, user_id: &str) -> bool {
        self.groups
            .get(group_id)
            .map(|r| r.admins.contains(&user_id.to_string()))
            .unwrap_or(false)
    }

    /// Get member count for a group.
    pub fn member_count(&self, group_id: &str) -> usize {
        self.groups.get(group_id).map(|r| r.members.len()).unwrap_or(0)
    }

    /// List all groups for a channel.
    pub fn list_by_channel(&self, channel_id: &str) -> Vec<GroupInfo> {
        self.groups
            .iter()
            .filter(|r| r.value().channel_id == channel_id)
            .map(|r| r.value().clone())
            .collect()
    }

    /// Total number of tracked groups.
    pub fn count(&self) -> usize {
        self.groups.len()
    }
}

impl Default for GroupManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_group() -> GroupInfo {
        GroupInfo {
            id: "g1".into(),
            name: "Developers".into(),
            channel_id: "tg".into(),
            members: vec!["alice".into(), "bob".into()],
            admins: vec!["alice".into()],
            description: Some("Dev group".into()),
            created_at: Utc::now(),
            last_activity_at: Utc::now(),
        }
    }

    #[test]
    fn test_add_and_get_group() {
        let mgr = GroupManager::new();
        mgr.add_group(sample_group());

        let g = mgr.get_group("g1").expect("should exist");
        assert_eq!(g.name, "Developers");
        assert_eq!(g.members.len(), 2);
    }

    #[test]
    fn test_add_member() {
        let mgr = GroupManager::new();
        mgr.add_group(sample_group());

        mgr.add_member("g1", "charlie").expect("should work");
        let g = mgr.get_group("g1").expect("exists");
        assert_eq!(g.members.len(), 3);
        assert!(g.members.contains(&"charlie".to_string()));
    }

    #[test]
    fn test_add_member_duplicate() {
        let mgr = GroupManager::new();
        mgr.add_group(sample_group());

        mgr.add_member("g1", "alice").expect("no-op");
        let g = mgr.get_group("g1").expect("exists");
        assert_eq!(g.members.len(), 2); // no duplicate
    }

    #[test]
    fn test_remove_member() {
        let mgr = GroupManager::new();
        mgr.add_group(sample_group());

        mgr.remove_member("g1", "bob").expect("should work");
        let g = mgr.get_group("g1").expect("exists");
        assert_eq!(g.members.len(), 1);
        assert!(!g.members.contains(&"bob".to_string()));
    }

    #[test]
    fn test_promote_and_demote() {
        let mgr = GroupManager::new();
        mgr.add_group(sample_group());

        mgr.promote_admin("g1", "bob").expect("promote");
        assert!(mgr.is_admin("g1", "bob"));

        mgr.demote_admin("g1", "bob").expect("demote");
        assert!(!mgr.is_admin("g1", "bob"));
    }

    #[test]
    fn test_promote_non_member() {
        let mgr = GroupManager::new();
        mgr.add_group(sample_group());

        let result = mgr.promote_admin("g1", "stranger");
        assert!(result.is_err());
    }

    #[test]
    fn test_is_member_and_admin() {
        let mgr = GroupManager::new();
        mgr.add_group(sample_group());

        assert!(mgr.is_member("g1", "alice"));
        assert!(mgr.is_admin("g1", "alice"));
        assert!(mgr.is_member("g1", "bob"));
        assert!(!mgr.is_admin("g1", "bob"));
        assert!(!mgr.is_member("g1", "nobody"));
    }

    #[test]
    fn test_member_count() {
        let mgr = GroupManager::new();
        mgr.add_group(sample_group());
        assert_eq!(mgr.member_count("g1"), 2);
        assert_eq!(mgr.member_count("nonexistent"), 0);
    }

    #[test]
    fn test_list_by_channel() {
        let mgr = GroupManager::new();
        mgr.add_group(sample_group());
        mgr.add_group(GroupInfo {
            id: "g2".into(),
            name: "General".into(),
            channel_id: "discord".into(),
            members: vec![],
            admins: vec![],
            description: None,
            created_at: Utc::now(),
            last_activity_at: Utc::now(),
        });

        let tg = mgr.list_by_channel("tg");
        assert_eq!(tg.len(), 1);
        let dc = mgr.list_by_channel("discord");
        assert_eq!(dc.len(), 1);
    }

    #[test]
    fn test_remove_group() {
        let mgr = GroupManager::new();
        mgr.add_group(sample_group());
        assert_eq!(mgr.count(), 1);

        let removed = mgr.remove_group("g1");
        assert!(removed.is_some());
        assert_eq!(mgr.count(), 0);
    }

    #[test]
    fn test_add_member_nonexistent_group() {
        let mgr = GroupManager::new();
        let result = mgr.add_member("nope", "user");
        assert!(result.is_err());
    }
}
