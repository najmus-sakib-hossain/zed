//! Error types for the build pipeline

use std::path::PathBuf;
use thiserror::Error;

/// Result type for build operations
pub type Result<T> = std::result::Result<T, BuildError>;

/// Errors that can occur during the build process
#[derive(Error, Debug)]
pub enum BuildError {
    /// I/O error
    #[error("I/O error at {path:?}: {source}")]
    Io {
        /// Path where the error occurred
        path: PathBuf,
        /// Underlying I/O error
        #[source]
        source: std::io::Error,
    },

    /// Cache error
    #[error("Cache error: {0}")]
    Cache(String),

    /// Media processing error
    #[error("Media processing error: {0}")]
    Media(String),

    /// Style processing error
    #[error("Style processing error: {0}")]
    Style(String),

    /// Icon processing error
    #[error("Icon processing error: {0}")]
    Icon(String),

    /// Font processing error
    #[error("Font processing error: {0}")]
    Font(String),

    /// i18n processing error
    #[error("i18n processing error: {0}")]
    I18n(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// File not found
    #[error("File not found: {0:?}")]
    FileNotFound(PathBuf),

    /// Invalid file format
    #[error("Invalid file format at {path:?}: {message}")]
    InvalidFormat {
        /// Path to the invalid file
        path: PathBuf,
        /// Error message
        message: String,
    },
}
