//! Error types for plugin compatibility.

use thiserror::Error;

/// Plugin error type.
#[derive(Debug, Error)]
pub enum PluginError {
    /// Plugin not found
    #[error("Plugin not found: {0}")]
    NotFound(String),

    /// Setup failed
    #[error("Setup failed: {0}")]
    SetupFailed(String),

    /// Handler error
    #[error("Handler error: {0}")]
    Handler(String),
}

/// Result type for plugin operations.
pub type PluginResult<T> = Result<T, PluginError>;
