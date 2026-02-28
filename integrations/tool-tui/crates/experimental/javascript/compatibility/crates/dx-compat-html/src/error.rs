//! Error types for HTML compatibility.

use thiserror::Error;

/// HTML error type.
#[derive(Debug, Error)]
pub enum HtmlError {
    /// Parse error
    #[error("Parse error: {0}")]
    Parse(String),

    /// Selector error
    #[error("Invalid selector: {0}")]
    InvalidSelector(String),

    /// Transform error
    #[error("Transform error: {0}")]
    Transform(String),

    /// Rewriting error
    #[error("Rewriting error: {0}")]
    Rewriting(#[from] lol_html::errors::RewritingError),
}

/// Result type for HTML operations.
pub type HtmlResult<T> = Result<T, HtmlError>;
