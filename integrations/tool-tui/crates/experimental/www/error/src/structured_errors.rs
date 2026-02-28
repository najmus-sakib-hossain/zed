//! Structured error types with error codes, messages, and recovery suggestions.
//!
//! This module provides production-ready error types that include:
//! - Unique error codes for programmatic handling
//! - Human-readable messages
//! - Recovery suggestions where applicable
//! - Source error chaining

use std::time::Duration;

/// Production error type with full context.
///
/// Each variant includes structured information for:
/// - Logging and monitoring
/// - User-facing error messages
/// - Programmatic error handling
#[derive(Debug)]
pub enum DxError {
    /// Authentication-related errors
    Auth {
        message: String,
        code: AuthErrorCode,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Database and query errors
    Database {
        message: String,
        code: DatabaseErrorCode,
        query_context: Option<String>,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Configuration errors
    Config {
        message: String,
        key: String,
        suggestion: Option<String>,
    },

    /// Synchronization errors (WebSocket, real-time)
    Sync {
        message: String,
        code: SyncErrorCode,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Internal/unexpected errors
    Internal {
        message: String,
        code: InternalErrorCode,
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Lock acquisition errors
    Lock { message: String, resource: String },

    /// Validation errors
    Validation {
        message: String,
        field: Option<String>,
        suggestion: Option<String>,
    },
}

impl std::fmt::Display for DxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DxError::Auth { message, code, .. } => {
                write!(f, "Authentication error [{}]: {}", code.as_code(), message)
            }
            DxError::Database {
                message,
                code,
                query_context,
                ..
            } => {
                if let Some(ctx) = query_context {
                    write!(f, "Database error [{}]: {} (context: {})", code.as_code(), message, ctx)
                } else {
                    write!(f, "Database error [{}]: {}", code.as_code(), message)
                }
            }
            DxError::Config { message, key, .. } => {
                write!(f, "Configuration error for '{}': {}", key, message)
            }
            DxError::Sync { message, code, .. } => {
                write!(f, "Sync error [{}]: {}", code.as_code(), message)
            }
            DxError::Internal { message, code, .. } => {
                write!(f, "Internal error [{}]: {}", code.as_code(), message)
            }
            DxError::Lock { message, resource } => {
                write!(f, "Lock error for '{}': {}", resource, message)
            }
            DxError::Validation { message, field, .. } => {
                if let Some(f_name) = field {
                    write!(f, "Validation error for '{}': {}", f_name, message)
                } else {
                    write!(f, "Validation error: {}", message)
                }
            }
        }
    }
}

impl std::error::Error for DxError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DxError::Auth { source, .. } => {
                source.as_ref().map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
            }
            DxError::Database { source, .. } => {
                source.as_ref().map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
            }
            DxError::Sync { source, .. } => {
                source.as_ref().map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
            }
            DxError::Internal { source, .. } => {
                source.as_ref().map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
            }
            _ => None,
        }
    }
}

impl DxError {
    /// Create an authentication error.
    pub fn auth(code: AuthErrorCode, message: impl Into<String>) -> Self {
        DxError::Auth {
            message: message.into(),
            code,
            source: None,
        }
    }

    /// Create an authentication error with a source.
    pub fn auth_with_source(
        code: AuthErrorCode,
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        DxError::Auth {
            message: message.into(),
            code,
            source: Some(Box::new(source)),
        }
    }

    /// Create a database error.
    pub fn database(code: DatabaseErrorCode, message: impl Into<String>) -> Self {
        DxError::Database {
            message: message.into(),
            code,
            query_context: None,
            source: None,
        }
    }

    /// Create a database error with query context.
    pub fn database_with_context(
        code: DatabaseErrorCode,
        message: impl Into<String>,
        context: impl Into<String>,
    ) -> Self {
        DxError::Database {
            message: message.into(),
            code,
            query_context: Some(context.into()),
            source: None,
        }
    }

    /// Create a configuration error.
    pub fn config(key: impl Into<String>, message: impl Into<String>) -> Self {
        DxError::Config {
            message: message.into(),
            key: key.into(),
            suggestion: None,
        }
    }

    /// Create a configuration error with a suggestion.
    pub fn config_with_suggestion(
        key: impl Into<String>,
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        DxError::Config {
            message: message.into(),
            key: key.into(),
            suggestion: Some(suggestion.into()),
        }
    }

    /// Create a sync error.
    pub fn sync(code: SyncErrorCode, message: impl Into<String>) -> Self {
        DxError::Sync {
            message: message.into(),
            code,
            source: None,
        }
    }

    /// Create an internal error.
    pub fn internal(code: InternalErrorCode, message: impl Into<String>) -> Self {
        DxError::Internal {
            message: message.into(),
            code,
            source: None,
        }
    }

    /// Create an internal error with a source.
    pub fn internal_with_source(
        code: InternalErrorCode,
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        DxError::Internal {
            message: message.into(),
            code,
            source: Some(Box::new(source)),
        }
    }

    /// Create a lock error.
    pub fn lock(resource: impl Into<String>, message: impl Into<String>) -> Self {
        DxError::Lock {
            message: message.into(),
            resource: resource.into(),
        }
    }

    /// Create a validation error.
    pub fn validation(message: impl Into<String>) -> Self {
        DxError::Validation {
            message: message.into(),
            field: None,
            suggestion: None,
        }
    }

    /// Create a validation error for a specific field.
    pub fn validation_field(field: impl Into<String>, message: impl Into<String>) -> Self {
        DxError::Validation {
            message: message.into(),
            field: Some(field.into()),
            suggestion: None,
        }
    }

    /// Get the error code as a string.
    pub fn error_code(&self) -> String {
        match self {
            DxError::Auth { code, .. } => code.as_code().to_string(),
            DxError::Database { code, .. } => code.as_code().to_string(),
            DxError::Config { .. } => "CONFIG".to_string(),
            DxError::Sync { code, .. } => code.as_code().to_string(),
            DxError::Internal { code, .. } => code.as_code().to_string(),
            DxError::Lock { .. } => "LOCK".to_string(),
            DxError::Validation { .. } => "VALIDATION".to_string(),
        }
    }

    /// Get a recovery suggestion if available.
    pub fn recovery_suggestion(&self) -> Option<&str> {
        match self {
            DxError::Auth { code, .. } => code.recovery_suggestion(),
            DxError::Database { code, .. } => code.recovery_suggestion(),
            DxError::Config { suggestion, .. } => suggestion.as_deref(),
            DxError::Sync { code, .. } => code.recovery_suggestion(),
            DxError::Internal { code, .. } => code.recovery_suggestion(),
            DxError::Lock { .. } => Some("Retry the operation after a short delay"),
            DxError::Validation { suggestion, .. } => suggestion.as_deref(),
        }
    }

    /// Check if this error has a non-empty message.
    pub fn has_message(&self) -> bool {
        match self {
            DxError::Auth { message, .. } => !message.is_empty(),
            DxError::Database { message, .. } => !message.is_empty(),
            DxError::Config { message, .. } => !message.is_empty(),
            DxError::Sync { message, .. } => !message.is_empty(),
            DxError::Internal { message, .. } => !message.is_empty(),
            DxError::Lock { message, .. } => !message.is_empty(),
            DxError::Validation { message, .. } => !message.is_empty(),
        }
    }

    /// Check if this error has a valid error code.
    pub fn has_code(&self) -> bool {
        !self.error_code().is_empty()
    }
}

/// Authentication error codes (1xxx range).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthErrorCode {
    /// Invalid username or password
    InvalidCredentials = 1001,
    /// Token has expired
    TokenExpired = 1002,
    /// Token signature is invalid
    TokenInvalid = 1003,
    /// Token has been revoked
    TokenRevoked = 1004,
    /// Too many authentication attempts
    RateLimited = 1005,
    /// CSRF token validation failed
    CsrfInvalid = 1006,
    /// Missing authentication header
    MissingAuth = 1007,
    /// Insufficient permissions
    Forbidden = 1008,
}

impl AuthErrorCode {
    /// Get the numeric error code.
    pub fn as_code(&self) -> &'static str {
        match self {
            AuthErrorCode::InvalidCredentials => "AUTH_1001",
            AuthErrorCode::TokenExpired => "AUTH_1002",
            AuthErrorCode::TokenInvalid => "AUTH_1003",
            AuthErrorCode::TokenRevoked => "AUTH_1004",
            AuthErrorCode::RateLimited => "AUTH_1005",
            AuthErrorCode::CsrfInvalid => "AUTH_1006",
            AuthErrorCode::MissingAuth => "AUTH_1007",
            AuthErrorCode::Forbidden => "AUTH_1008",
        }
    }

    /// Get a recovery suggestion for this error.
    pub fn recovery_suggestion(&self) -> Option<&'static str> {
        match self {
            AuthErrorCode::InvalidCredentials => Some("Check your username and password"),
            AuthErrorCode::TokenExpired => Some("Please log in again to get a new token"),
            AuthErrorCode::TokenInvalid => Some("Please log in again"),
            AuthErrorCode::TokenRevoked => Some("Please log in again"),
            AuthErrorCode::RateLimited => Some("Wait a few minutes before trying again"),
            AuthErrorCode::CsrfInvalid => Some("Refresh the page and try again"),
            AuthErrorCode::MissingAuth => Some("Include an Authorization header with your request"),
            AuthErrorCode::Forbidden => Some("Contact an administrator for access"),
        }
    }
}

/// Database error codes (2xxx range).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseErrorCode {
    /// Failed to connect to database
    ConnectionFailed = 2001,
    /// Query timed out
    QueryTimeout = 2002,
    /// Constraint violation (unique, foreign key, etc.)
    ConstraintViolation = 2003,
    /// Transaction failed
    TransactionFailed = 2004,
    /// Pool exhausted
    PoolExhausted = 2005,
    /// Query syntax error
    QuerySyntax = 2006,
}

impl DatabaseErrorCode {
    /// Get the string error code.
    pub fn as_code(&self) -> &'static str {
        match self {
            DatabaseErrorCode::ConnectionFailed => "DB_2001",
            DatabaseErrorCode::QueryTimeout => "DB_2002",
            DatabaseErrorCode::ConstraintViolation => "DB_2003",
            DatabaseErrorCode::TransactionFailed => "DB_2004",
            DatabaseErrorCode::PoolExhausted => "DB_2005",
            DatabaseErrorCode::QuerySyntax => "DB_2006",
        }
    }

    /// Get a recovery suggestion for this error.
    pub fn recovery_suggestion(&self) -> Option<&'static str> {
        match self {
            DatabaseErrorCode::ConnectionFailed => Some("Check database connection settings"),
            DatabaseErrorCode::QueryTimeout => Some("Try again or optimize the query"),
            DatabaseErrorCode::ConstraintViolation => Some("Check for duplicate or invalid data"),
            DatabaseErrorCode::TransactionFailed => Some("Retry the operation"),
            DatabaseErrorCode::PoolExhausted => Some("Wait and retry, or increase pool size"),
            DatabaseErrorCode::QuerySyntax => Some("Check the query syntax"),
        }
    }
}

/// Sync/WebSocket error codes (3xxx range).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncErrorCode {
    /// Connection lost
    ConnectionLost = 3001,
    /// Channel not found
    ChannelNotFound = 3002,
    /// Message delivery failed
    DeliveryFailed = 3003,
    /// Buffer overflow
    BufferOverflow = 3004,
    /// Invalid message format
    InvalidMessage = 3005,
    /// Subscription failed
    SubscriptionFailed = 3006,
}

impl SyncErrorCode {
    /// Get the string error code.
    pub fn as_code(&self) -> &'static str {
        match self {
            SyncErrorCode::ConnectionLost => "SYNC_3001",
            SyncErrorCode::ChannelNotFound => "SYNC_3002",
            SyncErrorCode::DeliveryFailed => "SYNC_3003",
            SyncErrorCode::BufferOverflow => "SYNC_3004",
            SyncErrorCode::InvalidMessage => "SYNC_3005",
            SyncErrorCode::SubscriptionFailed => "SYNC_3006",
        }
    }

    /// Get a recovery suggestion for this error.
    pub fn recovery_suggestion(&self) -> Option<&'static str> {
        match self {
            SyncErrorCode::ConnectionLost => Some("Reconnection will be attempted automatically"),
            SyncErrorCode::ChannelNotFound => Some("Check the channel name"),
            SyncErrorCode::DeliveryFailed => Some("Message will be retried"),
            SyncErrorCode::BufferOverflow => Some("Some messages may be lost"),
            SyncErrorCode::InvalidMessage => Some("Check message format"),
            SyncErrorCode::SubscriptionFailed => Some("Try subscribing again"),
        }
    }
}

/// Internal error codes (5xxx range).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InternalErrorCode {
    /// Configuration error
    ConfigError = 5001,
    /// Lock acquisition failed
    LockFailed = 5002,
    /// Resource exhausted
    ResourceExhausted = 5003,
    /// Unexpected state
    UnexpectedState = 5004,
    /// IO error
    IoError = 5005,
    /// Serialization error
    SerializationError = 5006,
}

impl InternalErrorCode {
    /// Get the string error code.
    pub fn as_code(&self) -> &'static str {
        match self {
            InternalErrorCode::ConfigError => "INT_5001",
            InternalErrorCode::LockFailed => "INT_5002",
            InternalErrorCode::ResourceExhausted => "INT_5003",
            InternalErrorCode::UnexpectedState => "INT_5004",
            InternalErrorCode::IoError => "INT_5005",
            InternalErrorCode::SerializationError => "INT_5006",
        }
    }

    /// Get a recovery suggestion for this error.
    pub fn recovery_suggestion(&self) -> Option<&'static str> {
        match self {
            InternalErrorCode::ConfigError => Some("Check configuration settings"),
            InternalErrorCode::LockFailed => Some("Retry after a short delay"),
            InternalErrorCode::ResourceExhausted => Some("Free up resources and retry"),
            InternalErrorCode::UnexpectedState => Some("Restart the application"),
            InternalErrorCode::IoError => Some("Check file permissions and disk space"),
            InternalErrorCode::SerializationError => Some("Check data format"),
        }
    }
}

/// Error recovery configuration.
#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Base delay between retries (exponential backoff)
    pub base_delay: Duration,
    /// Maximum delay cap
    pub max_delay: Duration,
    /// Jitter factor (0.0 - 1.0)
    pub jitter: f64,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            jitter: 0.1,
        }
    }
}

impl RecoveryConfig {
    /// Create a new recovery config with custom settings.
    pub fn new(max_retries: u32, base_delay: Duration, max_delay: Duration, jitter: f64) -> Self {
        Self {
            max_retries,
            base_delay,
            max_delay,
            jitter: jitter.clamp(0.0, 1.0),
        }
    }

    /// Calculate the delay for a given retry attempt (0-indexed).
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let base_ms = self.base_delay.as_millis() as f64;
        let exponential_ms = base_ms * 2.0_f64.powi(attempt as i32);
        let max_ms = self.max_delay.as_millis() as f64;
        let capped_ms = exponential_ms.min(max_ms);

        // Add jitter
        let jitter_range = capped_ms * self.jitter;
        let jitter_offset = jitter_range * (rand_simple() * 2.0 - 1.0);
        let final_ms = (capped_ms + jitter_offset).max(0.0);

        Duration::from_millis(final_ms as u64)
    }

    /// Check if more retries are allowed.
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_retries
    }
}

/// Simple pseudo-random number generator for jitter.
/// Returns a value between 0.0 and 1.0.
fn rand_simple() -> f64 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    nanos as f64 / u32::MAX as f64
}

// Implement From traits for common error types

impl From<std::io::Error> for DxError {
    fn from(err: std::io::Error) -> Self {
        DxError::internal_with_source(InternalErrorCode::IoError, format!("IO error: {}", err), err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_error_creation() {
        let err = DxError::auth(AuthErrorCode::InvalidCredentials, "Bad password");
        assert!(err.has_message());
        assert!(err.has_code());
        assert_eq!(err.error_code(), "AUTH_1001");
        assert!(err.recovery_suggestion().is_some());
    }

    #[test]
    fn test_database_error_with_context() {
        let err = DxError::database_with_context(
            DatabaseErrorCode::QueryTimeout,
            "Query took too long",
            "SELECT * FROM users",
        );
        assert!(err.has_message());
        assert_eq!(err.error_code(), "DB_2002");
        let display = format!("{}", err);
        assert!(display.contains("SELECT * FROM users"));
    }

    #[test]
    fn test_config_error_with_suggestion() {
        let err = DxError::config_with_suggestion(
            "DATABASE_URL",
            "Missing required configuration",
            "Set DATABASE_URL environment variable",
        );
        assert!(err.has_message());
        assert_eq!(err.recovery_suggestion(), Some("Set DATABASE_URL environment variable"));
    }

    #[test]
    fn test_sync_error() {
        let err = DxError::sync(SyncErrorCode::ConnectionLost, "WebSocket disconnected");
        assert!(err.has_message());
        assert_eq!(err.error_code(), "SYNC_3001");
    }

    #[test]
    fn test_internal_error() {
        let err = DxError::internal(InternalErrorCode::UnexpectedState, "Invalid state transition");
        assert!(err.has_message());
        assert_eq!(err.error_code(), "INT_5004");
    }

    #[test]
    fn test_lock_error() {
        let err = DxError::lock("user_cache", "Failed to acquire lock");
        assert!(err.has_message());
        assert_eq!(err.error_code(), "LOCK");
    }

    #[test]
    fn test_validation_error() {
        let err = DxError::validation_field("email", "Invalid email format");
        assert!(err.has_message());
        let display = format!("{}", err);
        assert!(display.contains("email"));
    }

    #[test]
    fn test_recovery_config_default() {
        let config = RecoveryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert!(config.should_retry(0));
        assert!(config.should_retry(2));
        assert!(!config.should_retry(3));
    }

    #[test]
    fn test_recovery_config_delay() {
        let config = RecoveryConfig::new(
            3,
            Duration::from_millis(100),
            Duration::from_secs(10),
            0.0, // No jitter for predictable testing
        );

        let delay0 = config.delay_for_attempt(0);
        let delay1 = config.delay_for_attempt(1);
        let delay2 = config.delay_for_attempt(2);

        // Exponential backoff: 100ms, 200ms, 400ms
        assert_eq!(delay0.as_millis(), 100);
        assert_eq!(delay1.as_millis(), 200);
        assert_eq!(delay2.as_millis(), 400);
    }

    #[test]
    fn test_recovery_config_max_delay() {
        let config =
            RecoveryConfig::new(10, Duration::from_millis(100), Duration::from_millis(500), 0.0);

        // After several attempts, should cap at max_delay
        let delay = config.delay_for_attempt(10);
        assert!(delay.as_millis() <= 500);
    }

    #[test]
    fn test_error_display() {
        let err = DxError::auth(AuthErrorCode::TokenExpired, "Token has expired");
        let display = format!("{}", err);
        assert!(display.contains("AUTH_1002"));
        assert!(display.contains("Token has expired"));
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let dx_err: DxError = io_err.into();
        assert_eq!(dx_err.error_code(), "INT_5005");
    }

    #[test]
    fn test_all_auth_codes_have_suggestions() {
        let codes = [
            AuthErrorCode::InvalidCredentials,
            AuthErrorCode::TokenExpired,
            AuthErrorCode::TokenInvalid,
            AuthErrorCode::TokenRevoked,
            AuthErrorCode::RateLimited,
            AuthErrorCode::CsrfInvalid,
            AuthErrorCode::MissingAuth,
            AuthErrorCode::Forbidden,
        ];

        for code in codes {
            assert!(code.recovery_suggestion().is_some(), "Missing suggestion for {:?}", code);
        }
    }

    #[test]
    fn test_all_database_codes_have_suggestions() {
        let codes = [
            DatabaseErrorCode::ConnectionFailed,
            DatabaseErrorCode::QueryTimeout,
            DatabaseErrorCode::ConstraintViolation,
            DatabaseErrorCode::TransactionFailed,
            DatabaseErrorCode::PoolExhausted,
            DatabaseErrorCode::QuerySyntax,
        ];

        for code in codes {
            assert!(code.recovery_suggestion().is_some(), "Missing suggestion for {:?}", code);
        }
    }

    #[test]
    fn test_all_sync_codes_have_suggestions() {
        let codes = [
            SyncErrorCode::ConnectionLost,
            SyncErrorCode::ChannelNotFound,
            SyncErrorCode::DeliveryFailed,
            SyncErrorCode::BufferOverflow,
            SyncErrorCode::InvalidMessage,
            SyncErrorCode::SubscriptionFailed,
        ];

        for code in codes {
            assert!(code.recovery_suggestion().is_some(), "Missing suggestion for {:?}", code);
        }
    }

    #[test]
    fn test_all_internal_codes_have_suggestions() {
        let codes = [
            InternalErrorCode::ConfigError,
            InternalErrorCode::LockFailed,
            InternalErrorCode::ResourceExhausted,
            InternalErrorCode::UnexpectedState,
            InternalErrorCode::IoError,
            InternalErrorCode::SerializationError,
        ];

        for code in codes {
            assert!(code.recovery_suggestion().is_some(), "Missing suggestion for {:?}", code);
        }
    }
}
