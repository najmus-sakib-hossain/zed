//! Comprehensive Error Handling Module
//!
//! This module provides enhanced error handling with actionable error messages
//! and proper error propagation across the driven crate.
//!
//! ## Features
//!
//! - Structured error types with context
//! - Actionable error messages with suggestions
//! - Error chaining for debugging
//! - No panics in library code

use std::path::PathBuf;
use thiserror::Error;

/// Enhanced error type with actionable messages
#[derive(Debug, Error)]
pub enum EnhancedError {
    /// Configuration error with suggestion
    #[error("Configuration error: {message}\n  Suggestion: {suggestion}")]
    Config {
        message: String,
        suggestion: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// File not found with path context
    #[error("File not found: {path}\n  Suggestion: {suggestion}")]
    FileNotFound { path: PathBuf, suggestion: String },

    /// Parse error with location
    #[error("Parse error at {location}: {message}\n  Context: {context}")]
    Parse {
        location: String,
        message: String,
        context: String,
    },

    /// Validation error with details
    #[error("Validation error: {message}\n  Field: {field}\n  Suggestion: {suggestion}")]
    Validation {
        field: String,
        message: String,
        suggestion: String,
    },

    /// IO error with path context
    #[error("IO error for {path}: {message}")]
    Io {
        path: PathBuf,
        message: String,
        #[source]
        source: std::io::Error,
    },

    /// Template error with template name
    #[error("Template error in '{template}': {message}\n  Suggestion: {suggestion}")]
    Template {
        template: String,
        message: String,
        suggestion: String,
    },

    /// Sync error with editor context
    #[error("Sync error for {editor}: {message}\n  Suggestion: {suggestion}")]
    Sync {
        editor: String,
        message: String,
        suggestion: String,
    },

    /// Security error
    #[error("Security error: {message}\n  Action required: {action}")]
    Security { message: String, action: String },

    /// Binary format error
    #[error("Binary format error: {message}\n  Expected: {expected}, Got: {actual}")]
    BinaryFormat {
        message: String,
        expected: String,
        actual: String,
    },

    /// Hook error
    #[error("Hook error for '{hook_id}': {message}")]
    Hook { hook_id: String, message: String },

    /// DCP protocol error
    #[error("DCP protocol error: {message}\n  Code: {code}")]
    Protocol { code: i32, message: String },
}

impl EnhancedError {
    /// Create a configuration error
    pub fn config(message: impl Into<String>, suggestion: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
            suggestion: suggestion.into(),
            source: None,
        }
    }

    /// Create a configuration error with source
    pub fn config_with_source(
        message: impl Into<String>,
        suggestion: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Config {
            message: message.into(),
            suggestion: suggestion.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Create a file not found error
    pub fn file_not_found(path: impl Into<PathBuf>, suggestion: impl Into<String>) -> Self {
        Self::FileNotFound {
            path: path.into(),
            suggestion: suggestion.into(),
        }
    }

    /// Create a parse error
    pub fn parse(
        location: impl Into<String>,
        message: impl Into<String>,
        context: impl Into<String>,
    ) -> Self {
        Self::Parse {
            location: location.into(),
            message: message.into(),
            context: context.into(),
        }
    }

    /// Create a validation error
    pub fn validation(
        field: impl Into<String>,
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        Self::Validation {
            field: field.into(),
            message: message.into(),
            suggestion: suggestion.into(),
        }
    }

    /// Create an IO error
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        let message = source.to_string();
        Self::Io {
            path: path.into(),
            message,
            source,
        }
    }

    /// Create a template error
    pub fn template(
        template: impl Into<String>,
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        Self::Template {
            template: template.into(),
            message: message.into(),
            suggestion: suggestion.into(),
        }
    }

    /// Create a sync error
    pub fn sync(
        editor: impl Into<String>,
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        Self::Sync {
            editor: editor.into(),
            message: message.into(),
            suggestion: suggestion.into(),
        }
    }

    /// Create a security error
    pub fn security(message: impl Into<String>, action: impl Into<String>) -> Self {
        Self::Security {
            message: message.into(),
            action: action.into(),
        }
    }

    /// Create a binary format error
    pub fn binary_format(
        message: impl Into<String>,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        Self::BinaryFormat {
            message: message.into(),
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Create a hook error
    pub fn hook(hook_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Hook {
            hook_id: hook_id.into(),
            message: message.into(),
        }
    }

    /// Create a protocol error
    pub fn protocol(code: i32, message: impl Into<String>) -> Self {
        Self::Protocol {
            code,
            message: message.into(),
        }
    }

    /// Check if this is a recoverable error
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::Config { .. }
                | Self::Validation { .. }
                | Self::Template { .. }
                | Self::Sync { .. }
        )
    }

    /// Get the error code for this error type
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Config { .. } => "E001",
            Self::FileNotFound { .. } => "E002",
            Self::Parse { .. } => "E003",
            Self::Validation { .. } => "E004",
            Self::Io { .. } => "E005",
            Self::Template { .. } => "E006",
            Self::Sync { .. } => "E007",
            Self::Security { .. } => "E008",
            Self::BinaryFormat { .. } => "E009",
            Self::Hook { .. } => "E010",
            Self::Protocol { .. } => "E011",
        }
    }
}

/// Result type alias for enhanced errors
pub type EnhancedResult<T> = std::result::Result<T, EnhancedError>;

/// Extension trait for adding context to errors
pub trait ErrorContext<T> {
    /// Add context to an error
    fn with_context(self, context: impl FnOnce() -> String) -> EnhancedResult<T>;

    /// Add a suggestion to an error
    fn with_suggestion(self, suggestion: impl Into<String>) -> EnhancedResult<T>;
}

impl<T, E: std::error::Error + Send + Sync + 'static> ErrorContext<T> for Result<T, E> {
    fn with_context(self, context: impl FnOnce() -> String) -> EnhancedResult<T> {
        self.map_err(|e| {
            EnhancedError::config_with_source(
                context(),
                "Check the error details for more information",
                e,
            )
        })
    }

    fn with_suggestion(self, suggestion: impl Into<String>) -> EnhancedResult<T> {
        self.map_err(|e| EnhancedError::config_with_source(e.to_string(), suggestion, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_error() {
        let err = EnhancedError::config(
            "Invalid editor configuration",
            "Check that the editor name is one of: cursor, copilot, windsurf",
        );

        assert!(err.to_string().contains("Invalid editor configuration"));
        assert!(err.to_string().contains("Suggestion:"));
        assert_eq!(err.error_code(), "E001");
        assert!(err.is_recoverable());
    }

    #[test]
    fn test_file_not_found_error() {
        let err = EnhancedError::file_not_found(
            "/path/to/missing.drv",
            "Run 'dx driven init' to create the configuration",
        );

        assert!(err.to_string().contains("missing.drv"));
        assert_eq!(err.error_code(), "E002");
    }

    #[test]
    fn test_parse_error() {
        let err = EnhancedError::parse(
            "line 42, column 10",
            "Unexpected token",
            "Expected ':' after key name",
        );

        assert!(err.to_string().contains("line 42"));
        assert_eq!(err.error_code(), "E003");
    }

    #[test]
    fn test_validation_error() {
        let err = EnhancedError::validation(
            "sync.debounce_ms",
            "Value must be positive",
            "Use a value between 100 and 5000",
        );

        assert!(err.to_string().contains("debounce_ms"));
        assert_eq!(err.error_code(), "E004");
        assert!(err.is_recoverable());
    }

    #[test]
    fn test_security_error() {
        let err = EnhancedError::security("Invalid signature", "Re-sign the file with a valid key");

        assert!(err.to_string().contains("Invalid signature"));
        assert_eq!(err.error_code(), "E008");
        assert!(!err.is_recoverable());
    }

    #[test]
    fn test_error_codes_unique() {
        let errors = vec![
            EnhancedError::config("", ""),
            EnhancedError::file_not_found("", ""),
            EnhancedError::parse("", "", ""),
            EnhancedError::validation("", "", ""),
            EnhancedError::template("", "", ""),
            EnhancedError::sync("", "", ""),
            EnhancedError::security("", ""),
            EnhancedError::binary_format("", "", ""),
            EnhancedError::hook("", ""),
            EnhancedError::protocol(0, ""),
        ];

        let codes: Vec<_> = errors.iter().map(|e| e.error_code()).collect();
        let unique_codes: std::collections::HashSet<_> = codes.iter().collect();

        assert_eq!(codes.len(), unique_codes.len(), "Error codes should be unique");
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // Property 15: Error Handling Consistency
    // For any error condition, the error SHALL be properly propagated with
    // a descriptive message and SHALL not cause panics in library code.

    proptest! {
        /// Property: All errors have non-empty messages
        #[test]
        fn prop_errors_have_messages(
            message in ".+",
            suggestion in ".+",
        ) {
            let err = EnhancedError::config(&message, &suggestion);
            let display = err.to_string();

            prop_assert!(!display.is_empty());
            prop_assert!(display.contains(&message) || display.contains("Configuration"));
        }

        /// Property: Error codes are consistent
        #[test]
        fn prop_error_codes_consistent(
            message in ".{1,100}",
        ) {
            let err1 = EnhancedError::config(&message, "suggestion");
            let err2 = EnhancedError::config("different", "suggestion");

            // Same error type should have same code
            prop_assert_eq!(err1.error_code(), err2.error_code());
        }

        /// Property: Recoverable errors are correctly identified
        #[test]
        fn prop_recoverable_errors(
            message in ".{1,100}",
        ) {
            // Config errors should be recoverable
            let config_err = EnhancedError::config(&message, "fix it");
            prop_assert!(config_err.is_recoverable());

            // Security errors should not be recoverable
            let security_err = EnhancedError::security(&message, "action");
            prop_assert!(!security_err.is_recoverable());
        }

        /// Property: Error display never panics
        #[test]
        fn prop_error_display_no_panic(
            message in ".*",
            path in "[a-zA-Z0-9/._-]*",
        ) {
            // These should never panic
            let _ = EnhancedError::config(&message, "suggestion").to_string();
            let _ = EnhancedError::file_not_found(&path, "suggestion").to_string();
            let _ = EnhancedError::parse("loc", &message, "ctx").to_string();
            let _ = EnhancedError::validation("field", &message, "suggestion").to_string();
            let _ = EnhancedError::template("tmpl", &message, "suggestion").to_string();
            let _ = EnhancedError::sync("editor", &message, "suggestion").to_string();
            let _ = EnhancedError::security(&message, "action").to_string();
            let _ = EnhancedError::binary_format(&message, "exp", "act").to_string();
            let _ = EnhancedError::hook("hook", &message).to_string();
            let _ = EnhancedError::protocol(0, &message).to_string();
        }
    }
}
