//! Session context window management.
//!
//! Manages the sliding window of conversation context sent to the LLM,
//! with token counting, compaction, and summarization support.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single message in the context window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMessage {
    /// Unique message ID
    pub id: String,
    /// Role: "user", "assistant", "system", "tool"
    pub role: String,
    /// Message content
    pub content: String,
    /// Approximate token count
    pub token_count: u32,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Tool call ID (for tool messages)
    pub tool_call_id: Option<String>,
    /// Whether this message is pinned (won't be evicted)
    pub pinned: bool,
}

/// Context window with sliding window management
#[derive(Debug, Clone)]
pub struct ContextWindow {
    /// All messages in the window
    messages: Vec<ContextMessage>,
    /// System prompt (always first)
    system_prompt: Option<ContextMessage>,
    /// Maximum token budget
    max_tokens: u32,
    /// Current token usage
    current_tokens: u32,
    /// Reserve tokens for response
    reserve_tokens: u32,
}

impl ContextWindow {
    /// Create a new context window with the given token limits
    pub fn new(max_tokens: u32, reserve_tokens: u32) -> Self {
        Self {
            messages: Vec::new(),
            system_prompt: None,
            max_tokens,
            current_tokens: 0,
            reserve_tokens,
        }
    }

    /// Set the system prompt
    pub fn set_system_prompt(&mut self, content: String, token_count: u32) {
        let msg = ContextMessage {
            id: "system".into(),
            role: "system".into(),
            content,
            token_count,
            timestamp: Utc::now(),
            tool_call_id: None,
            pinned: true,
        };
        if let Some(ref old) = self.system_prompt {
            self.current_tokens -= old.token_count;
        }
        self.current_tokens += token_count;
        self.system_prompt = Some(msg);
    }

    /// Add a message to the context window
    pub fn push(&mut self, message: ContextMessage) {
        self.current_tokens += message.token_count;
        self.messages.push(message);
        self.evict_if_needed();
    }

    /// Get available tokens for the response
    pub fn available_tokens(&self) -> u32 {
        self.max_tokens
            .saturating_sub(self.current_tokens)
            .saturating_sub(self.reserve_tokens)
    }

    /// Get all messages in order (system prompt first)
    pub fn messages(&self) -> Vec<&ContextMessage> {
        let mut result = Vec::new();
        if let Some(ref sys) = self.system_prompt {
            result.push(sys);
        }
        result.extend(self.messages.iter());
        result
    }

    /// Get the total number of messages
    pub fn len(&self) -> usize {
        self.messages.len() + if self.system_prompt.is_some() { 1 } else { 0 }
    }

    /// Check if the window is empty
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty() && self.system_prompt.is_none()
    }

    /// Get current token usage
    pub fn current_tokens(&self) -> u32 {
        self.current_tokens
    }

    /// Clear all non-pinned messages
    pub fn clear(&mut self) {
        let pinned: Vec<ContextMessage> = self.messages.drain(..).filter(|m| m.pinned).collect();
        self.current_tokens = self.system_prompt.as_ref().map(|s| s.token_count).unwrap_or(0);
        for msg in &pinned {
            self.current_tokens += msg.token_count;
        }
        self.messages = pinned;
    }

    /// Compact the window by summarizing old messages
    pub fn compact(&mut self, summary: String, summary_tokens: u32) {
        if self.messages.len() <= 2 {
            return;
        }

        // Keep the last 2 messages, replace older with summary
        let keep_count = 2;
        let remove_count = self.messages.len() - keep_count;
        let removed: Vec<ContextMessage> = self.messages.drain(..remove_count).collect();

        let removed_tokens: u32 = removed.iter().map(|m| m.token_count).sum();
        self.current_tokens -= removed_tokens;

        // Insert summary at the beginning
        let summary_msg = ContextMessage {
            id: format!("summary-{}", Utc::now().timestamp()),
            role: "system".into(),
            content: format!("[Conversation summary]: {}", summary),
            token_count: summary_tokens,
            timestamp: Utc::now(),
            tool_call_id: None,
            pinned: false,
        };
        self.current_tokens += summary_tokens;
        self.messages.insert(0, summary_msg);
    }

    /// Evict oldest non-pinned messages to stay within budget
    fn evict_if_needed(&mut self) {
        let budget = self.max_tokens.saturating_sub(self.reserve_tokens);
        while self.current_tokens > budget && !self.messages.is_empty() {
            // Find the first non-pinned message
            if let Some(idx) = self.messages.iter().position(|m| !m.pinned) {
                let removed = self.messages.remove(idx);
                self.current_tokens -= removed.token_count;
            } else {
                break; // All messages are pinned
            }
        }
    }
}

/// Approximate token count for a string (rough: ~4 chars per token)
pub fn estimate_tokens(text: &str) -> u32 {
    (text.len() as u32 / 4).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_window_basic() {
        let mut window = ContextWindow::new(1000, 200);
        assert!(window.is_empty());

        window.push(ContextMessage {
            id: "1".into(),
            role: "user".into(),
            content: "Hello".into(),
            token_count: 5,
            timestamp: Utc::now(),
            tool_call_id: None,
            pinned: false,
        });

        assert_eq!(window.len(), 1);
        assert_eq!(window.current_tokens(), 5);
        assert_eq!(window.available_tokens(), 795);
    }

    #[test]
    fn test_context_window_eviction() {
        let mut window = ContextWindow::new(100, 20);

        for i in 0..20 {
            window.push(ContextMessage {
                id: i.to_string(),
                role: "user".into(),
                content: format!("Message {}", i),
                token_count: 10,
                timestamp: Utc::now(),
                tool_call_id: None,
                pinned: false,
            });
        }

        // Should have evicted older messages to stay within budget
        assert!(window.current_tokens() <= 80);
    }

    #[test]
    fn test_context_window_pinned() {
        let mut window = ContextWindow::new(50, 10);

        window.push(ContextMessage {
            id: "pinned".into(),
            role: "user".into(),
            content: "Important".into(),
            token_count: 10,
            pinned: true,
            timestamp: Utc::now(),
            tool_call_id: None,
        });

        // Fill with non-pinned
        for i in 0..10 {
            window.push(ContextMessage {
                id: i.to_string(),
                role: "user".into(),
                content: format!("Msg {}", i),
                token_count: 10,
                pinned: false,
                timestamp: Utc::now(),
                tool_call_id: None,
            });
        }

        // Pinned message should still be there
        assert!(window.messages().iter().any(|m| m.id == "pinned"));
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens("hello world"), 2);
        assert_eq!(estimate_tokens(""), 1);
    }
}
