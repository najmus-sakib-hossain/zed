//! Production Error Handling and Retry Logic
//!
//! Provides robust error handling, retry mechanisms, and detailed error reporting
//! for DX tools orchestration.
//!
//! Features:
//! - Categorized error types with context
//! - Exponential backoff retry with timing validation
//! - Structured error logging
//! - Platform-specific error handlers

use anyhow::Result;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Retry policy configuration
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,

    /// Initial delay between retries
    pub initial_delay: Duration,

    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,

    /// Maximum delay between retries
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            backoff_multiplier: 2.0,
            max_delay: Duration::from_secs(5),
        }
    }
}

impl RetryPolicy {
    /// Create a no-retry policy
    pub fn no_retry() -> Self {
        Self {
            max_attempts: 1,
            ..Default::default()
        }
    }

    /// Create an aggressive retry policy
    pub fn aggressive() -> Self {
        Self {
            max_attempts: 5,
            initial_delay: Duration::from_millis(50),
            backoff_multiplier: 1.5,
            max_delay: Duration::from_secs(3),
        }
    }

    /// Calculate delay for a given attempt number (0-indexed)
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::ZERO;
        }

        let delay_secs =
            self.initial_delay.as_secs_f64() * self.backoff_multiplier.powi(attempt as i32 - 1);

        Duration::from_secs_f64(delay_secs.min(self.max_delay.as_secs_f64()))
    }
}

/// Execute with retry logic and exponential backoff
///
/// Returns the result along with timing information for each attempt.
pub async fn with_retry<F, T, E>(policy: &RetryPolicy, mut operation: F) -> Result<T>
where
    F: FnMut() -> Result<T, E>,
    E: std::fmt::Display,
{
    let mut attempts = 0;
    let mut last_delay = Duration::ZERO;

    loop {
        attempts += 1;
        let _attempt_start = Instant::now();

        match operation() {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempts >= policy.max_attempts {
                    return Err(anyhow::anyhow!(
                        "Operation failed after {} attempts: {}",
                        attempts,
                        e
                    ));
                }

                // Calculate next delay with exponential backoff
                let delay = policy.delay_for_attempt(attempts);

                // Validate exponential growth (delay should be >= last delay)
                debug_assert!(
                    delay >= last_delay || delay == policy.max_delay,
                    "Exponential backoff violated: {:?} < {:?}",
                    delay,
                    last_delay
                );

                tracing::warn!(
                    attempt = attempts,
                    max_attempts = policy.max_attempts,
                    delay_ms = delay.as_millis(),
                    error = %e,
                    "Retry attempt failed, backing off"
                );

                sleep(delay).await;
                last_delay = delay;
            }
        }
    }
}

/// Execute with retry logic, returning timing information
pub async fn with_retry_timed<F, T, E>(
    policy: &RetryPolicy,
    mut operation: F,
) -> Result<(T, Vec<RetryAttempt>)>
where
    F: FnMut() -> Result<T, E>,
    E: std::fmt::Display + Clone,
{
    let mut attempts = Vec::new();
    let mut attempt_num = 0;

    loop {
        attempt_num += 1;
        let start = Instant::now();

        match operation() {
            Ok(result) => {
                attempts.push(RetryAttempt {
                    attempt: attempt_num,
                    duration: start.elapsed(),
                    delay_before: if attempt_num == 1 {
                        Duration::ZERO
                    } else {
                        policy.delay_for_attempt(attempt_num - 1)
                    },
                    success: true,
                    error: None,
                });
                return Ok((result, attempts));
            }
            Err(e) => {
                let error_str = e.to_string();
                attempts.push(RetryAttempt {
                    attempt: attempt_num,
                    duration: start.elapsed(),
                    delay_before: if attempt_num == 1 {
                        Duration::ZERO
                    } else {
                        policy.delay_for_attempt(attempt_num - 1)
                    },
                    success: false,
                    error: Some(error_str.clone()),
                });

                if attempt_num >= policy.max_attempts {
                    return Err(anyhow::anyhow!(
                        "Operation failed after {} attempts: {}",
                        attempt_num,
                        e
                    ));
                }

                let delay = policy.delay_for_attempt(attempt_num);
                sleep(delay).await;
            }
        }
    }
}

/// Information about a single retry attempt
#[derive(Debug, Clone)]
pub struct RetryAttempt {
    /// Attempt number (1-indexed)
    pub attempt: u32,
    /// Duration of the operation
    pub duration: Duration,
    /// Delay before this attempt
    pub delay_before: Duration,
    /// Whether this attempt succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Categorized error types for better handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Network-related errors (retryable)
    Network,

    /// File system errors (may be retryable)
    FileSystem,

    /// Configuration errors (not retryable)
    Configuration,

    /// Validation errors (not retryable)
    Validation,

    /// Dependency errors (not retryable)
    Dependency,

    /// Timeout errors (may be retryable)
    Timeout,

    /// Unknown errors
    Unknown,
}

impl ErrorCategory {
    /// Check if this error category is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ErrorCategory::Network | ErrorCategory::FileSystem | ErrorCategory::Timeout
        )
    }

    /// Get the category name as a string
    pub fn name(&self) -> &'static str {
        match self {
            ErrorCategory::Network => "network",
            ErrorCategory::FileSystem => "filesystem",
            ErrorCategory::Configuration => "configuration",
            ErrorCategory::Validation => "validation",
            ErrorCategory::Dependency => "dependency",
            ErrorCategory::Timeout => "timeout",
            ErrorCategory::Unknown => "unknown",
        }
    }
}

/// Categorize an error
pub fn categorize_error(error: &anyhow::Error) -> ErrorCategory {
    let error_str = error.to_string().to_lowercase();

    if error_str.contains("network")
        || error_str.contains("connection")
        || error_str.contains("dns")
        || error_str.contains("socket")
    {
        ErrorCategory::Network
    } else if error_str.contains("timeout") || error_str.contains("timed out") {
        ErrorCategory::Timeout
    } else if error_str.contains("config") || error_str.contains("invalid") {
        // Check config before file system since "config file" should be Configuration
        ErrorCategory::Configuration
    } else if error_str.contains("validation") || error_str.contains("required") {
        ErrorCategory::Validation
    } else if error_str.contains("dependency") || error_str.contains("version") {
        ErrorCategory::Dependency
    } else if error_str.contains("file")
        || error_str.contains("directory")
        || error_str.contains("permission")
        || error_str.contains("io error")
        || error_str.contains("not found")
    {
        ErrorCategory::FileSystem
    } else {
        ErrorCategory::Unknown
    }
}

/// Error context with full details
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// File path involved (if any)
    pub file: Option<PathBuf>,
    /// Operation being performed
    pub operation: String,
    /// Timestamp when error occurred
    pub timestamp: DateTime<Utc>,
    /// Number of retry attempts
    pub retry_count: u32,
    /// Platform name
    pub platform: &'static str,
    /// I/O backend name
    pub backend: &'static str,
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self {
            file: None,
            operation: String::new(),
            timestamp: Utc::now(),
            retry_count: 0,
            platform: std::env::consts::OS,
            backend: "unknown",
        }
    }
}

impl ErrorContext {
    /// Create a new error context
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            ..Default::default()
        }
    }

    /// Set the file path
    pub fn with_file(mut self, file: impl Into<PathBuf>) -> Self {
        self.file = Some(file.into());
        self
    }

    /// Set the backend name
    pub fn with_backend(mut self, backend: &'static str) -> Self {
        self.backend = backend;
        self
    }

    /// Set the retry count
    pub fn with_retry_count(mut self, count: u32) -> Self {
        self.retry_count = count;
        self
    }
}

/// Enhanced error with full context
#[derive(Debug)]
pub struct ForgeError {
    /// Error category
    pub category: ErrorCategory,
    /// Error message
    pub message: String,
    /// Original error
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
    /// Error context
    pub context: ErrorContext,
}

impl ForgeError {
    /// Create a new ForgeError
    pub fn new(category: ErrorCategory, message: impl Into<String>, context: ErrorContext) -> Self {
        Self {
            category,
            message: message.into(),
            source: None,
            context,
        }
    }

    /// Create from an anyhow error
    pub fn from_anyhow(error: anyhow::Error, context: ErrorContext) -> Self {
        let category = categorize_error(&error);
        Self {
            category,
            message: error.to_string(),
            source: None,
            context,
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        self.category.is_retryable()
    }

    /// Get suggestions for resolving this error
    pub fn suggestions(&self) -> Vec<String> {
        let mut suggestions = Vec::new();

        match self.category {
            ErrorCategory::Network => {
                suggestions.push("Check your internet connection".to_string());
                suggestions.push("Verify firewall settings".to_string());
                suggestions.push("Try again in a few moments".to_string());
            }
            ErrorCategory::FileSystem => {
                suggestions.push("Check file permissions".to_string());
                suggestions.push("Verify the path exists".to_string());
                suggestions.push("Ensure sufficient disk space".to_string());
                if let Some(ref file) = self.context.file {
                    suggestions.push(format!("Check path: {}", file.display()));
                }
            }
            ErrorCategory::Configuration => {
                suggestions.push("Review your configuration file".to_string());
                suggestions.push("Check environment variables".to_string());
                suggestions.push("Refer to documentation for valid options".to_string());
            }
            ErrorCategory::Validation => {
                suggestions.push("Review input data".to_string());
                suggestions.push("Check for required fields".to_string());
            }
            ErrorCategory::Dependency => {
                suggestions.push("Check tool dependencies".to_string());
                suggestions.push("Verify version compatibility".to_string());
                suggestions.push("Run 'forge update' to sync dependencies".to_string());
            }
            ErrorCategory::Timeout => {
                suggestions.push("The operation may need more time".to_string());
                suggestions.push("Try increasing timeout settings".to_string());
                suggestions.push("Check system resources".to_string());
            }
            ErrorCategory::Unknown => {
                suggestions.push("Check logs for more details".to_string());
                suggestions.push("Report this issue if it persists".to_string());
            }
        }

        suggestions
    }

    /// Log this error with structured format
    pub fn log(&self) {
        tracing::error!(
            category = self.category.name(),
            message = %self.message,
            file = ?self.context.file,
            operation = %self.context.operation,
            timestamp = %self.context.timestamp,
            retry_count = self.context.retry_count,
            platform = self.context.platform,
            backend = self.context.backend,
            retryable = self.is_retryable(),
            "Forge error occurred"
        );
    }
}

impl std::fmt::Display for ForgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.category.name(), self.message)
    }
}

impl std::error::Error for ForgeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
    }
}

/// Enhanced error with context and suggestions
#[derive(Debug)]
pub struct EnhancedError {
    pub error: anyhow::Error,
    pub category: ErrorCategory,
    pub context: Vec<String>,
    pub suggestions: Vec<String>,
}

impl EnhancedError {
    /// Create an enhanced error
    pub fn new(error: anyhow::Error) -> Self {
        let category = categorize_error(&error);
        let (context, suggestions) = generate_context_and_suggestions(&category, &error);

        Self {
            error,
            category,
            context,
            suggestions,
        }
    }

    /// Display the error with all context
    pub fn display(&self) -> String {
        let mut output = format!("❌ Error: {}\n", self.error);

        if !self.context.is_empty() {
            output.push_str("\n📋 Context:\n");
            for ctx in &self.context {
                output.push_str(&format!("   • {}\n", ctx));
            }
        }

        if !self.suggestions.is_empty() {
            output.push_str("\n💡 Suggestions:\n");
            for suggestion in &self.suggestions {
                output.push_str(&format!("   • {}\n", suggestion));
            }
        }

        output
    }
}

/// Generate helpful context and suggestions based on error category
fn generate_context_and_suggestions(
    category: &ErrorCategory,
    error: &anyhow::Error,
) -> (Vec<String>, Vec<String>) {
    let mut context = Vec::new();
    let mut suggestions = Vec::new();

    match category {
        ErrorCategory::Network => {
            context.push("Network operation failed".to_string());
            suggestions.push("Check your internet connection".to_string());
            suggestions.push("Verify firewall settings".to_string());
            suggestions.push("Try again in a few moments".to_string());
        }
        ErrorCategory::FileSystem => {
            context.push("File system operation failed".to_string());
            suggestions.push("Check file permissions".to_string());
            suggestions.push("Verify the path exists".to_string());
            suggestions.push("Ensure sufficient disk space".to_string());
        }
        ErrorCategory::Configuration => {
            context.push("Configuration error detected".to_string());
            suggestions.push("Review your configuration file".to_string());
            suggestions.push("Check environment variables".to_string());
            suggestions.push("Refer to documentation for valid options".to_string());
        }
        ErrorCategory::Dependency => {
            context.push("Dependency resolution failed".to_string());
            suggestions.push("Check tool dependencies".to_string());
            suggestions.push("Verify version compatibility".to_string());
            suggestions.push("Run 'forge update' to sync dependencies".to_string());
        }
        ErrorCategory::Timeout => {
            context.push("Operation timed out".to_string());
            suggestions.push("The operation may need more time".to_string());
            suggestions.push("Try increasing timeout settings".to_string());
            suggestions.push("Check system resources".to_string());
        }
        ErrorCategory::Validation => {
            context.push("Validation error".to_string());
            suggestions.push("Review input data".to_string());
            suggestions.push("Check for required fields".to_string());
        }
        ErrorCategory::Unknown => {
            context.push(format!("Unexpected error: {}", error));
            suggestions.push("Check logs for more details".to_string());
            suggestions.push("Report this issue if it persists".to_string());
        }
    }

    (context, suggestions)
}

/// Result type with enhanced error
pub type EnhancedResult<T> = Result<T, EnhancedError>;

/// Convert regular Result to EnhancedResult
pub trait ToEnhanced<T> {
    fn enhance(self) -> EnhancedResult<T>;
}

impl<T, E: Into<anyhow::Error>> ToEnhanced<T> for Result<T, E> {
    fn enhance(self) -> EnhancedResult<T> {
        self.map_err(|e| EnhancedError::new(e.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_categorization() {
        let net_err = anyhow::anyhow!("Network connection failed");
        assert_eq!(categorize_error(&net_err), ErrorCategory::Network);

        let fs_err = anyhow::anyhow!("File not found");
        assert_eq!(categorize_error(&fs_err), ErrorCategory::FileSystem);

        let config_err = anyhow::anyhow!("Invalid config value");
        assert_eq!(categorize_error(&config_err), ErrorCategory::Configuration);
    }

    #[test]
    fn test_retryable() {
        assert!(ErrorCategory::Network.is_retryable());
        assert!(!ErrorCategory::Configuration.is_retryable());
    }

    #[test]
    fn test_retry_policy() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);

        let no_retry = RetryPolicy::no_retry();
        assert_eq!(no_retry.max_attempts, 1);
    }

    #[test]
    fn test_delay_for_attempt() {
        let policy = RetryPolicy {
            max_attempts: 5,
            initial_delay: Duration::from_millis(100),
            backoff_multiplier: 2.0,
            max_delay: Duration::from_secs(10),
        };

        // Attempt 0 should have no delay
        assert_eq!(policy.delay_for_attempt(0), Duration::ZERO);

        // Attempt 1 should have initial delay
        assert_eq!(policy.delay_for_attempt(1), Duration::from_millis(100));

        // Attempt 2 should be 2x initial
        assert_eq!(policy.delay_for_attempt(2), Duration::from_millis(200));

        // Attempt 3 should be 4x initial
        assert_eq!(policy.delay_for_attempt(3), Duration::from_millis(400));
    }

    #[test]
    fn test_forge_error_suggestions() {
        let context = ErrorContext::new("test_operation")
            .with_file("/test/path.txt")
            .with_backend("fallback");

        let error = ForgeError::new(ErrorCategory::FileSystem, "File not found", context);

        let suggestions = error.suggestions();
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.contains("permission") || s.contains("path")));
    }

    #[test]
    fn test_error_context_defaults() {
        let context = ErrorContext::default();
        assert!(context.file.is_none());
        assert!(context.operation.is_empty());
        assert_eq!(context.retry_count, 0);
        assert_eq!(context.platform, std::env::consts::OS);
    }
}

/// Property-based tests for error handling
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate arbitrary error messages for different categories
    fn error_message_strategy() -> impl Strategy<Value = (String, ErrorCategory)> {
        prop_oneof![
            Just(("network connection failed".to_string(), ErrorCategory::Network)),
            Just(("socket error occurred".to_string(), ErrorCategory::Network)),
            Just(("dns resolution failed".to_string(), ErrorCategory::Network)),
            Just(("connection refused".to_string(), ErrorCategory::Network)),
            Just(("file not found".to_string(), ErrorCategory::FileSystem)),
            Just(("permission denied".to_string(), ErrorCategory::FileSystem)),
            Just(("directory does not exist".to_string(), ErrorCategory::FileSystem)),
            Just(("io error reading file".to_string(), ErrorCategory::FileSystem)),
            Just(("invalid config value".to_string(), ErrorCategory::Configuration)),
            Just(("config file missing".to_string(), ErrorCategory::Configuration)),
            Just(("validation failed for field".to_string(), ErrorCategory::Validation)),
            Just(("required field missing".to_string(), ErrorCategory::Validation)),
            Just(("dependency not found".to_string(), ErrorCategory::Dependency)),
            Just(("version mismatch".to_string(), ErrorCategory::Dependency)),
            Just(("operation timed out".to_string(), ErrorCategory::Timeout)),
            Just(("request timeout exceeded".to_string(), ErrorCategory::Timeout)),
            Just(("unexpected internal error".to_string(), ErrorCategory::Unknown)),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 9: Error Categorization Completeness
        /// For any error produced by the system, the error categorization function
        /// SHALL assign exactly one category from {Network, FileSystem, Configuration,
        /// Validation, Dependency, Timeout, Unknown}.
        #[test]
        fn prop_error_categorization_completeness((msg, expected_category) in error_message_strategy()) {
            let error = anyhow::anyhow!("{}", msg);
            let category = categorize_error(&error);

            // Category must be one of the valid categories
            let valid_categories = [
                ErrorCategory::Network,
                ErrorCategory::FileSystem,
                ErrorCategory::Configuration,
                ErrorCategory::Validation,
                ErrorCategory::Dependency,
                ErrorCategory::Timeout,
                ErrorCategory::Unknown,
            ];

            prop_assert!(valid_categories.contains(&category),
                "Category {:?} is not a valid category", category);

            // For known error messages, verify correct categorization
            prop_assert_eq!(category, expected_category,
                "Error '{}' should be {:?} but was {:?}", msg, expected_category, category);
        }

        /// Property 9 (continued): Any arbitrary error gets categorized
        #[test]
        fn prop_any_error_gets_categorized(msg in "[a-zA-Z0-9 ]{1,100}") {
            let error = anyhow::anyhow!("{}", msg);
            let category = categorize_error(&error);

            // Must be assigned exactly one category
            let valid_categories = [
                ErrorCategory::Network,
                ErrorCategory::FileSystem,
                ErrorCategory::Configuration,
                ErrorCategory::Validation,
                ErrorCategory::Dependency,
                ErrorCategory::Timeout,
                ErrorCategory::Unknown,
            ];

            prop_assert!(valid_categories.contains(&category),
                "Category {:?} is not valid", category);
        }

        /// Property 10: Exponential Backoff Retry
        /// For any retryable error with retry policy configured for N attempts,
        /// the delay between attempt i and attempt i+1 SHALL be greater than or
        /// equal to the delay between attempt i-1 and attempt i (exponential growth),
        /// up to the maximum delay.
        #[test]
        fn prop_exponential_backoff(
            max_attempts in 2..10u32,
            initial_delay_ms in 10..500u64,
            multiplier in 1.1..3.0f64,
            max_delay_ms in 1000..10000u64,
        ) {
            let policy = RetryPolicy {
                max_attempts,
                initial_delay: Duration::from_millis(initial_delay_ms),
                backoff_multiplier: multiplier,
                max_delay: Duration::from_millis(max_delay_ms),
            };

            let mut prev_delay = Duration::ZERO;

            for attempt in 0..max_attempts {
                let delay = policy.delay_for_attempt(attempt);

                if attempt == 0 {
                    // First attempt has no delay
                    prop_assert_eq!(delay, Duration::ZERO,
                        "Attempt 0 should have zero delay");
                } else if attempt == 1 {
                    // Second attempt has initial delay
                    prop_assert_eq!(delay, policy.initial_delay,
                        "Attempt 1 should have initial delay");
                } else {
                    // Subsequent attempts should have exponentially growing delay
                    // (or be capped at max_delay)
                    prop_assert!(
                        delay >= prev_delay || delay == policy.max_delay,
                        "Delay should grow exponentially: attempt {} delay {:?} < prev {:?}",
                        attempt, delay, prev_delay
                    );

                    // Delay should never exceed max
                    prop_assert!(delay <= policy.max_delay,
                        "Delay {:?} exceeds max {:?}", delay, policy.max_delay);
                }

                prev_delay = delay;
            }
        }

        /// Property 10 (continued): Verify actual timing in retry execution
        #[test]
        fn prop_exponential_backoff_timing(
            num_failures in 1..4usize,
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let policy = RetryPolicy {
                    max_attempts: (num_failures + 1) as u32,
                    initial_delay: Duration::from_millis(50),
                    backoff_multiplier: 2.0,
                    max_delay: Duration::from_secs(1),
                };

                let mut attempt_count = 0;
                let target_failures = num_failures;

                let operation = || {
                    attempt_count += 1;
                    if attempt_count <= target_failures {
                        Err::<(), _>("simulated failure")
                    } else {
                        Ok(())
                    }
                };

                let start = Instant::now();
                let result = with_retry(&policy, operation).await;
                let elapsed = start.elapsed();

                prop_assert!(result.is_ok(), "Operation should eventually succeed");

                // Calculate expected minimum delay
                let mut expected_min_delay = Duration::ZERO;
                for i in 1..=target_failures {
                    expected_min_delay += policy.delay_for_attempt(i as u32);
                }

                // Allow some tolerance for timing
                let tolerance = Duration::from_millis(50);
                prop_assert!(
                    elapsed >= expected_min_delay.saturating_sub(tolerance),
                    "Elapsed {:?} should be >= expected {:?}", elapsed, expected_min_delay
                );

                Ok(())
            })?;
        }

        /// Property 11: Error Context Completeness
        /// For any error logged by the system, the error record SHALL contain:
        /// a non-empty message, a valid category, a timestamp, and context-specific
        /// suggestions (non-empty list).
        #[test]
        fn prop_error_context_completeness(
            operation in "[a-zA-Z_]{1,50}",
            file_path in prop::option::of("[a-zA-Z0-9/._]{1,100}"),
            retry_count in 0..10u32,
        ) {
            let mut context = ErrorContext::new(operation.clone());

            if let Some(ref path) = file_path {
                context = context.with_file(path);
            }
            context = context.with_retry_count(retry_count);
            context = context.with_backend("test_backend");

            // Create ForgeError for each category
            let categories = [
                ErrorCategory::Network,
                ErrorCategory::FileSystem,
                ErrorCategory::Configuration,
                ErrorCategory::Validation,
                ErrorCategory::Dependency,
                ErrorCategory::Timeout,
                ErrorCategory::Unknown,
            ];

            for category in categories {
                let error = ForgeError::new(
                    category,
                    format!("Test error for {:?}", category),
                    context.clone(),
                );

                // Verify non-empty message
                prop_assert!(!error.message.is_empty(),
                    "Error message should not be empty");

                // Verify valid category
                prop_assert_eq!(error.category, category,
                    "Category should match");

                // Verify timestamp is set (not zero)
                prop_assert!(error.context.timestamp.timestamp() > 0,
                    "Timestamp should be set");

                // Verify operation is preserved
                prop_assert_eq!(&error.context.operation, &operation,
                    "Operation should be preserved");

                // Verify suggestions are non-empty
                let suggestions = error.suggestions();
                prop_assert!(!suggestions.is_empty(),
                    "Suggestions should not be empty for category {:?}", category);

                // Verify platform is set
                prop_assert!(!error.context.platform.is_empty(),
                    "Platform should be set");

                // Verify backend is set
                prop_assert!(!error.context.backend.is_empty(),
                    "Backend should be set");
            }
        }

        /// Property 11 (continued): Verify ForgeError display format
        #[test]
        fn prop_error_display_format(
            msg in "[a-zA-Z0-9 ]{1,100}",
        ) {
            let context = ErrorContext::new("test_op");
            let error = ForgeError::new(
                ErrorCategory::FileSystem,
                msg.clone(),
                context,
            );

            let display = error.to_string();

            // Display should contain category name
            prop_assert!(display.contains("filesystem"),
                "Display should contain category name");

            // Display should contain message
            prop_assert!(display.contains(&msg),
                "Display should contain message");
        }
    }

    /// Test that retryable categories are correctly identified
    #[test]
    fn test_retryable_categories() {
        assert!(ErrorCategory::Network.is_retryable());
        assert!(ErrorCategory::FileSystem.is_retryable());
        assert!(ErrorCategory::Timeout.is_retryable());

        assert!(!ErrorCategory::Configuration.is_retryable());
        assert!(!ErrorCategory::Validation.is_retryable());
        assert!(!ErrorCategory::Dependency.is_retryable());
        assert!(!ErrorCategory::Unknown.is_retryable());
    }

    /// Test ForgeError logging (structured format)
    #[test]
    fn test_structured_error_logging() {
        let context = ErrorContext::new("read_file")
            .with_file("/test/file.txt")
            .with_backend("fallback")
            .with_retry_count(2);

        let error = ForgeError::new(ErrorCategory::FileSystem, "File not found", context);

        // Verify all fields are accessible for structured logging
        assert_eq!(error.category.name(), "filesystem");
        assert_eq!(error.message, "File not found");
        assert_eq!(error.context.operation, "read_file");
        assert_eq!(error.context.file, Some(PathBuf::from("/test/file.txt")));
        assert_eq!(error.context.backend, "fallback");
        assert_eq!(error.context.retry_count, 2);
        assert!(!error.context.platform.is_empty());
    }
}
