//! Error types for the layout cache

use thiserror::Error;

/// Result type for layout operations
pub type LayoutResult<T> = Result<T, LayoutError>;

/// Errors that can occur during layout operations
#[derive(Error, Debug)]
pub enum LayoutError {
    /// Layout not found in cache
    #[error("layout not found for project hash: {0}")]
    LayoutNotFound(String),

    /// Layout corrupted
    #[error("layout corrupted: {0}")]
    LayoutCorrupted(String),

    /// Index corrupted
    #[error("layout index corrupted: {0}")]
    IndexCorrupted(String),

    /// Invalid magic bytes
    #[error("invalid magic bytes: expected {expected:?}, found {found:?}")]
    InvalidMagic { expected: [u8; 4], found: [u8; 4] },

    /// Unsupported version
    #[error("unsupported layout version: {0}")]
    UnsupportedVersion(u16),

    /// Link creation failed
    #[error("failed to create link: {0}")]
    LinkFailed(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Store error
    #[error("store error: {0}")]
    Store(#[from] dx_py_store::StoreError),
}
