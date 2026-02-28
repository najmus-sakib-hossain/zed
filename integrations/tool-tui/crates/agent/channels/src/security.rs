//! Security policies and access control for channels.
//!
//! Provides DM policies, allowlists, blocklists, and permission
//! checking utilities that complement the existing `allowlist` module.

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Direct message policy mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DmPolicy {
    /// User must be explicitly allowed before DM is accepted.
    #[default]
    Explicit,
    /// Any user can initiate a DM.
    Implicit,
    /// User must send a heartbeat / pairing code first.
    Heartbeat,
}

/// Security policy for a channel instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// DM acceptance policy.
    #[serde(default)]
    pub dm_policy: DmPolicy,
    /// Explicitly allowed user/chat identifiers.
    #[serde(default)]
    pub allowlist: Vec<String>,
    /// Explicitly blocked user/chat identifiers.
    #[serde(default)]
    pub blocklist: Vec<String>,
    /// Maximum messages per minute per user (rate limit).
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_minute: u32,
    /// Whether to require mention in group chats.
    #[serde(default)]
    pub require_mention_in_groups: bool,
}

fn default_rate_limit() -> u32 {
    60
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            dm_policy: DmPolicy::default(),
            allowlist: Vec::new(),
            blocklist: Vec::new(),
            rate_limit_per_minute: default_rate_limit(),
            require_mention_in_groups: true,
        }
    }
}

/// Runtime context for a permission check.
#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// Channel the message arrived on.
    pub channel_id: String,
    /// Sender identifier.
    pub sender_id: String,
    /// Whether the message is in a group chat.
    pub is_group: bool,
    /// Whether the agent was mentioned in the message.
    pub is_mentioned: bool,
}

/// Check whether a given security context is permitted
/// under the supplied policy.
///
/// Returns `Ok(true)` if allowed, `Ok(false)` if silently
/// rejected, or `Err` if the rejection should be reported.
pub fn check_permission(policy: &SecurityPolicy, ctx: &SecurityContext) -> Result<bool> {
    // Blocklist always wins.
    let normalized = normalize_target(&ctx.sender_id);
    if policy.blocklist.iter().any(|b| normalize_target(b) == normalized) {
        bail!("User {} is blocked on channel {}", ctx.sender_id, ctx.channel_id);
    }

    // Groups: check mention requirement.
    if ctx.is_group && policy.require_mention_in_groups && !ctx.is_mentioned {
        return Ok(false);
    }

    // DM policy handling.
    if !ctx.is_group {
        match policy.dm_policy {
            DmPolicy::Explicit => {
                if !is_allowed(policy, &ctx.sender_id) {
                    return Ok(false);
                }
            }
            DmPolicy::Heartbeat => {
                // Heartbeat pairing is handled externally; just
                // delegate to allowlist check.
                if !is_allowed(policy, &ctx.sender_id) {
                    return Ok(false);
                }
            }
            DmPolicy::Implicit => {
                // Implicit = anyone can DM.
            }
        }
    }

    Ok(true)
}

/// Check whether `target` appears in the policy's allowlist.
///
/// An empty allowlist means "allow everyone".
pub fn is_allowed(policy: &SecurityPolicy, target: &str) -> bool {
    if policy.allowlist.is_empty() {
        return true;
    }
    let normalized = normalize_target(target);
    policy.allowlist.iter().any(|a| normalize_target(a) == normalized)
}

/// Normalize a target identifier for comparison.
///
/// Strips leading `@`, trims whitespace, and lowercases.
pub fn normalize_target(target: &str) -> String {
    target.trim().trim_start_matches('@').to_lowercase()
}

/// Build a `HashSet` of normalized allowlist entries for
/// efficient batch lookups.
pub fn allowlist_set(policy: &SecurityPolicy) -> HashSet<String> {
    policy.allowlist.iter().map(|s| normalize_target(s)).collect()
}

/// Build a `HashSet` of normalized blocklist entries.
pub fn blocklist_set(policy: &SecurityPolicy) -> HashSet<String> {
    policy.blocklist.iter().map(|s| normalize_target(s)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_target() {
        assert_eq!(normalize_target("@Alice"), "alice");
        assert_eq!(normalize_target("  Bob  "), "bob");
        assert_eq!(normalize_target("@USER"), "user");
    }

    #[test]
    fn test_is_allowed_empty_allowlist() {
        let policy = SecurityPolicy::default();
        assert!(is_allowed(&policy, "anyone"));
    }

    #[test]
    fn test_is_allowed_with_entries() {
        let policy = SecurityPolicy {
            allowlist: vec!["alice".into(), "bob".into()],
            ..Default::default()
        };
        assert!(is_allowed(&policy, "alice"));
        assert!(is_allowed(&policy, "@Alice"));
        assert!(!is_allowed(&policy, "charlie"));
    }

    #[test]
    fn test_check_permission_blocked() {
        let policy = SecurityPolicy {
            blocklist: vec!["bad_user".into()],
            ..Default::default()
        };
        let ctx = SecurityContext {
            channel_id: "tg".into(),
            sender_id: "bad_user".into(),
            is_group: false,
            is_mentioned: false,
        };
        let result = check_permission(&policy, &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_check_permission_dm_explicit_allowed() {
        let policy = SecurityPolicy {
            dm_policy: DmPolicy::Explicit,
            allowlist: vec!["alice".into()],
            ..Default::default()
        };
        let ctx = SecurityContext {
            channel_id: "tg".into(),
            sender_id: "alice".into(),
            is_group: false,
            is_mentioned: false,
        };
        assert!(check_permission(&policy, &ctx).unwrap_or(false));
    }

    #[test]
    fn test_check_permission_dm_explicit_denied() {
        let policy = SecurityPolicy {
            dm_policy: DmPolicy::Explicit,
            allowlist: vec!["alice".into()],
            ..Default::default()
        };
        let ctx = SecurityContext {
            channel_id: "tg".into(),
            sender_id: "charlie".into(),
            is_group: false,
            is_mentioned: false,
        };
        assert!(!check_permission(&policy, &ctx).unwrap_or(true));
    }

    #[test]
    fn test_check_permission_dm_implicit() {
        let policy = SecurityPolicy {
            dm_policy: DmPolicy::Implicit,
            ..Default::default()
        };
        let ctx = SecurityContext {
            channel_id: "tg".into(),
            sender_id: "anyone".into(),
            is_group: false,
            is_mentioned: false,
        };
        assert!(check_permission(&policy, &ctx).unwrap_or(false));
    }

    #[test]
    fn test_check_permission_group_mention_required() {
        let policy = SecurityPolicy {
            require_mention_in_groups: true,
            ..Default::default()
        };
        let ctx_no_mention = SecurityContext {
            channel_id: "tg".into(),
            sender_id: "user".into(),
            is_group: true,
            is_mentioned: false,
        };
        assert!(!check_permission(&policy, &ctx_no_mention).unwrap_or(true));

        let ctx_mention = SecurityContext {
            channel_id: "tg".into(),
            sender_id: "user".into(),
            is_group: true,
            is_mentioned: true,
        };
        assert!(check_permission(&policy, &ctx_mention).unwrap_or(false));
    }

    #[test]
    fn test_sets() {
        let policy = SecurityPolicy {
            allowlist: vec!["@Alice".into(), "Bob".into()],
            blocklist: vec!["Charlie".into()],
            ..Default::default()
        };
        let allow = allowlist_set(&policy);
        assert!(allow.contains("alice"));
        assert!(allow.contains("bob"));
        let block = blocklist_set(&policy);
        assert!(block.contains("charlie"));
    }
}
