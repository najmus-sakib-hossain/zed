//! Message streaming and coalescing.
//!
//! Buffers partial LLM output and flushes it as edits
//! to a single message, avoiding rapid-fire updates.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Configuration for streaming behaviour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    /// Minimum characters before a flush is allowed.
    pub min_chars: usize,
    /// Idle time (milliseconds) after which the buffer
    /// is flushed regardless of length.
    pub idle_ms: u64,
    /// Maximum characters before a forced split.
    pub max_chars: usize,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            min_chars: 80,
            idle_ms: 500,
            max_chars: 4000,
        }
    }
}

/// Accumulates text chunks and decides when to flush.
#[derive(Debug, Clone)]
pub struct MessageStream {
    buffer: String,
    config: StreamConfig,
    last_append: DateTime<Utc>,
    total_flushed: usize,
    flush_count: usize,
}

impl MessageStream {
    /// Create a new stream with the given config.
    pub fn new(config: StreamConfig) -> Self {
        Self {
            buffer: String::new(),
            config,
            last_append: Utc::now(),
            total_flushed: 0,
            flush_count: 0,
        }
    }

    /// Create a stream with default config.
    pub fn with_defaults() -> Self {
        Self::new(StreamConfig::default())
    }

    /// Append text to the internal buffer.
    pub fn append(&mut self, text: &str) {
        self.buffer.push_str(text);
        self.last_append = Utc::now();
    }

    /// Current buffer length in characters.
    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Check whether a flush should happen now.
    ///
    /// Criteria (any one triggers):
    /// - Buffer length ≥ `min_chars`
    /// - Time since last append ≥ `idle_ms`
    /// - Buffer length ≥ `max_chars`
    pub fn should_flush(&self) -> bool {
        if self.buffer.is_empty() {
            return false;
        }
        if self.buffer.len() >= self.config.max_chars {
            return true;
        }
        if self.buffer.len() >= self.config.min_chars {
            return true;
        }
        let elapsed = Utc::now().signed_duration_since(self.last_append).num_milliseconds();
        elapsed >= self.config.idle_ms as i64
    }

    /// Drain the buffer and return its contents.
    ///
    /// Returns an empty string if the buffer is empty.
    pub fn flush(&mut self) -> String {
        if self.buffer.is_empty() {
            return String::new();
        }
        let content = std::mem::take(&mut self.buffer);
        self.total_flushed += content.len();
        self.flush_count += 1;
        content
    }

    /// Flush if `should_flush()` returns true; otherwise
    /// returns `None`.
    pub fn try_flush(&mut self) -> Option<String> {
        if self.should_flush() {
            Some(self.flush())
        } else {
            None
        }
    }

    /// Total characters flushed over the stream's lifetime.
    pub fn total_flushed(&self) -> usize {
        self.total_flushed
    }

    /// Number of flush operations performed.
    pub fn flush_count(&self) -> usize {
        self.flush_count
    }

    /// Reset the stream to empty state, keeping config.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.total_flushed = 0;
        self.flush_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_stream_is_empty() {
        let s = MessageStream::with_defaults();
        assert!(s.is_empty());
        assert_eq!(s.buffer_len(), 0);
    }

    #[test]
    fn test_append() {
        let mut s = MessageStream::with_defaults();
        s.append("Hello ");
        s.append("World");
        assert_eq!(s.buffer_len(), 11);
        assert!(!s.is_empty());
    }

    #[test]
    fn test_flush() {
        let mut s = MessageStream::with_defaults();
        s.append("data");
        let content = s.flush();
        assert_eq!(content, "data");
        assert!(s.is_empty());
        assert_eq!(s.total_flushed(), 4);
        assert_eq!(s.flush_count(), 1);
    }

    #[test]
    fn test_flush_empty() {
        let mut s = MessageStream::with_defaults();
        let content = s.flush();
        assert_eq!(content, "");
        assert_eq!(s.flush_count(), 0);
    }

    #[test]
    fn test_should_flush_min_chars() {
        let config = StreamConfig {
            min_chars: 5,
            idle_ms: 100_000,
            max_chars: 1000,
        };
        let mut s = MessageStream::new(config);
        s.append("abc");
        assert!(!s.should_flush());
        s.append("de");
        assert!(s.should_flush());
    }

    #[test]
    fn test_should_flush_max_chars() {
        let config = StreamConfig {
            min_chars: 1000,
            idle_ms: 100_000,
            max_chars: 10,
        };
        let mut s = MessageStream::new(config);
        s.append("0123456789");
        assert!(s.should_flush());
    }

    #[test]
    fn test_try_flush() {
        let config = StreamConfig {
            min_chars: 5,
            idle_ms: 100_000,
            max_chars: 1000,
        };
        let mut s = MessageStream::new(config);
        s.append("ab");
        assert!(s.try_flush().is_none());
        s.append("cde");
        let content = s.try_flush().expect("should flush");
        assert_eq!(content, "abcde");
    }

    #[test]
    fn test_reset() {
        let mut s = MessageStream::with_defaults();
        s.append("data");
        s.flush();
        s.reset();
        assert_eq!(s.total_flushed(), 0);
        assert_eq!(s.flush_count(), 0);
        assert!(s.is_empty());
    }

    #[test]
    fn test_should_flush_empty() {
        let s = MessageStream::with_defaults();
        assert!(!s.should_flush());
    }
}
