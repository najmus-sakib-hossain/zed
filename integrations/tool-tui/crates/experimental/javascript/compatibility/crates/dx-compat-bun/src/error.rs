//! Error types for Bun compatibility layer.

use thiserror::Error;

/// Unified error type for Bun compatibility operations.
#[derive(Debug, Error)]
pub enum BunError {
    /// File error
    #[error("File error: {0}")]
    File(String),

    /// Server error
    #[error("Server error: {0}")]
    Server(String),

    /// Spawn error
    #[error("Spawn error: {0}")]
    Spawn(String),

    /// Hash error
    #[error("Hash error: {0}")]
    Hash(String),

    /// Compression error
    #[error("Compression error: {0}")]
    Compression(String),

    /// Password error
    #[error("Password error: {0}")]
    Password(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for Bun compatibility operations.
pub type BunResult<T> = Result<T, BunError>;
