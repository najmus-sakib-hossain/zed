//! Error types for the reactor

use std::io;
use thiserror::Error;

/// Result type for reactor operations
pub type Result<T> = std::result::Result<T, ReactorError>;

/// Errors that can occur in reactor operations
#[derive(Error, Debug)]
pub enum ReactorError {
    /// I/O error from the underlying system
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Submission queue is full
    #[error("Submission queue is full")]
    SubmissionQueueFull,

    /// Operation not supported on this platform
    #[error("Operation not supported: {0}")]
    Unsupported(String),

    /// Invalid operation parameters
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    /// Reactor has been shut down
    #[error("Reactor has been shut down")]
    Shutdown,

    /// Buffer registration failed
    #[error("Buffer registration failed: {0}")]
    BufferRegistrationFailed(String),

    /// File descriptor registration failed
    #[error("File descriptor registration failed: {0}")]
    FdRegistrationFailed(String),

    /// Operation timed out
    #[error("Operation timed out")]
    Timeout,

    /// Operation was cancelled
    #[error("Operation was cancelled")]
    Cancelled,
}

impl ReactorError {
    /// Create an unsupported operation error
    pub fn unsupported(op: &str) -> Self {
        ReactorError::Unsupported(op.to_string())
    }

    /// Create an invalid parameters error
    pub fn invalid_params(msg: &str) -> Self {
        ReactorError::InvalidParameters(msg.to_string())
    }

    /// Check if this error is recoverable (can retry)
    pub fn is_recoverable(&self) -> bool {
        matches!(self, ReactorError::SubmissionQueueFull | ReactorError::Timeout)
    }

    /// Check if this error indicates the reactor is unusable
    pub fn is_fatal(&self) -> bool {
        matches!(self, ReactorError::Shutdown)
    }
}
