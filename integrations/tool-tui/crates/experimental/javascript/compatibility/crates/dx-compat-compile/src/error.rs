//! Error types for compile compatibility.

use thiserror::Error;

/// Compile error type.
#[derive(Debug, Error)]
pub enum CompileError {
    /// Bundling failed
    #[error("Bundling failed: {0}")]
    BundlingFailed(String),

    /// Target not supported
    #[error("Target not supported: {0}")]
    UnsupportedTarget(String),

    /// Asset embedding failed
    #[error("Asset embedding failed: {0}")]
    AssetEmbedding(String),

    /// Compression failed
    #[error("Compression failed: {0}")]
    CompressionFailed(String),

    /// Decompression failed
    #[error("Decompression failed: {0}")]
    DecompressionFailed(String),

    /// Invalid executable format
    #[error("Invalid executable format: {0}")]
    InvalidFormat(String),

    /// Cross-compilation not available
    #[error("Cross-compilation not available for target: {0}")]
    CrossCompilationUnavailable(String),

    /// Asset not found
    #[error("Asset not found: {0}")]
    AssetNotFound(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for compile operations.
pub type CompileResult<T> = Result<T, CompileError>;
