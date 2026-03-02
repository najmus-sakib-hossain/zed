//! Unified error types for the metasearch engine.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MetasearchError {
    #[error("Engine '{engine}' failed: {message}")]
    EngineError { engine: String, message: String },

    /// General engine error message (used by most engine implementations).
    #[error("Engine error: {0}")]
    Engine(String),

    #[error("HTTP request failed: {0}")]
    HttpError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Rate limited: try again in {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("Timeout after {0}ms")]
    Timeout(u64),

    #[error("No engines available for category '{0}'")]
    NoEnginesAvailable(String),

    #[error("{0}")]
    Other(String),
}

impl From<serde_json::Error> for MetasearchError {
    fn from(e: serde_json::Error) -> Self {
        MetasearchError::ParseError(e.to_string())
    }
}

impl From<url::ParseError> for MetasearchError {
    fn from(e: url::ParseError) -> Self {
        MetasearchError::ParseError(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, MetasearchError>;
