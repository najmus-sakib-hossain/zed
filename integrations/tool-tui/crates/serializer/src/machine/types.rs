//! Machine format error types

use thiserror::Error;

/// Result type for machine operations
pub type Result<T> = std::result::Result<T, DxMachineError>;

/// Machine format errors
#[derive(Debug, Error)]
pub enum DxMachineError {
    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Decompression error: {0}")]
    Decompression(String),

    #[error("Decompression failed: {0}")]
    DecompressionFailed(String),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Invalid magic bytes")]
    InvalidMagic,

    #[error("Buffer too small: required {required}, got {actual}")]
    BufferTooSmall { required: usize, actual: usize },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
