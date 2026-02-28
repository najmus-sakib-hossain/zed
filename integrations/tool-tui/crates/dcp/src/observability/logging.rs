//! Structured JSON logging for DCP server.
//!
//! Provides structured logging compatible with log aggregation systems.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogLevel {
    /// Trace level - most verbose
    Trace,
    /// Debug level
    Debug,
    /// Info level
    Info,
    /// Warning level
    Warn,
    /// Error level
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

impl LogLevel {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "TRACE" => Some(LogLevel::Trace),
            "DEBUG" => Some(LogLevel::Debug),
            "INFO" => Some(LogLevel::Info),
            "WARN" | "WARNING" => Some(LogLevel::Warn),
            "ERROR" => Some(LogLevel::Error),
            _ => None,
        }
    }
}

/// Log configuration
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Minimum log level
    pub level: LogLevel,
    /// Output format (json or text)
    pub format: LogFormat,
    /// Include timestamps
    pub include_timestamp: bool,
    /// Include source location
    pub include_location: bool,
    /// Service name
    pub service_name: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Json,
            include_timestamp: true,
            include_location: false,
            service_name: "dcp-server".to_string(),
        }
    }
}

/// Log output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// JSON format for log aggregation
    Json,
    /// Human-readable text format
    Text,
}

/// Log field value
#[derive(Debug, Clone)]
pub enum LogValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
}

impl From<&str> for LogValue {
    fn from(s: &str) -> Self {
        LogValue::String(s.to_string())
    }
}

impl From<String> for LogValue {
    fn from(s: String) -> Self {
        LogValue::String(s)
    }
}

impl From<i64> for LogValue {
    fn from(v: i64) -> Self {
        LogValue::Int(v)
    }
}

impl From<i32> for LogValue {
    fn from(v: i32) -> Self {
        LogValue::Int(v as i64)
    }
}

impl From<u64> for LogValue {
    fn from(v: u64) -> Self {
        LogValue::Int(v as i64)
    }
}

impl From<f64> for LogValue {
    fn from(v: f64) -> Self {
        LogValue::Float(v)
    }
}

impl From<bool> for LogValue {
    fn from(v: bool) -> Self {
        LogValue::Bool(v)
    }
}

impl fmt::Display for LogValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogValue::String(s) => write!(f, "{}", s),
            LogValue::Int(i) => write!(f, "{}", i),
            LogValue::Float(fl) => write!(f, "{}", fl),
            LogValue::Bool(b) => write!(f, "{}", b),
            LogValue::Null => write!(f, "null"),
        }
    }
}

/// A structured log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Log level
    pub level: LogLevel,
    /// Log message
    pub message: String,
    /// Timestamp (Unix milliseconds)
    pub timestamp: u64,
    /// Request ID for correlation
    pub request_id: Option<String>,
    /// Trace ID for distributed tracing
    pub trace_id: Option<String>,
    /// Additional fields
    pub fields: HashMap<String, LogValue>,
    /// Source file
    pub file: Option<String>,
    /// Source line
    pub line: Option<u32>,
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        let timestamp =
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

        Self {
            level,
            message: message.into(),
            timestamp,
            request_id: None,
            trace_id: None,
            fields: HashMap::new(),
            file: None,
            line: None,
        }
    }

    /// Set request ID
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Set trace ID
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    /// Add a field
    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<LogValue>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    /// Set source location
    pub fn with_location(mut self, file: impl Into<String>, line: u32) -> Self {
        self.file = Some(file.into());
        self.line = Some(line);
        self
    }

    /// Format as JSON
    pub fn to_json(&self, service_name: &str) -> String {
        let mut json = String::from("{");

        // Required fields
        json.push_str(&format!("\"timestamp\":{},", self.timestamp));
        json.push_str(&format!("\"level\":\"{}\",", self.level));
        json.push_str(&format!("\"message\":{},", escape_json_string(&self.message)));
        json.push_str(&format!("\"service\":\"{}\",", service_name));

        // Optional fields
        if let Some(ref request_id) = self.request_id {
            json.push_str(&format!("\"request_id\":\"{}\",", request_id));
        }
        if let Some(ref trace_id) = self.trace_id {
            json.push_str(&format!("\"trace_id\":\"{}\",", trace_id));
        }
        if let Some(ref file) = self.file {
            json.push_str(&format!("\"file\":\"{}\",", file));
        }
        if let Some(line) = self.line {
            json.push_str(&format!("\"line\":{},", line));
        }

        // Additional fields
        for (key, value) in &self.fields {
            let value_str = match value {
                LogValue::String(s) => escape_json_string(s),
                LogValue::Int(i) => i.to_string(),
                LogValue::Float(f) => f.to_string(),
                LogValue::Bool(b) => b.to_string(),
                LogValue::Null => "null".to_string(),
            };
            json.push_str(&format!("\"{}\":{},", key, value_str));
        }

        // Remove trailing comma and close
        if json.ends_with(',') {
            json.pop();
        }
        json.push('}');

        json
    }

    /// Format as text
    pub fn to_text(&self) -> String {
        let mut text =
            format!("{} [{}] {}", format_timestamp(self.timestamp), self.level, self.message);

        if let Some(ref request_id) = self.request_id {
            text.push_str(&format!(" request_id={}", request_id));
        }

        for (key, value) in &self.fields {
            text.push_str(&format!(" {}={}", key, value));
        }

        text
    }
}

/// Escape a string for JSON
fn escape_json_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 2);
    result.push('"');
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() => {
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => result.push(c),
        }
    }
    result.push('"');
    result
}

/// Format timestamp as ISO 8601
fn format_timestamp(millis: u64) -> String {
    let secs = millis / 1000;
    let ms = millis % 1000;

    // Simple UTC timestamp formatting
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Calculate year, month, day from days since epoch (1970-01-01)
    let (year, month, day) = days_to_ymd(days_since_epoch as i64);

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        year, month, day, hours, minutes, seconds, ms
    )
}

/// Convert days since epoch to year, month, day
fn days_to_ymd(days: i64) -> (i32, u32, u32) {
    // Simplified calculation - good enough for logging
    let mut remaining = days;
    let mut year = 1970i32;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }

    let days_in_months: [i64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1u32;
    for days_in_month in days_in_months.iter() {
        if remaining < *days_in_month {
            break;
        }
        remaining -= days_in_month;
        month += 1;
    }

    (year, month, (remaining + 1) as u32)
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Structured logger
pub struct StructuredLogger {
    config: LogConfig,
    /// Log sink for testing
    sink: Arc<RwLock<Vec<LogEntry>>>,
}

impl StructuredLogger {
    /// Create a new structured logger
    pub fn new(config: LogConfig) -> Self {
        Self {
            config,
            sink: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(LogConfig::default())
    }

    /// Log at trace level
    pub fn trace(&self, message: impl Into<String>) -> LogEntryBuilder<'_> {
        LogEntryBuilder::new(self, LogLevel::Trace, message.into())
    }

    /// Log at debug level
    pub fn debug(&self, message: impl Into<String>) -> LogEntryBuilder<'_> {
        LogEntryBuilder::new(self, LogLevel::Debug, message.into())
    }

    /// Log at info level
    pub fn info(&self, message: impl Into<String>) -> LogEntryBuilder<'_> {
        LogEntryBuilder::new(self, LogLevel::Info, message.into())
    }

    /// Log at warn level
    pub fn warn(&self, message: impl Into<String>) -> LogEntryBuilder<'_> {
        LogEntryBuilder::new(self, LogLevel::Warn, message.into())
    }

    /// Log at error level
    pub fn error(&self, message: impl Into<String>) -> LogEntryBuilder<'_> {
        LogEntryBuilder::new(self, LogLevel::Error, message.into())
    }

    /// Check if level is enabled
    pub fn is_enabled(&self, level: LogLevel) -> bool {
        level >= self.config.level
    }

    /// Get configuration
    pub fn config(&self) -> &LogConfig {
        &self.config
    }

    /// Set log level
    pub fn set_level(&mut self, level: LogLevel) {
        self.config.level = level;
    }

    /// Format and emit a log entry
    pub fn emit(&self, entry: LogEntry) {
        if entry.level < self.config.level {
            return;
        }

        // Store in sink for testing
        self.sink.write().unwrap().push(entry.clone());

        // Format output
        let output = match self.config.format {
            LogFormat::Json => entry.to_json(&self.config.service_name),
            LogFormat::Text => entry.to_text(),
        };

        // In a real implementation, this would write to stdout/stderr/file
        // For now, we just store it
        let _ = output;
    }

    /// Get logged entries (for testing)
    pub fn entries(&self) -> Vec<LogEntry> {
        self.sink.read().unwrap().clone()
    }

    /// Clear logged entries
    pub fn clear(&self) {
        self.sink.write().unwrap().clear();
    }
}

impl Default for StructuredLogger {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Builder for log entries
pub struct LogEntryBuilder<'a> {
    logger: &'a StructuredLogger,
    entry: LogEntry,
}

impl<'a> LogEntryBuilder<'a> {
    fn new(logger: &'a StructuredLogger, level: LogLevel, message: String) -> Self {
        Self {
            logger,
            entry: LogEntry::new(level, message),
        }
    }

    /// Set request ID
    pub fn request_id(mut self, request_id: impl Into<String>) -> Self {
        self.entry.request_id = Some(request_id.into());
        self
    }

    /// Set trace ID
    pub fn trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.entry.trace_id = Some(trace_id.into());
        self
    }

    /// Add a field
    pub fn field(mut self, key: impl Into<String>, value: impl Into<LogValue>) -> Self {
        self.entry.fields.insert(key.into(), value.into());
        self
    }

    /// Emit the log entry
    pub fn emit(self) {
        self.logger.emit(self.entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Trace < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
    }

    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::from_str("INFO"), Some(LogLevel::Info));
        assert_eq!(LogLevel::from_str("info"), Some(LogLevel::Info));
        assert_eq!(LogLevel::from_str("WARNING"), Some(LogLevel::Warn));
        assert_eq!(LogLevel::from_str("invalid"), None);
    }

    #[test]
    fn test_log_entry_json() {
        let entry = LogEntry::new(LogLevel::Info, "Test message")
            .with_request_id("req-123")
            .with_field("user_id", "user-456");

        let json = entry.to_json("test-service");

        assert!(json.contains("\"level\":\"INFO\""));
        assert!(json.contains("\"message\":\"Test message\""));
        assert!(json.contains("\"request_id\":\"req-123\""));
        assert!(json.contains("\"user_id\":\"user-456\""));
        assert!(json.contains("\"service\":\"test-service\""));
        assert!(json.contains("\"timestamp\":"));
    }

    #[test]
    fn test_log_entry_text() {
        let entry =
            LogEntry::new(LogLevel::Error, "Something went wrong").with_request_id("req-789");

        let text = entry.to_text();

        assert!(text.contains("[ERROR]"));
        assert!(text.contains("Something went wrong"));
        assert!(text.contains("request_id=req-789"));
    }

    #[test]
    fn test_json_escaping() {
        let entry = LogEntry::new(LogLevel::Info, "Message with \"quotes\" and \nnewline");
        let json = entry.to_json("test");

        assert!(json.contains("\\\"quotes\\\""));
        assert!(json.contains("\\n"));
    }

    #[test]
    fn test_structured_logger() {
        let logger = StructuredLogger::with_defaults();

        logger
            .info("Test info message")
            .request_id("req-001")
            .field("method", "tools/call")
            .emit();

        let entries = logger.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].level, LogLevel::Info);
        assert_eq!(entries[0].message, "Test info message");
        assert_eq!(entries[0].request_id, Some("req-001".to_string()));
    }

    #[test]
    fn test_log_level_filtering() {
        let mut logger = StructuredLogger::new(LogConfig {
            level: LogLevel::Warn,
            ..Default::default()
        });

        logger.debug("Debug message").emit();
        logger.info("Info message").emit();
        logger.warn("Warn message").emit();
        logger.error("Error message").emit();

        let entries = logger.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].level, LogLevel::Warn);
        assert_eq!(entries[1].level, LogLevel::Error);
    }

    #[test]
    fn test_timestamp_formatting() {
        // Test a known timestamp: 2024-01-15 12:30:45.123 UTC
        // This is approximately 1705321845123 milliseconds since epoch
        let formatted = format_timestamp(1705321845123);
        assert!(formatted.contains("2024-01-15"));
        assert!(formatted.contains("12:30:45.123Z"));
    }

    #[test]
    fn test_log_value_types() {
        let entry = LogEntry::new(LogLevel::Info, "Test")
            .with_field("string", "value")
            .with_field("int", 42i64)
            .with_field("float", 3.14f64)
            .with_field("bool", true);

        assert_eq!(entry.fields.len(), 4);
    }
}
