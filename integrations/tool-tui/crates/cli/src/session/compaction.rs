//! # Session Compaction
//!
//! Reduces session size by summarizing old messages, removing redundant
//! tool interactions, and deduplicating content while preserving
//! conversation context and important decisions.

use super::{Message, MessageRole, Session, SessionError, SessionStatus};
use chrono::Utc;

/// Result of a compaction operation
#[derive(Debug, Clone)]
pub struct CompactionResult {
    /// Number of messages before compaction
    pub original_message_count: usize,
    /// Number of messages after compaction
    pub compacted_message_count: usize,
    /// Bytes before compaction
    pub original_size: usize,
    /// Bytes after compaction
    pub compacted_size: usize,
    /// Number of messages removed
    pub messages_removed: usize,
    /// Number of tool call chains collapsed
    pub tool_chains_collapsed: usize,
}

/// Compaction configuration
#[derive(Debug, Clone)]
pub struct CompactionConfig {
    /// Keep the last N messages unmodified
    pub keep_recent: usize,
    /// Maximum messages after compaction
    pub target_max_messages: usize,
    /// Collapse consecutive tool calls into summaries
    pub collapse_tool_calls: bool,
    /// Remove empty or whitespace-only messages
    pub remove_empty: bool,
    /// Deduplicate identical consecutive messages
    pub deduplicate: bool,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            keep_recent: 10,
            target_max_messages: 50,
            collapse_tool_calls: true,
            remove_empty: true,
            deduplicate: true,
        }
    }
}

/// Compact a session to reduce its size
pub fn compact_session(session: &mut Session) -> Result<CompactionResult, SessionError> {
    compact_session_with_config(session, &CompactionConfig::default())
}

/// Compact a session with custom configuration
pub fn compact_session_with_config(
    session: &mut Session,
    config: &CompactionConfig,
) -> Result<CompactionResult, SessionError> {
    let original_count = session.messages.len();
    let original_size = session.byte_size();

    if original_count <= config.keep_recent {
        return Ok(CompactionResult {
            original_message_count: original_count,
            compacted_message_count: original_count,
            original_size,
            compacted_size: original_size,
            messages_removed: 0,
            tool_chains_collapsed: 0,
        });
    }

    let mut tool_chains_collapsed = 0u32;

    // Phase 1: Remove empty messages
    if config.remove_empty {
        session.messages.retain(|m| !m.content.trim().is_empty());
    }

    // Phase 2: Deduplicate consecutive identical messages
    if config.deduplicate {
        session.messages.dedup_by(|a, b| a.role == b.role && a.content == b.content);
    }

    // Phase 3: Collapse tool call chains
    if config.collapse_tool_calls {
        let split_point = if session.messages.len() > config.keep_recent {
            session.messages.len() - config.keep_recent
        } else {
            0
        };

        if split_point > 0 {
            let (older, recent) = session.messages.split_at(split_point);
            let mut compacted_older = Vec::new();
            let mut i = 0;
            let older_vec: Vec<Message> = older.to_vec();

            while i < older_vec.len() {
                if older_vec[i].role == MessageRole::Tool {
                    // Collect consecutive tool messages
                    let chain_start = i;
                    let mut tool_names: Vec<String> = Vec::new();

                    while i < older_vec.len() && older_vec[i].role == MessageRole::Tool {
                        if let Some(ref name) = older_vec[i].tool_name {
                            if !tool_names.contains(name) {
                                tool_names.push(name.clone());
                            }
                        }
                        i += 1;
                    }

                    let chain_len = i - chain_start;
                    if chain_len > 1 {
                        // Collapse chain into summary
                        let summary = Message {
                            id: uuid::Uuid::new_v4().to_string(),
                            role: MessageRole::System,
                            content: format!(
                                "[Compacted: {} tool calls ({})]",
                                chain_len,
                                if tool_names.is_empty() {
                                    "various".to_string()
                                } else {
                                    tool_names.join(", ")
                                }
                            ),
                            timestamp: older_vec[chain_start].timestamp,
                            token_count: None,
                            tool_call_id: None,
                            tool_name: None,
                            metadata: std::collections::HashMap::new(),
                        };
                        compacted_older.push(summary);
                        tool_chains_collapsed += 1;
                    } else {
                        compacted_older.push(older_vec[chain_start].clone());
                    }
                } else {
                    compacted_older.push(older_vec[i].clone());
                    i += 1;
                }
            }

            // Phase 4: If still too many messages, keep system + first exchange + summary + recent
            if compacted_older.len() + recent.len() > config.target_max_messages {
                let mut final_older = Vec::new();

                // Keep system messages
                for msg in &compacted_older {
                    if msg.role == MessageRole::System {
                        final_older.push(msg.clone());
                    }
                }

                // Keep first user-assistant exchange for context
                let mut found_user = false;
                for msg in &compacted_older {
                    if msg.role == MessageRole::User && !found_user {
                        final_older.push(msg.clone());
                        found_user = true;
                    } else if msg.role == MessageRole::Assistant && found_user {
                        final_older.push(msg.clone());
                        break;
                    }
                }

                // Add compaction summary
                let non_system_count =
                    compacted_older.iter().filter(|m| m.role != MessageRole::System).count();
                if non_system_count > 2 {
                    final_older.push(Message {
                        id: uuid::Uuid::new_v4().to_string(),
                        role: MessageRole::System,
                        content: format!(
                            "[Session compacted: {} earlier messages summarized]",
                            non_system_count - 2
                        ),
                        timestamp: Utc::now(),
                        token_count: None,
                        tool_call_id: None,
                        tool_name: None,
                        metadata: std::collections::HashMap::new(),
                    });
                }

                compacted_older = final_older;
            }

            // Reassemble messages
            let mut new_messages = compacted_older;
            new_messages.extend(recent.to_vec());
            session.messages = new_messages;
        }
    }

    // Update session metadata
    session.compaction_count += 1;
    session.status = SessionStatus::Compacted;
    session.updated_at = Utc::now();

    let compacted_count = session.messages.len();
    let compacted_size = session.byte_size();

    Ok(CompactionResult {
        original_message_count: original_count,
        compacted_message_count: compacted_count,
        original_size,
        compacted_size,
        messages_removed: original_count.saturating_sub(compacted_count),
        tool_chains_collapsed: tool_chains_collapsed as usize,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Session;

    #[test]
    fn test_compact_empty_session() {
        let mut session = Session::new("agent-1");
        let result = compact_session(&mut session).unwrap();
        assert_eq!(result.messages_removed, 0);
        assert_eq!(result.original_message_count, 0);
    }

    #[test]
    fn test_compact_small_session() {
        let mut session = Session::new("agent-1");
        session.add_message(MessageRole::User, "Hello");
        session.add_message(MessageRole::Assistant, "Hi");

        let result = compact_session(&mut session).unwrap();
        assert_eq!(result.messages_removed, 0);
        assert_eq!(result.compacted_message_count, 2);
    }

    #[test]
    fn test_compact_removes_empty_messages() {
        let mut session = Session::new("agent-1");
        for i in 0..15 {
            if i % 3 == 0 {
                session.add_message(MessageRole::User, "   ");
            } else {
                session.add_message(MessageRole::User, &format!("Message {}", i));
            }
        }

        let original_count = session.messages.len();
        let result = compact_session(&mut session).unwrap();
        assert!(result.compacted_message_count < original_count);
    }

    #[test]
    fn test_compact_deduplicates() {
        let mut session = Session::new("agent-1");
        for _ in 0..20 {
            session.add_message(MessageRole::User, "Same message");
            session.add_message(MessageRole::Assistant, "Same response");
        }

        let result = compact_session(&mut session).unwrap();
        assert!(result.messages_removed > 0);
    }

    #[test]
    fn test_compact_collapses_tool_chains() {
        let mut session = Session::new("agent-1");
        session.add_message(MessageRole::User, "Do something");

        // Add a chain of tool calls
        for i in 0..5 {
            let msg = session.add_message(MessageRole::Tool, &format!("Tool result {}", i));
            // We need mutable access pattern to set tool_name
            let idx = session.messages.len() - 1;
            session.messages[idx].tool_name = Some(format!("tool_{}", i));
        }

        // Add more regular messages to push tool chain into "older" section
        for i in 0..12 {
            session.add_message(MessageRole::User, &format!("Query {}", i));
            session.add_message(MessageRole::Assistant, &format!("Answer {}", i));
        }

        let result = compact_session(&mut session).unwrap();
        assert!(result.tool_chains_collapsed > 0);
    }

    #[test]
    fn test_compaction_increments_count() {
        let mut session = Session::new("agent-1");
        for i in 0..20 {
            session.add_message(MessageRole::User, &format!("Q{}", i));
            session.add_message(MessageRole::Assistant, &format!("A{}", i));
        }

        assert_eq!(session.compaction_count, 0);
        compact_session(&mut session).unwrap();
        assert_eq!(session.compaction_count, 1);
        compact_session(&mut session).unwrap();
        assert_eq!(session.compaction_count, 2);
    }
}
