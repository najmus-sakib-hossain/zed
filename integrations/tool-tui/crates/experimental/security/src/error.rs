//! Error types for dx-security

use std::path::PathBuf;

/// Result type alias for dx-security operations
pub type Result<T> = std::result::Result<T, SecurityError>;

/// Security scanner error types
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    /// Failed to load vulnerability index
    #[error("Failed to load vulnerability index: {0}")]
    IndexLoadError(String),

    /// Failed to map file into memory
    #[error("Failed to map file: {path}")]
    MapError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Invalid cryptographic signature
    #[error("Invalid signature")]
    SignatureError,

    /// Rule compilation failed
    #[error("Rule compilation failed: {0}")]
    RuleCompileError(String),

    /// Lockfile parsing error
    #[error("Lockfile parse error: {0}")]
    LockfileParseError(String),

    /// Score below threshold
    #[error("Score below threshold: {score} < {threshold}")]
    ThresholdError { score: u8, threshold: u8 },

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Invalid database format
    #[error("Invalid database format: {0}")]
    InvalidFormat(String),
}
