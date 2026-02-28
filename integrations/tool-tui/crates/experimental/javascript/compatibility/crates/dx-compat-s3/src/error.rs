//! Error types for S3 compatibility.

use thiserror::Error;

/// S3 error type.
#[derive(Debug, Error)]
pub enum S3Error {
    /// No such key
    #[error("No such key: {0}")]
    NoSuchKey(String),

    /// Access denied
    #[error("Access denied: {0}")]
    AccessDenied(String),

    /// Bucket not found
    #[error("Bucket not found: {0}")]
    BucketNotFound(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Invalid data
    #[error("Invalid data: {0}")]
    InvalidData(String),

    /// Upload error
    #[error("Upload error: {0}")]
    Upload(String),
}

/// Result type for S3 operations.
pub type S3Result<T> = Result<T, S3Error>;
