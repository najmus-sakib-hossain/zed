//! Error types for macro compatibility.

use thiserror::Error;

/// Macro error type.
#[derive(Debug, Error)]
pub enum MacroError {
    /// Execution failed
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    /// Serialization failed
    #[error("Serialization failed: {0}")]
    Serialization(String),

    /// File access error
    #[error("File access error: {0}")]
    FileAccess(String),

    /// Environment variable error
    #[error("Environment variable error: {0}")]
    EnvVar(String),

    /// Timeout error
    #[error("Macro execution timed out after {0}ms")]
    Timeout(u64),

    /// Invalid macro definition
    #[error("Invalid macro definition: {0}")]
    InvalidDefinition(String),

    /// Macro not found
    #[error("Macro not found: {0}")]
    NotFound(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for macro operations.
pub type MacroResult<T> = Result<T, MacroError>;
