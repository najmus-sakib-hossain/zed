//! Error types for FFI compatibility.

use thiserror::Error;

/// FFI error type.
#[derive(Debug, Error)]
pub enum FfiError {
    /// Library not found
    #[error("Library not found: {0}")]
    LibraryNotFound(String),

    /// Symbol not found
    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),

    /// Invalid pointer
    #[error("Invalid pointer: {0}")]
    InvalidPointer(String),

    /// Type mismatch
    #[error("Type mismatch: {0}")]
    TypeMismatch(String),

    /// Loading error
    #[error("Loading error: {0}")]
    Loading(#[from] libloading::Error),
}

/// Result type for FFI operations.
pub type FfiResult<T> = Result<T, FfiError>;
