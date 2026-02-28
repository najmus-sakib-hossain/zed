//! Streaming response types for LLM providers

use serde::{Deserialize, Serialize};

use crate::models::{ToolCall, Usage};

/// Stream events emitted during streaming completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamEvent {
    /// Start of stream
    Start { id: String, model: String },

    /// Text content delta
    Delta { content: String },

    /// Tool call delta
    ToolCallDelta { tool_call: ToolCall },

    /// Usage statistics (sent at end)
    Usage(Usage),

    /// Stream complete
    Done { finish_reason: String },

    /// Error during streaming
    Error { message: String },
}

impl StreamEvent {
    /// Get text content if this is a Delta event
    pub fn as_text(&self) -> Option<&str> {
        match self {
            StreamEvent::Delta { content } => Some(content),
            _ => None,
        }
    }

    /// Check if this is the final event
    pub fn is_done(&self) -> bool {
        matches!(self, StreamEvent::Done { .. } | StreamEvent::Error { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_event_as_text() {
        let evt = StreamEvent::Delta {
            content: "Hello".into(),
        };
        assert_eq!(evt.as_text(), Some("Hello"));

        let evt = StreamEvent::Start {
            id: "1".into(),
            model: "gpt-4".into(),
        };
        assert_eq!(evt.as_text(), None);
    }

    #[test]
    fn test_is_done() {
        let evt = StreamEvent::Done {
            finish_reason: "stop".into(),
        };
        assert!(evt.is_done());

        let evt = StreamEvent::Delta {
            content: "text".into(),
        };
        assert!(!evt.is_done());
    }
}
