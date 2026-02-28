//! Allowlist/denylist for channel access control.
//! Adapted from OpenClaw's allowlist-match.ts

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Access control rules for a channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControl {
    /// Allowed user IDs (empty = allow all)
    #[serde(default)]
    pub allowed_users: HashSet<String>,
    /// Denied user IDs
    #[serde(default)]
    pub denied_users: HashSet<String>,
    /// Allowed group/chat IDs (empty = allow all)
    #[serde(default)]
    pub allowed_groups: HashSet<String>,
    /// Denied group/chat IDs
    #[serde(default)]
    pub denied_groups: HashSet<String>,
    /// Require mention in groups (mention gating)
    #[serde(default)]
    pub require_mention_in_groups: bool,
    /// DM pairing required
    #[serde(default)]
    pub require_dm_pairing: bool,
}

impl Default for AccessControl {
    fn default() -> Self {
        Self {
            allowed_users: HashSet::new(),
            denied_users: HashSet::new(),
            allowed_groups: HashSet::new(),
            denied_groups: HashSet::new(),
            require_mention_in_groups: true,
            require_dm_pairing: false,
        }
    }
}

impl AccessControl {
    /// Check if a user is allowed to interact
    pub fn is_user_allowed(&self, user_id: &str) -> bool {
        // Check deny list first
        if self.denied_users.contains(user_id) {
            return false;
        }

        // If allowlist is empty, allow all (except denied)
        if self.allowed_users.is_empty() {
            return true;
        }

        // Check allowlist
        self.allowed_users.contains(user_id)
    }

    /// Check if a group is allowed
    pub fn is_group_allowed(&self, group_id: &str) -> bool {
        if self.denied_groups.contains(group_id) {
            return false;
        }

        if self.allowed_groups.is_empty() {
            return true;
        }

        self.allowed_groups.contains(group_id)
    }

    /// Check if a message should be processed (combining user + group checks)
    pub fn should_process(
        &self,
        user_id: &str,
        group_id: Option<&str>,
        is_mentioned: bool,
    ) -> bool {
        // User-level check
        if !self.is_user_allowed(user_id) {
            return false;
        }

        // Group-level check
        if let Some(gid) = group_id {
            if !self.is_group_allowed(gid) {
                return false;
            }

            // Mention gating for groups
            if self.require_mention_in_groups && !is_mentioned {
                return false;
            }
        }

        true
    }
}

/// DM pairing state tracker
#[derive(Debug, Clone)]
pub struct DmPairingManager {
    /// Map of pairing code -> user ID
    pending_codes: dashmap::DashMap<String, PairingRequest>,
    /// Set of paired user IDs
    paired_users: dashmap::DashMap<String, PairedUser>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingRequest {
    pub code: String,
    pub channel: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairedUser {
    pub user_id: String,
    pub channel: String,
    pub paired_at: chrono::DateTime<chrono::Utc>,
}

impl DmPairingManager {
    pub fn new() -> Self {
        Self {
            pending_codes: dashmap::DashMap::new(),
            paired_users: dashmap::DashMap::new(),
        }
    }

    /// Generate a new pairing code
    pub fn generate_code(&self, channel: &str) -> String {
        let code = format!("{:06}", rand_code());
        let now = chrono::Utc::now();
        self.pending_codes.insert(
            code.clone(),
            PairingRequest {
                code: code.clone(),
                channel: channel.into(),
                created_at: now,
                expires_at: now + chrono::Duration::minutes(10),
            },
        );
        code
    }

    /// Verify a pairing code and pair the user
    pub fn verify_code(&self, code: &str, user_id: &str) -> bool {
        if let Some((_, request)) = self.pending_codes.remove(code) {
            let now = chrono::Utc::now();
            if now < request.expires_at {
                self.paired_users.insert(
                    user_id.to_string(),
                    PairedUser {
                        user_id: user_id.to_string(),
                        channel: request.channel,
                        paired_at: now,
                    },
                );
                return true;
            }
        }
        false
    }

    /// Check if a user is paired
    pub fn is_paired(&self, user_id: &str) -> bool {
        self.paired_users.contains_key(user_id)
    }

    /// Clean up expired pairing codes
    pub fn cleanup_expired(&self) {
        let now = chrono::Utc::now();
        self.pending_codes.retain(|_, v| v.expires_at > now);
    }
}

impl Default for DmPairingManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate a simple random 6-digit code
fn rand_code() -> u32 {
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    seed % 1_000_000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allowlist_empty_allows_all() {
        let acl = AccessControl::default();
        assert!(acl.is_user_allowed("anyone"));
    }

    #[test]
    fn test_allowlist_with_users() {
        let mut acl = AccessControl::default();
        acl.allowed_users.insert("user1".into());
        acl.allowed_users.insert("user2".into());

        assert!(acl.is_user_allowed("user1"));
        assert!(!acl.is_user_allowed("user3"));
    }

    #[test]
    fn test_denylist_overrides() {
        let mut acl = AccessControl::default();
        acl.denied_users.insert("banned".into());

        assert!(!acl.is_user_allowed("banned"));
        assert!(acl.is_user_allowed("others"));
    }

    #[test]
    fn test_mention_gating() {
        let acl = AccessControl {
            require_mention_in_groups: true,
            ..Default::default()
        };

        // DM without group - should process
        assert!(acl.should_process("user1", None, false));

        // Group without mention - should NOT process
        assert!(!acl.should_process("user1", Some("group1"), false));

        // Group with mention - should process
        assert!(acl.should_process("user1", Some("group1"), true));
    }

    #[test]
    fn test_dm_pairing() {
        let manager = DmPairingManager::new();
        let code = manager.generate_code("telegram");

        assert!(!manager.is_paired("user1"));
        assert!(manager.verify_code(&code, "user1"));
        assert!(manager.is_paired("user1"));
    }
}
