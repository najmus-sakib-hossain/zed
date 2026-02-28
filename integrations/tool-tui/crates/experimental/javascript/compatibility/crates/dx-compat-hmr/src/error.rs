//! Error types for HMR compatibility.

use thiserror::Error;

/// HMR error type.
#[derive(Debug, Error)]
pub enum HmrError {
    /// Watch error
    #[error("Watch error: {0}")]
    Watch(String),

    /// Update failed
    #[error("Update failed: {0}")]
    UpdateFailed(String),

    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Notify error
    #[error("Notify error: {0}")]
    Notify(#[from] notify::Error),
}

/// Result type for HMR operations.
pub type HmrResult<T> = Result<T, HmrError>;
