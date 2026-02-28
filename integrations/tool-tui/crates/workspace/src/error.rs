//! Error types for dx-workspace.

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias using dx-workspace's Error type.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during workspace configuration operations.
#[derive(Debug, Error)]
pub enum Error {
    /// Failed to read or write configuration file.
    #[error("IO error at '{path}': {source}")]
    Io {
        /// Path where the error occurred.
        path: PathBuf,
        /// Underlying IO error.
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse JSON configuration.
    #[error("JSON parse error in '{path}': {source}")]
    JsonParse {
        /// Path to the JSON file.
        path: PathBuf,
        /// Underlying parse error.
        #[source]
        source: serde_json::Error,
    },

    /// Failed to parse YAML configuration.
    #[error("YAML parse error in '{path}': {source}")]
    YamlParse {
        /// Path to the YAML file.
        path: PathBuf,
        /// Underlying parse error.
        #[source]
        source: serde_yaml::Error,
    },

    /// Failed to parse TOML configuration.
    #[error("TOML parse error in '{path}': {source}")]
    TomlParse {
        /// Path to the TOML file.
        path: PathBuf,
        /// Underlying parse error.
        #[source]
        source: toml::de::Error,
    },

    /// Invalid configuration value.
    #[error("Invalid configuration: {message}")]
    InvalidConfig {
        /// Description of what's invalid.
        message: String,
    },

    /// Platform not supported for the requested operation.
    #[error("Platform '{platform}' is not supported for this operation")]
    UnsupportedPlatform {
        /// The unsupported platform name.
        platform: String,
    },

    /// Project detection failed.
    #[error("Failed to detect project type in '{path}'")]
    ProjectDetectionFailed {
        /// Path where detection was attempted.
        path: PathBuf,
    },

    /// Workspace configuration not found.
    #[error("Workspace configuration not found in '{path}'")]
    ConfigNotFound {
        /// Path where config was expected.
        path: PathBuf,
    },

    /// Template rendering error.
    #[error("Template error: {message}")]
    Template {
        /// Description of the template error.
        message: String,
    },

    /// Cache operation failed.
    #[error("Cache error: {message}")]
    Cache {
        /// Description of the cache error.
        message: String,
    },

    /// Validation error with details.
    #[error("Validation failed: {message}")]
    Validation {
        /// Description of validation failure.
        message: String,
    },

    /// Sync conflict detected.
    #[error("Sync conflict detected in '{path}': {message}")]
    SyncConflict {
        /// Path with conflict.
        path: PathBuf,
        /// Description of the conflict.
        message: String,
    },

    /// Invalid binary format.
    #[error("Invalid binary format: {reason}")]
    InvalidBinaryFormat {
        /// Description of what's invalid.
        reason: String,
    },

    /// Serialization error.
    #[error("Serialization error ({format}): {details}")]
    Serialization {
        /// Format being serialized (json, yaml, binary, etc.).
        format: String,
        /// Error details.
        details: String,
    },
}

impl Error {
    /// Create an IO error with path context.
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// Create a JSON parse error with path context.
    pub fn json_parse(path: impl Into<PathBuf>, source: serde_json::Error) -> Self {
        Self::JsonParse {
            path: path.into(),
            source,
        }
    }

    /// Create a YAML parse error with path context.
    pub fn yaml_parse(path: impl Into<PathBuf>, source: serde_yaml::Error) -> Self {
        Self::YamlParse {
            path: path.into(),
            source,
        }
    }

    /// Create a TOML parse error with path context.
    pub fn toml_parse(path: impl Into<PathBuf>, source: toml::de::Error) -> Self {
        Self::TomlParse {
            path: path.into(),
            source,
        }
    }

    /// Create an invalid configuration error.
    pub fn invalid_config(message: impl Into<String>) -> Self {
        Self::InvalidConfig {
            message: message.into(),
        }
    }

    /// Create an unsupported platform error.
    pub fn unsupported_platform(platform: impl Into<String>) -> Self {
        Self::UnsupportedPlatform {
            platform: platform.into(),
        }
    }

    /// Create a validation error.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }
}
