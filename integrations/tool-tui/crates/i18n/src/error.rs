//! Error types for the i18n library

use thiserror::Error;

/// Main error type for the i18n library
#[derive(Error, Debug)]
pub enum I18nError {
    #[error("Language not supported: {0}")]
    LanguageNotSupported(String),

    #[error("Invalid source or target language: {0}")]
    InvalidLanguage(String),

    #[error("Translation not found for text: {0}")]
    TranslationNotFound(String),

    #[error("Text length must be between {min} and {max} characters")]
    InvalidLength { min: usize, max: usize },

    #[error("API key required for {0}. Set environment variable: {1}")]
    ApiKeyRequired(String, String),

    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("Too many requests: {0}")]
    TooManyRequests(String),

    #[error("Server error ({code}): {message}")]
    ServerError { code: u16, message: String },

    #[error("No audio received from TTS service")]
    NoAudioReceived,

    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    #[error("Unexpected response: {0}")]
    UnexpectedResponse(String),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Invalid voice: {0}")]
    InvalidVoice(String),

    #[error("Processing error: {0}")]
    ProcessingError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("{0}")]
    Other(String),
}

/// Result type alias for i18n operations
pub type Result<T> = std::result::Result<T, I18nError>;
