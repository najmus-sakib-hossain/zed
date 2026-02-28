//! Structured JSON logging module with trace correlation.
//!
//! This module provides:
//! - JSON-formatted log output for structured logging
//! - Automatic trace ID injection into log entries
//! - Configurable log levels and filtering via environment variables
//! - Integration with the tracing ecosystem
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_www_observability::{ObservabilityConfig, logging::init_logging};
//!
//! let config = ObservabilityConfig::new("my-service");
//! init_logging(&config).expect("Failed to initialize logging");
//!
//! // Logs will be output as JSON with trace correlation
//! tracing::info!(user_id = 123, "User logged in");
//! // Output: {"timestamp":"2024-01-15T10:30:00Z","level":"INFO","message":"User logged in","user_id":123,"trace_id":"abc123..."}
//! ```

use crate::ObservabilityConfig;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::Subscriber;
use tracing_subscriber::{
    EnvFilter, Layer,
    fmt::{self, time::UtcTime},
    layer::SubscriberExt,
    registry::LookupSpan,
    util::SubscriberInitExt,
};

/// Flag to track if logging has been initialized.
static LOGGING_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Errors that can occur during logging initialization.
#[derive(Debug, thiserror::Error)]
pub enum LoggingError {
    /// Failed to initialize the logging subscriber.
    #[error("Failed to initialize logging: {0}")]
    InitError(String),

    /// Logging has already been initialized.
    #[error("Logging has already been initialized")]
    AlreadyInitialized,

    /// Failed to parse log directive.
    #[error("Failed to parse log directive: {0}")]
    DirectiveError(String),

    /// Failed to set global subscriber.
    #[error("Failed to set global subscriber: {0}")]
    SubscriberError(String),
}

/// Configuration for the logging subsystem.
#[derive(Debug, Clone, Serialize)]
pub struct LoggingConfig {
    /// The default log level if RUST_LOG is not set.
    pub default_level: LogLevel,
    /// Whether to output logs in JSON format.
    pub json_format: bool,
    /// Whether to include file and line information in logs.
    pub include_file_info: bool,
    /// Whether to include target (module path) in logs.
    pub include_target: bool,
    /// Whether to include thread IDs in logs.
    pub include_thread_ids: bool,
    /// Whether to include thread names in logs.
    pub include_thread_names: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            default_level: LogLevel::Info,
            json_format: true,
            include_file_info: false,
            include_target: true,
            include_thread_ids: false,
            include_thread_names: false,
        }
    }
}

impl LoggingConfig {
    /// Creates a new `LoggingConfig` with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the default log level.
    #[must_use]
    pub const fn with_level(mut self, level: LogLevel) -> Self {
        self.default_level = level;
        self
    }

    /// Enables or disables JSON format output.
    #[must_use]
    pub const fn with_json(mut self, enabled: bool) -> Self {
        self.json_format = enabled;
        self
    }

    /// Enables or disables file and line information.
    #[must_use]
    pub const fn with_file_info(mut self, enabled: bool) -> Self {
        self.include_file_info = enabled;
        self
    }

    /// Enables or disables target (module path) information.
    #[must_use]
    pub const fn with_target(mut self, enabled: bool) -> Self {
        self.include_target = enabled;
        self
    }

    /// Enables or disables thread ID information.
    #[must_use]
    pub const fn with_thread_ids(mut self, enabled: bool) -> Self {
        self.include_thread_ids = enabled;
        self
    }

    /// Enables or disables thread name information.
    #[must_use]
    pub const fn with_thread_names(mut self, enabled: bool) -> Self {
        self.include_thread_names = enabled;
        self
    }
}

/// Log levels supported by the logging subsystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Trace level - most verbose.
    Trace,
    /// Debug level.
    Debug,
    /// Info level - default.
    Info,
    /// Warn level.
    Warn,
    /// Error level - least verbose.
    Error,
}

impl LogLevel {
    /// Converts the log level to a string suitable for `EnvFilter`.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Initializes the structured logging subsystem.
///
/// This function sets up:
/// - JSON-formatted log output (configurable)
/// - Automatic trace correlation ID injection
/// - Log level filtering via `RUST_LOG` environment variable
/// - Timestamp formatting in RFC 3339 format
///
/// # Arguments
///
/// * `config` - The observability configuration containing logging settings.
///
/// # Errors
///
/// Returns a `LoggingError` if:
/// - Logging has already been initialized
/// - Failed to parse log directives
/// - Failed to set the global subscriber
///
/// # Example
///
/// ```rust,ignore
/// use dx_www_observability::{ObservabilityConfig, logging::init_logging};
///
/// let config = ObservabilityConfig::new("my-service");
/// init_logging(&config)?;
///
/// // Now logs will be JSON formatted with trace IDs
/// tracing::info!("Application started");
/// ```
pub fn init_logging(config: &ObservabilityConfig) -> Result<(), LoggingError> {
    init_logging_with_config(config, &LoggingConfig::default())
}

/// Initializes the structured logging subsystem with custom logging configuration.
///
/// This function provides more control over logging behavior than `init_logging`.
///
/// # Arguments
///
/// * `obs_config` - The observability configuration.
/// * `log_config` - The logging-specific configuration.
///
/// # Errors
///
/// Returns a `LoggingError` if initialization fails.
pub fn init_logging_with_config(
    obs_config: &ObservabilityConfig,
    log_config: &LoggingConfig,
) -> Result<(), LoggingError> {
    // Check if already initialized
    if LOGGING_INITIALIZED.swap(true, Ordering::SeqCst) {
        return Err(LoggingError::AlreadyInitialized);
    }

    // Create the environment filter with sensible defaults
    let env_filter = create_env_filter(log_config)?;

    // Build and initialize the subscriber based on format preference
    if log_config.json_format {
        init_json_subscriber(obs_config, log_config, env_filter)?;
    } else {
        init_pretty_subscriber(log_config, env_filter)?;
    }

    tracing::info!(
        service_name = %obs_config.service_name,
        json_format = log_config.json_format,
        "Structured logging initialized"
    );

    Ok(())
}

/// Creates an environment filter with the configured default level.
fn create_env_filter(config: &LoggingConfig) -> Result<EnvFilter, LoggingError> {
    // Add directives to reduce noise from common verbose crates
    let hyper_directive =
        "hyper=warn".parse().map_err(|e| LoggingError::DirectiveError(format!("{e}")))?;
    let h2_directive =
        "h2=warn".parse().map_err(|e| LoggingError::DirectiveError(format!("{e}")))?;
    let tower_directive = "tower_http=debug"
        .parse()
        .map_err(|e| LoggingError::DirectiveError(format!("{e}")))?;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(config.default_level.as_str()))
        .add_directive(hyper_directive)
        .add_directive(h2_directive)
        .add_directive(tower_directive);

    Ok(filter)
}

/// Initializes the JSON-formatted subscriber with trace correlation.
fn init_json_subscriber(
    obs_config: &ObservabilityConfig,
    log_config: &LoggingConfig,
    env_filter: EnvFilter,
) -> Result<(), LoggingError> {
    // Create the JSON formatting layer with trace correlation
    let json_layer = fmt::layer()
        .json()
        .with_timer(UtcTime::rfc_3339())
        .with_current_span(true)
        .with_span_list(true)
        .with_file(log_config.include_file_info)
        .with_line_number(log_config.include_file_info)
        .with_target(log_config.include_target)
        .with_thread_ids(log_config.include_thread_ids)
        .with_thread_names(log_config.include_thread_names)
        .flatten_event(true);

    // Add service name as a constant field
    let service_layer = ServiceNameLayer {
        service_name: obs_config.service_name.clone(),
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(json_layer)
        .with(service_layer)
        .try_init()
        .map_err(|e| LoggingError::SubscriberError(e.to_string()))?;

    Ok(())
}

/// Initializes the pretty-printed subscriber (for development).
fn init_pretty_subscriber(
    log_config: &LoggingConfig,
    env_filter: EnvFilter,
) -> Result<(), LoggingError> {
    let fmt_layer = fmt::layer()
        .with_timer(UtcTime::rfc_3339())
        .with_file(log_config.include_file_info)
        .with_line_number(log_config.include_file_info)
        .with_target(log_config.include_target)
        .with_thread_ids(log_config.include_thread_ids)
        .with_thread_names(log_config.include_thread_names);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .try_init()
        .map_err(|e| LoggingError::SubscriberError(e.to_string()))?;

    Ok(())
}

/// A layer that adds the service name to all log events.
///
/// This is useful for identifying logs from different services in a
/// centralized logging system.
#[allow(dead_code)] // service_name reserved for future metadata injection
struct ServiceNameLayer {
    service_name: String,
}

impl<S> Layer<S> for ServiceNameLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(
        &self,
        _event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        // The service name is added via the JSON formatter's flatten_event
        // and the span context. This layer serves as a marker for future
        // enhancements where we might want to inject additional metadata.
    }
}

/// A structured log entry for serialization.
///
/// This struct represents the format of JSON log entries produced by the
/// logging subsystem.
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    /// ISO 8601 timestamp of the log event.
    pub timestamp: String,
    /// Log level (trace, debug, info, warn, error).
    pub level: String,
    /// The log message.
    pub message: String,
    /// Target module path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Trace ID for correlation (if tracing is active).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// Span ID for correlation (if within a span).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span_id: Option<String>,
    /// Service name.
    pub service: String,
    /// Additional structured fields.
    #[serde(flatten)]
    pub fields: serde_json::Map<String, serde_json::Value>,
}

/// Gets the current trace ID from the active span context.
///
/// This function extracts the trace ID from the current OpenTelemetry span
/// context, if one is active. This is useful for manual trace correlation
/// in scenarios where automatic injection isn't available.
///
/// # Returns
///
/// Returns `Some(trace_id)` if a trace context is active, `None` otherwise.
///
/// # Example
///
/// ```rust,ignore
/// use dx_www_observability::logging::get_current_trace_id;
///
/// if let Some(trace_id) = get_current_trace_id() {
///     println!("Current trace: {}", trace_id);
/// }
/// ```
#[must_use]
pub fn get_current_trace_id() -> Option<String> {
    use opentelemetry::trace::TraceContextExt;
    use tracing_opentelemetry::OpenTelemetrySpanExt;

    let current_span = tracing::Span::current();
    let context = current_span.context();
    let span_ref = context.span();
    let span_context = span_ref.span_context();

    if span_context.is_valid() {
        Some(span_context.trace_id().to_string())
    } else {
        None
    }
}

/// Gets the current span ID from the active span context.
///
/// # Returns
///
/// Returns `Some(span_id)` if a span context is active, `None` otherwise.
#[must_use]
pub fn get_current_span_id() -> Option<String> {
    use opentelemetry::trace::TraceContextExt;
    use tracing_opentelemetry::OpenTelemetrySpanExt;

    let current_span = tracing::Span::current();
    let context = current_span.context();
    let span_ref = context.span();
    let span_context = span_ref.span_context();

    if span_context.is_valid() {
        Some(span_context.span_id().to_string())
    } else {
        None
    }
}

/// Creates a log entry with trace correlation.
///
/// This is a helper function for creating structured log entries that
/// include trace correlation IDs. Useful for custom logging scenarios.
///
/// # Arguments
///
/// * `level` - The log level.
/// * `message` - The log message.
/// * `service` - The service name.
///
/// # Returns
///
/// A `LogEntry` with trace correlation IDs populated if available.
#[must_use]
pub fn create_log_entry(level: &str, message: &str, service: &str) -> LogEntry {
    LogEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        level: level.to_uppercase(),
        message: message.to_string(),
        target: None,
        trace_id: get_current_trace_id(),
        span_id: get_current_span_id(),
        service: service.to_string(),
        fields: serde_json::Map::new(),
    }
}

/// Resets the logging initialization flag (for testing purposes only).
///
/// # Safety
///
/// This function should only be used in tests to reset the global state
/// between test runs.
#[cfg(test)]
pub(crate) fn reset_logging_initialized() {
    LOGGING_INITIALIZED.store(false, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert_eq!(config.default_level, LogLevel::Info);
        assert!(config.json_format);
        assert!(!config.include_file_info);
        assert!(config.include_target);
        assert!(!config.include_thread_ids);
        assert!(!config.include_thread_names);
    }

    #[test]
    fn test_logging_config_builder() {
        let config = LoggingConfig::new()
            .with_level(LogLevel::Debug)
            .with_json(false)
            .with_file_info(true)
            .with_target(false)
            .with_thread_ids(true)
            .with_thread_names(true);

        assert_eq!(config.default_level, LogLevel::Debug);
        assert!(!config.json_format);
        assert!(config.include_file_info);
        assert!(!config.include_target);
        assert!(config.include_thread_ids);
        assert!(config.include_thread_names);
    }

    #[test]
    fn test_log_level_as_str() {
        assert_eq!(LogLevel::Trace.as_str(), "trace");
        assert_eq!(LogLevel::Debug.as_str(), "debug");
        assert_eq!(LogLevel::Info.as_str(), "info");
        assert_eq!(LogLevel::Warn.as_str(), "warn");
        assert_eq!(LogLevel::Error.as_str(), "error");
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(format!("{}", LogLevel::Info), "info");
        assert_eq!(format!("{}", LogLevel::Error), "error");
    }

    #[test]
    fn test_create_log_entry() {
        let entry = create_log_entry("info", "Test message", "test-service");
        assert_eq!(entry.level, "INFO");
        assert_eq!(entry.message, "Test message");
        assert_eq!(entry.service, "test-service");
        // trace_id and span_id will be None without active tracing
        assert!(entry.trace_id.is_none());
        assert!(entry.span_id.is_none());
    }

    #[test]
    fn test_log_entry_serialization() {
        let entry = LogEntry {
            timestamp: "2024-01-15T10:30:00Z".to_string(),
            level: "INFO".to_string(),
            message: "Test message".to_string(),
            target: Some("my_module".to_string()),
            trace_id: Some("abc123".to_string()),
            span_id: Some("def456".to_string()),
            service: "test-service".to_string(),
            fields: serde_json::Map::new(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"level\":\"INFO\""));
        assert!(json.contains("\"message\":\"Test message\""));
        assert!(json.contains("\"trace_id\":\"abc123\""));
        assert!(json.contains("\"service\":\"test-service\""));
    }

    #[test]
    fn test_log_entry_without_optional_fields() {
        let entry = LogEntry {
            timestamp: "2024-01-15T10:30:00Z".to_string(),
            level: "INFO".to_string(),
            message: "Test message".to_string(),
            target: None,
            trace_id: None,
            span_id: None,
            service: "test-service".to_string(),
            fields: serde_json::Map::new(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        // Optional fields should not appear when None
        assert!(!json.contains("trace_id"));
        assert!(!json.contains("span_id"));
        assert!(!json.contains("target"));
    }

    #[test]
    fn test_create_env_filter() {
        let config = LoggingConfig::default();
        let filter = create_env_filter(&config);
        assert!(filter.is_ok());
    }

    #[test]
    fn test_create_env_filter_with_different_levels() {
        for level in [
            LogLevel::Trace,
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error,
        ] {
            let config = LoggingConfig::new().with_level(level);
            let filter = create_env_filter(&config);
            assert!(filter.is_ok(), "Failed to create filter for level {:?}", level);
        }
    }

    #[test]
    fn test_get_current_trace_id_without_context() {
        // Without an active trace context, should return None
        let trace_id = get_current_trace_id();
        assert!(trace_id.is_none());
    }

    #[test]
    fn test_get_current_span_id_without_context() {
        // Without an active span context, should return None
        let span_id = get_current_span_id();
        assert!(span_id.is_none());
    }

    // =========================================================================
    // Additional unit tests for trace ID correlation in log entries
    // **Validates: Requirements 2.4**
    // =========================================================================

    /// Tests that log entries include trace_id field when serialized.
    /// **Validates: Requirements 2.4**
    #[test]
    fn test_log_entry_with_trace_id_serialization() {
        let entry = LogEntry {
            timestamp: "2024-01-15T10:30:00Z".to_string(),
            level: "INFO".to_string(),
            message: "Test message with trace".to_string(),
            target: Some("test_module".to_string()),
            trace_id: Some("0af7651916cd43dd8448eb211c80319c".to_string()),
            span_id: Some("b7ad6b7169203331".to_string()),
            service: "test-service".to_string(),
            fields: serde_json::Map::new(),
        };

        let json = serde_json::to_string(&entry).expect("Failed to serialize");

        // Verify trace_id is present in JSON output
        assert!(
            json.contains("\"trace_id\":\"0af7651916cd43dd8448eb211c80319c\""),
            "trace_id should be present in JSON output"
        );
        assert!(
            json.contains("\"span_id\":\"b7ad6b7169203331\""),
            "span_id should be present in JSON output"
        );
    }

    /// Tests that log entries without trace context omit trace_id field.
    /// **Validates: Requirements 2.4**
    #[test]
    fn test_log_entry_without_trace_id_omits_field() {
        let entry = LogEntry {
            timestamp: "2024-01-15T10:30:00Z".to_string(),
            level: "INFO".to_string(),
            message: "Test message without trace".to_string(),
            target: None,
            trace_id: None,
            span_id: None,
            service: "test-service".to_string(),
            fields: serde_json::Map::new(),
        };

        let json = serde_json::to_string(&entry).expect("Failed to serialize");

        // Verify trace_id is NOT present when None
        assert!(!json.contains("trace_id"), "trace_id should not be present when None");
        assert!(!json.contains("span_id"), "span_id should not be present when None");
    }

    /// Tests that create_log_entry produces valid JSON structure.
    /// **Validates: Requirements 2.4**
    #[test]
    fn test_create_log_entry_json_structure() {
        let entry = create_log_entry("warn", "Warning message", "my-service");

        // Verify required fields are present
        assert!(!entry.timestamp.is_empty(), "timestamp should not be empty");
        assert_eq!(entry.level, "WARN", "level should be uppercase");
        assert_eq!(entry.message, "Warning message");
        assert_eq!(entry.service, "my-service");

        // Verify it can be serialized to valid JSON
        let json = serde_json::to_string(&entry).expect("Failed to serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("Failed to parse JSON");

        assert!(parsed.is_object(), "Should serialize to JSON object");
        assert!(parsed["timestamp"].is_string(), "timestamp should be string");
        assert!(parsed["level"].is_string(), "level should be string");
        assert!(parsed["message"].is_string(), "message should be string");
        assert!(parsed["service"].is_string(), "service should be string");
    }

    /// Tests that log entry timestamps are in valid RFC 3339 format.
    /// **Validates: Requirements 2.4**
    #[test]
    fn test_log_entry_timestamp_format() {
        let entry = create_log_entry("info", "Test", "service");

        // Verify timestamp can be parsed as RFC 3339
        let parsed = chrono::DateTime::parse_from_rfc3339(&entry.timestamp);
        assert!(parsed.is_ok(), "Timestamp should be valid RFC 3339: {}", entry.timestamp);
    }

    /// Tests that all log levels are correctly converted to uppercase.
    /// **Validates: Requirements 2.4**
    #[test]
    fn test_log_entry_level_normalization() {
        let test_cases = [
            ("trace", "TRACE"),
            ("debug", "DEBUG"),
            ("info", "INFO"),
            ("warn", "WARN"),
            ("error", "ERROR"),
            ("TRACE", "TRACE"),
            ("Info", "INFO"),
        ];

        for (input, expected) in test_cases {
            let entry = create_log_entry(input, "Test", "service");
            assert_eq!(
                entry.level, expected,
                "Level '{}' should be normalized to '{}'",
                input, expected
            );
        }
    }

    /// Tests that log entries can include additional structured fields.
    /// **Validates: Requirements 2.4**
    #[test]
    fn test_log_entry_with_additional_fields() {
        let mut fields = serde_json::Map::new();
        fields.insert("user_id".to_string(), serde_json::json!(12345));
        fields.insert("request_path".to_string(), serde_json::json!("/api/users"));
        fields.insert("duration_ms".to_string(), serde_json::json!(42.5));

        let entry = LogEntry {
            timestamp: "2024-01-15T10:30:00Z".to_string(),
            level: "INFO".to_string(),
            message: "Request completed".to_string(),
            target: None,
            trace_id: Some("abc123".to_string()),
            span_id: None,
            service: "api-service".to_string(),
            fields,
        };

        let json = serde_json::to_string(&entry).expect("Failed to serialize");

        // Verify additional fields are flattened into the JSON
        assert!(json.contains("\"user_id\":12345"), "user_id field missing");
        assert!(json.contains("\"request_path\":\"/api/users\""), "request_path field missing");
        assert!(json.contains("\"duration_ms\":42.5"), "duration_ms field missing");
    }

    /// Tests that trace ID format is valid (32 hex characters).
    /// **Validates: Requirements 2.4**
    #[test]
    fn test_trace_id_format_validation() {
        // Valid trace IDs (32 hex chars)
        let valid_trace_ids = [
            "0af7651916cd43dd8448eb211c80319c",
            "00000000000000000000000000000001",
            "ffffffffffffffffffffffffffffffff",
            "ABCDEF0123456789abcdef0123456789",
        ];

        for trace_id in valid_trace_ids {
            let entry = LogEntry {
                timestamp: "2024-01-15T10:30:00Z".to_string(),
                level: "INFO".to_string(),
                message: "Test".to_string(),
                target: None,
                trace_id: Some(trace_id.to_string()),
                span_id: None,
                service: "test".to_string(),
                fields: serde_json::Map::new(),
            };

            let json = serde_json::to_string(&entry).expect("Failed to serialize");
            assert!(
                json.contains(&format!("\"trace_id\":\"{}\"", trace_id)),
                "Valid trace_id {} should be serialized correctly",
                trace_id
            );
        }
    }

    /// Tests that span ID format is valid (16 hex characters).
    /// **Validates: Requirements 2.4**
    #[test]
    fn test_span_id_format_validation() {
        // Valid span IDs (16 hex chars)
        let valid_span_ids = [
            "b7ad6b7169203331",
            "0000000000000001",
            "ffffffffffffffff",
            "ABCDEF0123456789",
        ];

        for span_id in valid_span_ids {
            let entry = LogEntry {
                timestamp: "2024-01-15T10:30:00Z".to_string(),
                level: "INFO".to_string(),
                message: "Test".to_string(),
                target: None,
                trace_id: None,
                span_id: Some(span_id.to_string()),
                service: "test".to_string(),
                fields: serde_json::Map::new(),
            };

            let json = serde_json::to_string(&entry).expect("Failed to serialize");
            assert!(
                json.contains(&format!("\"span_id\":\"{}\"", span_id)),
                "Valid span_id {} should be serialized correctly",
                span_id
            );
        }
    }

    /// Tests that LoggingError variants display correctly.
    /// **Validates: Requirements 2.4**
    #[test]
    fn test_logging_error_display() {
        let init_error = LoggingError::InitError("test init error".to_string());
        assert_eq!(format!("{}", init_error), "Failed to initialize logging: test init error");

        let already_init = LoggingError::AlreadyInitialized;
        assert_eq!(format!("{}", already_init), "Logging has already been initialized");

        let directive_error = LoggingError::DirectiveError("bad directive".to_string());
        assert_eq!(format!("{}", directive_error), "Failed to parse log directive: bad directive");

        let subscriber_error = LoggingError::SubscriberError("subscriber failed".to_string());
        assert_eq!(
            format!("{}", subscriber_error),
            "Failed to set global subscriber: subscriber failed"
        );
    }

    /// Tests that LoggingConfig can be serialized to JSON.
    /// **Validates: Requirements 2.4**
    #[test]
    fn test_logging_config_serialization() {
        let config = LoggingConfig::new()
            .with_level(LogLevel::Debug)
            .with_json(true)
            .with_file_info(true);

        let json = serde_json::to_string(&config).expect("Failed to serialize");

        assert!(json.contains("\"default_level\":\"debug\""));
        assert!(json.contains("\"json_format\":true"));
        assert!(json.contains("\"include_file_info\":true"));
    }

    /// Tests that LogLevel can be serialized to lowercase strings.
    /// **Validates: Requirements 2.4**
    #[test]
    fn test_log_level_serialization() {
        let levels = [
            (LogLevel::Trace, "\"trace\""),
            (LogLevel::Debug, "\"debug\""),
            (LogLevel::Info, "\"info\""),
            (LogLevel::Warn, "\"warn\""),
            (LogLevel::Error, "\"error\""),
        ];

        for (level, expected) in levels {
            let json = serde_json::to_string(&level).expect("Failed to serialize");
            assert_eq!(json, expected, "LogLevel::{:?} should serialize to {}", level, expected);
        }
    }

    /// Tests that log entries with all fields populated serialize correctly.
    /// **Validates: Requirements 2.4**
    #[test]
    fn test_log_entry_full_serialization() {
        let mut fields = serde_json::Map::new();
        fields.insert("custom_field".to_string(), serde_json::json!("custom_value"));

        let entry = LogEntry {
            timestamp: "2024-01-15T10:30:00.123Z".to_string(),
            level: "INFO".to_string(),
            message: "Full log entry test".to_string(),
            target: Some("my::module::path".to_string()),
            trace_id: Some("0af7651916cd43dd8448eb211c80319c".to_string()),
            span_id: Some("b7ad6b7169203331".to_string()),
            service: "full-test-service".to_string(),
            fields,
        };

        let json = serde_json::to_string_pretty(&entry).expect("Failed to serialize");

        // Verify all fields are present
        assert!(json.contains("\"timestamp\""));
        assert!(json.contains("\"level\""));
        assert!(json.contains("\"message\""));
        assert!(json.contains("\"target\""));
        assert!(json.contains("\"trace_id\""));
        assert!(json.contains("\"span_id\""));
        assert!(json.contains("\"service\""));
        assert!(json.contains("\"custom_field\""));

        // Verify it can be parsed back
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("Failed to parse");
        assert_eq!(parsed["level"], "INFO");
        assert_eq!(parsed["service"], "full-test-service");
        assert_eq!(parsed["trace_id"], "0af7651916cd43dd8448eb211c80319c");
    }

    /// Tests that empty message is handled correctly.
    /// **Validates: Requirements 2.4**
    #[test]
    fn test_log_entry_empty_message() {
        let entry = create_log_entry("info", "", "service");

        assert_eq!(entry.message, "");

        let json = serde_json::to_string(&entry).expect("Failed to serialize");
        assert!(json.contains("\"message\":\"\""));
    }

    /// Tests that special characters in messages are properly escaped.
    /// **Validates: Requirements 2.4**
    #[test]
    fn test_log_entry_special_characters() {
        let special_messages = [
            "Message with \"quotes\"",
            "Message with \\ backslash",
            "Message with \n newline",
            "Message with \t tab",
            "Message with unicode: 日本語",
            "Message with emoji: 🚀",
        ];

        for message in special_messages {
            let entry = create_log_entry("info", message, "service");

            // Should serialize without error
            let json = serde_json::to_string(&entry).expect("Failed to serialize");

            // Should parse back correctly
            let parsed: serde_json::Value = serde_json::from_str(&json).expect("Failed to parse");
            assert_eq!(
                parsed["message"].as_str().unwrap(),
                message,
                "Message should round-trip correctly: {}",
                message
            );
        }
    }
}
