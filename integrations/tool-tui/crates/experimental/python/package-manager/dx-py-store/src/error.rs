//! Error types for the package store

use thiserror::Error;

/// Result type for store operations
pub type StoreResult<T> = Result<T, StoreError>;

/// Errors that can occur during store operations
#[derive(Error, Debug)]
pub enum StoreError {
    /// Package not found in store
    #[error("package not found: {0}")]
    PackageNotFound(String),

    /// File not found within package
    #[error("file not found in package: {0}")]
    FileNotFound(String),

    /// Hash verification failed
    #[error("integrity error: expected hash {expected}, got {actual}")]
    IntegrityError { expected: String, actual: String },

    /// Invalid magic bytes
    #[error("invalid magic bytes: expected {expected:?}, found {found:?}")]
    InvalidMagic { expected: [u8; 4], found: [u8; 4] },

    /// Unsupported version
    #[error("unsupported store version: {0}")]
    UnsupportedVersion(u16),

    /// Package too large
    #[error("package too large: {size} bytes exceeds limit of {limit} bytes")]
    PackageTooLarge { size: u64, limit: u64 },

    /// Memory mapping failed
    #[error("memory mapping failed: {0}")]
    MmapFailed(String),

    /// Symlink creation failed
    #[error("failed to create symlink: {0}")]
    SymlinkFailed(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Index corrupted
    #[error("package index corrupted: {0}")]
    IndexCorrupted(String),
}
