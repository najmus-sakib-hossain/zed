//! Error types for Web compatibility layer.

use thiserror::Error;

/// Unified error type for Web compatibility operations.
#[derive(Debug, Error)]
pub enum WebError {
    /// Fetch error
    #[error("Fetch error: {0}")]
    Fetch(String),

    /// Stream error
    #[error("Stream error: {0}")]
    Stream(String),

    /// WebSocket error
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// URL parsing error
    #[error("URL error: {0}")]
    Url(String),

    /// Abort error
    #[error("Aborted: {0}")]
    Aborted(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Type error
    #[error("Type error: {0}")]
    TypeError(String),
}

/// Result type for Web compatibility operations.
pub type WebResult<T> = Result<T, WebError>;
