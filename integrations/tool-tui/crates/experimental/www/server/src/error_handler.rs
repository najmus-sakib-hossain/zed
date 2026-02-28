//! Error response sanitization for dx-server.
//!
//! This module provides environment-aware error handling that:
//! - Hides internal details in production (stack traces, paths, config)
//! - Shows full diagnostics in development for debugging
//! - Generates consistent error responses with request IDs

use axum::{http::StatusCode, response::Response};
use std::sync::Arc;
use uuid::Uuid;

use crate::error_pages::ErrorPageConfig;

/// Environment mode for error handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    /// Production mode - hide internal details
    Production,
    /// Development mode - show full diagnostics
    Development,
}

impl Environment {
    /// Detect environment from RUST_ENV or DX_ENV environment variables.
    ///
    /// Returns `Development` if either variable is set to "development" or "dev",
    /// otherwise returns `Production`.
    pub fn from_env() -> Self {
        let rust_env = std::env::var("RUST_ENV").unwrap_or_default().to_lowercase();
        let dx_env = std::env::var("DX_ENV").unwrap_or_default().to_lowercase();

        if rust_env == "development"
            || rust_env == "dev"
            || dx_env == "development"
            || dx_env == "dev"
        {
            Environment::Development
        } else {
            Environment::Production
        }
    }
}

/// Error handler that sanitizes error responses based on environment.
///
/// In production mode:
/// - Stack traces are hidden
/// - Internal file paths are hidden
/// - Configuration details are hidden
/// - Generic error messages are shown
///
/// In development mode:
/// - Full stack traces are shown
/// - File paths are shown
/// - Detailed error context is provided
#[derive(Clone)]
pub struct ErrorHandler {
    environment: Environment,
    error_page_config: Arc<ErrorPageConfig>,
}

impl Default for ErrorHandler {
    fn default() -> Self {
        Self::from_environment()
    }
}

impl ErrorHandler {
    /// Create a new error handler with the specified environment.
    pub fn new(environment: Environment) -> Self {
        let error_page_config = match environment {
            Environment::Production => ErrorPageConfig::production(),
            Environment::Development => ErrorPageConfig::development(),
        };

        Self {
            environment,
            error_page_config: Arc::new(error_page_config),
        }
    }

    /// Create an error handler by detecting the environment from env vars.
    pub fn from_environment() -> Self {
        Self::new(Environment::from_env())
    }

    /// Create a production error handler.
    pub fn production() -> Self {
        Self::new(Environment::Production)
    }

    /// Create a development error handler.
    pub fn development() -> Self {
        Self::new(Environment::Development)
    }

    /// Get the current environment.
    pub fn environment(&self) -> Environment {
        self.environment
    }

    /// Check if running in production mode.
    pub fn is_production(&self) -> bool {
        self.environment == Environment::Production
    }

    /// Check if running in development mode.
    pub fn is_development(&self) -> bool {
        self.environment == Environment::Development
    }

    /// Handle an internal server error.
    ///
    /// In production, returns a generic error message.
    /// In development, includes full error details.
    pub fn internal_error(&self, error: &dyn std::error::Error, request_id: &str) -> Response {
        let message = if self.is_production() {
            "An internal server error occurred. Please try again later.".to_string()
        } else {
            error.to_string()
        };

        let details = if self.is_development() {
            Some(self.format_error_chain(error))
        } else {
            None
        };

        // Log the full error regardless of environment
        tracing::error!(
            request_id = %request_id,
            error = %error,
            "Internal server error"
        );

        self.error_page_config.render(
            StatusCode::INTERNAL_SERVER_ERROR,
            &message,
            details.as_deref(),
            request_id,
        )
    }

    /// Handle a not found error.
    pub fn not_found(&self, path: &str, request_id: &str) -> Response {
        let message = if self.is_production() {
            "The requested resource was not found.".to_string()
        } else {
            format!("Resource not found: {}", path)
        };

        tracing::warn!(
            request_id = %request_id,
            path = %path,
            "Resource not found"
        );

        self.error_page_config.render(StatusCode::NOT_FOUND, &message, None, request_id)
    }

    /// Handle a bad request error.
    pub fn bad_request(&self, reason: &str, request_id: &str) -> Response {
        let message = if self.is_production() {
            "Invalid request.".to_string()
        } else {
            format!("Bad request: {}", reason)
        };

        tracing::warn!(
            request_id = %request_id,
            reason = %reason,
            "Bad request"
        );

        self.error_page_config
            .render(StatusCode::BAD_REQUEST, &message, None, request_id)
    }

    /// Handle an unauthorized error.
    pub fn unauthorized(&self, reason: &str, request_id: &str) -> Response {
        // Never expose auth failure reasons in production
        let message = if self.is_production() {
            "Authentication required.".to_string()
        } else {
            format!("Unauthorized: {}", reason)
        };

        tracing::warn!(
            request_id = %request_id,
            reason = %reason,
            "Unauthorized access attempt"
        );

        self.error_page_config
            .render(StatusCode::UNAUTHORIZED, &message, None, request_id)
    }

    /// Handle a forbidden error.
    pub fn forbidden(&self, reason: &str, request_id: &str) -> Response {
        let message = if self.is_production() {
            "Access denied.".to_string()
        } else {
            format!("Forbidden: {}", reason)
        };

        tracing::warn!(
            request_id = %request_id,
            reason = %reason,
            "Forbidden access attempt"
        );

        self.error_page_config.render(StatusCode::FORBIDDEN, &message, None, request_id)
    }

    /// Handle a service unavailable error.
    pub fn service_unavailable(&self, reason: &str, request_id: &str) -> Response {
        let message = if self.is_production() {
            "Service temporarily unavailable. Please try again later.".to_string()
        } else {
            format!("Service unavailable: {}", reason)
        };

        tracing::error!(
            request_id = %request_id,
            reason = %reason,
            "Service unavailable"
        );

        self.error_page_config
            .render(StatusCode::SERVICE_UNAVAILABLE, &message, None, request_id)
    }

    /// Create a generic error response with sanitization.
    ///
    /// This method sanitizes the error message and details based on the environment.
    pub fn error_response(
        &self,
        status: StatusCode,
        message: &str,
        details: Option<&str>,
        request_id: &str,
    ) -> Response {
        let sanitized_message = if self.is_production() {
            self.sanitize_message(message)
        } else {
            message.to_string()
        };

        let sanitized_details = if self.is_development() {
            details.map(|d| d.to_string())
        } else {
            None
        };

        self.error_page_config.render(
            status,
            &sanitized_message,
            sanitized_details.as_deref(),
            request_id,
        )
    }

    /// Generate a new request ID.
    pub fn generate_request_id(&self) -> String {
        Uuid::new_v4().to_string()
    }

    /// Sanitize an error message for production.
    ///
    /// Removes potentially sensitive information:
    /// - File paths
    /// - Stack traces
    /// - Internal configuration details
    /// - Database connection strings
    pub fn sanitize_message(&self, message: &str) -> String {
        // Check for common sensitive patterns and replace with generic messages
        let lower = message.to_lowercase();

        if lower.contains("stack trace") || lower.contains("backtrace") {
            return "An internal error occurred.".to_string();
        }

        if lower.contains("connection")
            && (lower.contains("database") || lower.contains("postgres") || lower.contains("mysql"))
        {
            return "A database error occurred.".to_string();
        }

        if lower.contains("password")
            || lower.contains("secret")
            || lower.contains("token")
            || lower.contains("api_key")
        {
            return "An authentication error occurred.".to_string();
        }

        // Remove file paths (Unix and Windows)
        let sanitized = self.remove_file_paths(message);

        // If the message is too long, truncate it
        if sanitized.len() > 200 {
            format!("{}...", &sanitized[..197])
        } else {
            sanitized
        }
    }

    /// Remove file paths from a message.
    fn remove_file_paths(&self, message: &str) -> String {
        use once_cell::sync::Lazy;

        // Compile regex patterns once and reuse
        static UNIX_PATH_PATTERN: Lazy<Option<regex_lite::Regex>> =
            Lazy::new(|| regex_lite::Regex::new(r"/[a-zA-Z0-9_\-/]+\.[a-zA-Z]+").ok());
        static WINDOWS_PATH_PATTERN: Lazy<Option<regex_lite::Regex>> =
            Lazy::new(|| regex_lite::Regex::new(r"[A-Za-z]:\\[a-zA-Z0-9_\-\\/]+\.[a-zA-Z]+").ok());

        let mut result = message.to_string();

        // Unix-style paths (e.g., /home/user/file.txt)
        if let Some(ref pattern) = *UNIX_PATH_PATTERN {
            result = pattern.replace_all(&result, "[path]").to_string();
        }

        // Windows-style paths (e.g., C:\Users\admin\file.txt)
        if let Some(ref pattern) = *WINDOWS_PATH_PATTERN {
            result = pattern.replace_all(&result, "[path]").to_string();
        }

        result
    }

    /// Format an error chain for development mode.
    fn format_error_chain(&self, error: &dyn std::error::Error) -> String {
        let mut chain = Vec::new();
        chain.push(format!("Error: {}", error));

        let mut source = error.source();
        while let Some(err) = source {
            chain.push(format!("Caused by: {}", err));
            source = err.source();
        }

        chain.join("\n")
    }
}

/// Sensitive patterns that should be sanitized in production error messages.
pub const SENSITIVE_PATTERNS: &[&str] = &[
    "password",
    "secret",
    "token",
    "api_key",
    "apikey",
    "auth",
    "credential",
    "private_key",
    "privatekey",
    "connection_string",
    "connectionstring",
    "database_url",
    "db_url",
];

/// Check if a string contains sensitive information.
pub fn contains_sensitive_info(s: &str) -> bool {
    let lower = s.to_lowercase();
    SENSITIVE_PATTERNS.iter().any(|pattern| lower.contains(pattern))
}

/// Sanitize a string by removing sensitive information.
///
/// This is a utility function for sanitizing arbitrary strings.
pub fn sanitize_string(s: &str) -> String {
    if contains_sensitive_info(s) {
        "[REDACTED]".to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_production_handler_hides_details() {
        let handler = ErrorHandler::production();
        assert!(handler.is_production());
        assert!(!handler.is_development());
    }

    #[test]
    fn test_development_handler_shows_details() {
        let handler = ErrorHandler::development();
        assert!(handler.is_development());
        assert!(!handler.is_production());
    }

    #[test]
    fn test_sanitize_message_removes_paths() {
        let handler = ErrorHandler::production();

        let message = "Error reading file /home/user/config/settings.json";
        let sanitized = handler.sanitize_message(message);
        assert!(!sanitized.contains("/home/user"));
        assert!(sanitized.contains("[path]"));
    }

    #[test]
    fn test_sanitize_message_removes_windows_paths() {
        let handler = ErrorHandler::production();

        let message = "Error reading file C:\\Users\\admin\\config.txt";
        let sanitized = handler.sanitize_message(message);
        assert!(!sanitized.contains("C:\\Users"));
        assert!(sanitized.contains("[path]"));
    }

    #[test]
    fn test_sanitize_message_hides_database_errors() {
        let handler = ErrorHandler::production();

        let message = "Connection to database postgres://user:pass@localhost failed";
        let sanitized = handler.sanitize_message(message);
        assert_eq!(sanitized, "A database error occurred.");
    }

    #[test]
    fn test_sanitize_message_hides_password_errors() {
        let handler = ErrorHandler::production();

        let message = "Invalid password for user admin";
        let sanitized = handler.sanitize_message(message);
        assert_eq!(sanitized, "An authentication error occurred.");
    }

    #[test]
    fn test_sanitize_message_truncates_long_messages() {
        let handler = ErrorHandler::production();

        let message = "a".repeat(300);
        let sanitized = handler.sanitize_message(&message);
        assert!(sanitized.len() <= 200);
        assert!(sanitized.ends_with("..."));
    }

    #[test]
    fn test_contains_sensitive_info() {
        assert!(contains_sensitive_info("password=secret123"));
        assert!(contains_sensitive_info("API_KEY: abc123"));
        assert!(contains_sensitive_info("database_url=postgres://..."));
        assert!(!contains_sensitive_info("Hello world"));
    }

    #[test]
    fn test_sanitize_string() {
        assert_eq!(sanitize_string("password=secret"), "[REDACTED]");
        assert_eq!(sanitize_string("Hello world"), "Hello world");
    }

    #[test]
    fn test_internal_error_production() {
        let handler = ErrorHandler::production();
        let error =
            std::io::Error::new(std::io::ErrorKind::NotFound, "File not found at /secret/path");
        let request_id = "test-123";

        let response = handler.internal_error(&error, request_id);
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_internal_error_development() {
        let handler = ErrorHandler::development();
        let error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let request_id = "test-123";

        let response = handler.internal_error(&error, request_id);
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_not_found() {
        let handler = ErrorHandler::production();
        let response = handler.not_found("/api/users/123", "test-123");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_unauthorized() {
        let handler = ErrorHandler::production();
        let response = handler.unauthorized("Invalid token", "test-123");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_forbidden() {
        let handler = ErrorHandler::production();
        let response = handler.forbidden("Insufficient permissions", "test-123");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn test_generate_request_id() {
        let handler = ErrorHandler::production();
        let id1 = handler.generate_request_id();
        let id2 = handler.generate_request_id();

        assert_ne!(id1, id2);
        assert!(Uuid::parse_str(&id1).is_ok());
        assert!(Uuid::parse_str(&id2).is_ok());
    }
}
