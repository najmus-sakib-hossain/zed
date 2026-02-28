//! Error context and enhanced error handling

use super::types::DxError;
use std::path::PathBuf;
use std::time::Duration;

/// Context information for enhanced error reporting
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Name of the operation that failed
    pub operation: String,
    /// Additional context details
    pub details: Option<String>,
    /// File path if relevant
    pub path: Option<PathBuf>,
}

impl ErrorContext {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            details: None,
            path: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }
}

/// Enhanced error with retry information and context
#[derive(Debug)]
pub struct EnhancedError {
    /// The underlying error
    pub error: DxError,
    /// Current retry attempt count
    pub retry_count: u32,
    /// Maximum number of retries allowed
    pub max_retries: u32,
    /// Error context information
    pub context: ErrorContext,
}

impl EnhancedError {
    /// Create a new enhanced error with context
    pub fn new(error: DxError, context: ErrorContext) -> Self {
        Self {
            error,
            retry_count: 0,
            max_retries: 0,
            context,
        }
    }

    /// Create an enhanced error with retry information
    pub fn with_retries(
        error: DxError,
        context: ErrorContext,
        retry_count: u32,
        max_retries: u32,
    ) -> Self {
        Self {
            error,
            retry_count,
            max_retries,
            context,
        }
    }

    /// Format the error for display with full context
    pub fn display_message(&self) -> String {
        let mut msg = format!("Error during {}: {}", self.context.operation, self.error);

        if let Some(ref details) = self.context.details {
            msg.push_str(&format!("\n  Details: {}", details));
        }

        if let Some(ref path) = self.context.path {
            msg.push_str(&format!("\n  Path: {}", path.display()));
        }

        if self.retry_count > 0 {
            msg.push_str(&format!(
                "\n  Retried {} of {} times",
                self.retry_count, self.max_retries
            ));
        }

        if let Some(hint) = self.error.hint() {
            msg.push_str(&format!("\n  Hint: {}", hint));
        }

        msg
    }

    /// Check if the error should be retried
    pub fn should_retry(&self) -> bool {
        self.retry_count < self.max_retries && self.error.is_retryable()
    }

    /// Calculate the next retry delay using exponential backoff
    pub fn next_retry_delay(&self) -> Duration {
        let base_delay_ms = 1000u64;
        let multiplier = 1u64 << self.retry_count.min(10);
        Duration::from_millis(base_delay_ms * multiplier)
    }
}

impl std::fmt::Display for EnhancedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_message())
    }
}

impl std::error::Error for EnhancedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}
