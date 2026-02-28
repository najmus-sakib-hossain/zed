//! # Session Repair
//!
//! Detects and fixes corrupted or inconsistent session data.
//! Handles: missing fields, invalid timestamps, orphaned tool calls,
//! duplicate message IDs, and structural integrity issues.

use std::collections::HashSet;

use chrono::Utc;

use super::{MessageRole, Session, SessionError, SessionStatus};

/// Result of a repair operation
#[derive(Debug, Clone)]
pub struct RepairResult {
    /// Whether any repairs were needed
    pub repairs_needed: bool,
    /// Number of issues found
    pub issues_found: usize,
    /// Number of issues fixed
    pub issues_fixed: usize,
    /// Description of each repair performed
    pub repairs: Vec<RepairAction>,
}

/// A single repair action performed
#[derive(Debug, Clone)]
pub struct RepairAction {
    /// Type of repair
    pub repair_type: RepairType,
    /// Human-readable description
    pub description: String,
}

/// Types of repairs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepairType {
    /// Regenerated missing message ID
    MissingMessageId,
    /// Fixed invalid timestamp
    InvalidTimestamp,
    /// Fixed duplicate message ID
    DuplicateMessageId,
    /// Removed orphaned tool response (no matching call)
    OrphanedToolResponse,
    /// Fixed empty content
    EmptyContent,
    /// Fixed session timestamps consistency
    TimestampConsistency,
    /// Fixed session status
    StatusRepair,
    /// Added missing session key
    MissingSessionKey,
}

/// Repair a session, fixing any detected issues
pub fn repair_session(session: &mut Session) -> Result<RepairResult, SessionError> {
    let mut repairs = Vec::new();

    // Repair 1: Fix missing session key
    if session.key.is_empty() {
        session.key = uuid::Uuid::new_v4().to_string();
        repairs.push(RepairAction {
            repair_type: RepairType::MissingSessionKey,
            description: format!("Generated new session key: {}", session.key),
        });
    }

    // Repair 2: Fix missing message IDs
    for msg in &mut session.messages {
        if msg.id.is_empty() {
            msg.id = uuid::Uuid::new_v4().to_string();
            repairs.push(RepairAction {
                repair_type: RepairType::MissingMessageId,
                description: format!("Generated ID for message: {}", msg.id),
            });
        }
    }

    // Repair 3: Fix duplicate message IDs
    let mut seen_ids = HashSet::new();
    for msg in &mut session.messages {
        if !seen_ids.insert(msg.id.clone()) {
            let old_id = msg.id.clone();
            msg.id = uuid::Uuid::new_v4().to_string();
            repairs.push(RepairAction {
                repair_type: RepairType::DuplicateMessageId,
                description: format!("Regenerated duplicate ID {} -> {}", old_id, msg.id),
            });
        }
    }

    // Repair 4: Fix invalid timestamps
    let epoch = chrono::DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let now = Utc::now();

    for msg in &mut session.messages {
        if msg.timestamp < epoch || msg.timestamp > now + chrono::Duration::hours(24) {
            msg.timestamp = now;
            repairs.push(RepairAction {
                repair_type: RepairType::InvalidTimestamp,
                description: format!("Fixed invalid timestamp for message {}", msg.id),
            });
        }
    }

    // Repair 5: Fix empty content (replace with placeholder)
    for msg in &mut session.messages {
        if msg.content.trim().is_empty() && msg.role != MessageRole::Tool {
            msg.content = "[empty message]".to_string();
            repairs.push(RepairAction {
                repair_type: RepairType::EmptyContent,
                description: format!("Fixed empty content for message {}", msg.id),
            });
        }
    }

    // Repair 6: Fix orphaned tool responses
    let tool_call_ids: HashSet<String> = session
        .messages
        .iter()
        .filter(|m| m.role == MessageRole::Tool && m.tool_call_id.is_some())
        .filter_map(|m| m.tool_call_id.clone())
        .collect();

    // Check if tool calls have corresponding assistant messages with tool_call_id
    // For now, just ensure tool messages have valid structure
    let mut orphaned_indices = Vec::new();
    for (i, msg) in session.messages.iter().enumerate() {
        if msg.role == MessageRole::Tool && msg.content.trim().is_empty() && msg.tool_name.is_none()
        {
            orphaned_indices.push(i);
        }
    }

    // Remove orphaned tool messages (iterate in reverse to preserve indices)
    for &idx in orphaned_indices.iter().rev() {
        let removed = session.messages.remove(idx);
        repairs.push(RepairAction {
            repair_type: RepairType::OrphanedToolResponse,
            description: format!("Removed orphaned tool message {}", removed.id),
        });
    }

    // Repair 7: Fix session timestamp consistency
    if let Some(first_msg) = session.messages.first() {
        if session.created_at > first_msg.timestamp {
            session.created_at = first_msg.timestamp;
            repairs.push(RepairAction {
                repair_type: RepairType::TimestampConsistency,
                description: "Fixed created_at to match earliest message".to_string(),
            });
        }
    }
    if let Some(last_msg) = session.messages.last() {
        if session.updated_at < last_msg.timestamp {
            session.updated_at = last_msg.timestamp;
            repairs.push(RepairAction {
                repair_type: RepairType::TimestampConsistency,
                description: "Fixed updated_at to match latest message".to_string(),
            });
        }
    }

    // Repair 8: Fix session status if inconsistent
    if session.messages.is_empty() && session.status == SessionStatus::Processing {
        session.status = SessionStatus::Idle;
        repairs.push(RepairAction {
            repair_type: RepairType::StatusRepair,
            description: "Fixed status from Processing to Idle (no messages)".to_string(),
        });
    }

    // Update session timestamp
    if !repairs.is_empty() {
        session.updated_at = Utc::now();
    }

    let issues_found = repairs.len();
    Ok(RepairResult {
        repairs_needed: !repairs.is_empty(),
        issues_found,
        issues_fixed: issues_found,
        repairs,
    })
}

/// Validate a session and return list of issues without fixing them
pub fn validate_session(session: &Session) -> Vec<String> {
    let mut issues = Vec::new();

    if session.key.is_empty() {
        issues.push("Session key is empty".to_string());
    }

    let mut seen_ids = HashSet::new();
    for msg in &session.messages {
        if msg.id.is_empty() {
            issues.push(format!("Message at index has empty ID (role: {:?})", msg.role));
        }
        if !seen_ids.insert(&msg.id) {
            issues.push(format!("Duplicate message ID: {}", msg.id));
        }
        if msg.content.trim().is_empty() && msg.role != MessageRole::Tool {
            issues.push(format!("Empty content in message {}", msg.id));
        }
    }

    if session.agent_id.is_empty() {
        issues.push("Agent ID is empty".to_string());
    }

    if session.created_at > session.updated_at {
        issues.push("created_at is after updated_at".to_string());
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Session;

    #[test]
    fn test_repair_healthy_session() {
        let mut session = Session::new("agent-1");
        session.add_message(MessageRole::User, "Hello");
        session.add_message(MessageRole::Assistant, "Hi");

        let result = repair_session(&mut session).unwrap();
        assert!(!result.repairs_needed);
        assert_eq!(result.issues_found, 0);
    }

    #[test]
    fn test_repair_missing_message_ids() {
        let mut session = Session::new("agent-1");
        session.add_message(MessageRole::User, "Hello");
        session.messages[0].id = String::new(); // Clear the ID

        let result = repair_session(&mut session).unwrap();
        assert!(result.repairs_needed);
        assert!(!session.messages[0].id.is_empty());
    }

    #[test]
    fn test_repair_duplicate_ids() {
        let mut session = Session::new("agent-1");
        session.add_message(MessageRole::User, "Hello");
        session.add_message(MessageRole::Assistant, "Hi");
        session.messages[1].id = session.messages[0].id.clone(); // Duplicate

        let result = repair_session(&mut session).unwrap();
        assert!(result.repairs_needed);
        assert_ne!(session.messages[0].id, session.messages[1].id);
    }

    #[test]
    fn test_repair_empty_content() {
        let mut session = Session::new("agent-1");
        session.add_message(MessageRole::User, "   ");

        let result = repair_session(&mut session).unwrap();
        assert!(result.repairs_needed);
        assert_eq!(session.messages[0].content, "[empty message]");
    }

    #[test]
    fn test_repair_missing_session_key() {
        let mut session = Session::new("agent-1");
        session.key = String::new();

        let result = repair_session(&mut session).unwrap();
        assert!(result.repairs_needed);
        assert!(!session.key.is_empty());
    }

    #[test]
    fn test_repair_processing_status_no_messages() {
        let mut session = Session::new("agent-1");
        session.status = SessionStatus::Processing;

        let result = repair_session(&mut session).unwrap();
        assert!(result.repairs_needed);
        assert_eq!(session.status, SessionStatus::Idle);
    }

    #[test]
    fn test_validate_session() {
        let mut session = Session::new("agent-1");
        session.add_message(MessageRole::User, "Hello");

        let issues = validate_session(&session);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_validate_session_with_issues() {
        let mut session = Session::new("");
        session.key = String::new();

        let issues = validate_session(&session);
        assert!(!issues.is_empty());
    }
}
