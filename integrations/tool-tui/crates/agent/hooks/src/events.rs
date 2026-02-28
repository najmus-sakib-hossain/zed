//! Hook event types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Event types that can trigger hooks
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HookEventType {
    /// Message received on any channel
    MessageReceived,
    /// Message sent
    MessageSent,
    /// Session started
    SessionStart,
    /// Session ended
    SessionEnd,
    /// File changed
    FileChanged,
    /// Command executed
    CommandExecuted,
    /// Tool invoked
    ToolInvoked,
    /// Error occurred
    Error,
    /// Scheduled trigger
    Scheduled,
    /// Custom event
    Custom(String),
}

/// Hook event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEvent {
    pub event_type: HookEventType,
    pub source: String,
    pub data: HashMap<String, serde_json::Value>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl HookEvent {
    pub fn new(event_type: HookEventType, source: impl Into<String>) -> Self {
        Self {
            event_type,
            source: source.into(),
            data: HashMap::new(),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn with_data(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.data.insert(key.into(), value);
        self
    }
}

/// Hook result from Lua execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookResult {
    /// Whether the event should continue propagating
    pub propagate: bool,
    /// Modified data (if hook modified the event)
    pub modified_data: Option<HashMap<String, serde_json::Value>>,
    /// Log messages from the hook
    pub logs: Vec<String>,
}

impl Default for HookResult {
    fn default() -> Self {
        Self {
            propagate: true,
            modified_data: None,
            logs: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_event_creation() {
        let event = HookEvent::new(HookEventType::MessageReceived, "telegram")
            .with_data("content", serde_json::json!("Hello!"))
            .with_data("user_id", serde_json::json!("12345"));

        assert_eq!(event.event_type, HookEventType::MessageReceived);
        assert_eq!(event.source, "telegram");
        assert_eq!(event.data.len(), 2);
    }
}
