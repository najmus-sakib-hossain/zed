//! Error types for shell compatibility.

use thiserror::Error;

/// Shell error type.
#[derive(Debug, Error)]
pub enum ShellError {
    /// Command not found
    #[error("Command not found: {0}")]
    CommandNotFound(String),

    /// Execution failed
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    /// Non-zero exit code
    #[error("Command exited with code {0}")]
    NonZeroExit(i32),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for shell operations.
pub type ShellResult<T> = Result<T, ShellError>;
