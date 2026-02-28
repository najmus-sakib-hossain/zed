//! Configuration Validator for DX Forge
//!
//! Provides comprehensive validation for Forge configuration including:
//! - Required field validation
//! - Range validation with error messages
//! - Path existence validation
//! - Network address validation
//!
//! # Example
//! ```rust,ignore
//! use dx_forge::config::{ConfigValidator, ValidationResult};
//! use dx_forge::core::ForgeConfig;
//!
//! let config = ForgeConfig::new(".");
//! match ConfigValidator::validate(&config) {
//!     ValidationResult::Valid => println!("Configuration is valid"),
//!     ValidationResult::Invalid(errors) => {
//!         for error in errors {
//!             eprintln!("{}", error);
//!         }
//!     }
//! }
//! ```

use std::net::SocketAddr;
use std::path::Path;

/// Validation error with field, message, and suggestion
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Field that failed validation
    pub field: String,
    /// Error message describing the failure
    pub message: String,
    /// Suggestion for fixing the error
    pub suggestion: String,
    /// Valid constraints for the field
    pub constraints: Option<String>,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.field, self.message)?;
        if let Some(ref constraints) = self.constraints {
            write!(f, " (valid: {})", constraints)?;
        }
        Ok(())
    }
}

impl ValidationError {
    /// Create a new validation error
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            suggestion: String::new(),
            constraints: None,
        }
    }

    /// Add a suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = suggestion.into();
        self
    }

    /// Add constraints description
    pub fn with_constraints(mut self, constraints: impl Into<String>) -> Self {
        self.constraints = Some(constraints.into());
        self
    }
}

/// Result of configuration validation
#[derive(Debug)]
pub enum ValidationResult {
    /// Configuration is valid
    Valid,
    /// Configuration has errors
    Invalid(Vec<ValidationError>),
}

impl ValidationResult {
    /// Check if validation passed
    pub fn is_valid(&self) -> bool {
        matches!(self, ValidationResult::Valid)
    }

    /// Get errors if any
    pub fn errors(&self) -> Option<&Vec<ValidationError>> {
        match self {
            ValidationResult::Valid => None,
            ValidationResult::Invalid(errors) => Some(errors),
        }
    }
}

/// Configuration validator
pub struct ConfigValidator {
    errors: Vec<ValidationError>,
}

impl ConfigValidator {
    /// Create a new validator
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Validate a ForgeConfig
    pub fn validate(config: &crate::core::ForgeConfig) -> ValidationResult {
        let mut validator = Self::new();

        validator.validate_paths(config);
        validator.validate_limits(config);

        if validator.errors.is_empty() {
            ValidationResult::Valid
        } else {
            ValidationResult::Invalid(validator.errors)
        }
    }

    /// Validate path fields
    fn validate_paths(&mut self, config: &crate::core::ForgeConfig) {
        // Validate project_root exists
        if !config.project_root.exists() {
            self.errors.push(
                ValidationError::new(
                    "project_root",
                    format!("Path does not exist: {}", config.project_root.display()),
                )
                .with_suggestion("Ensure the project directory exists before initializing Forge")
                .with_constraints("must be an existing directory"),
            );
        } else if !config.project_root.is_dir() {
            self.errors.push(
                ValidationError::new(
                    "project_root",
                    format!("Path is not a directory: {}", config.project_root.display()),
                )
                .with_suggestion("Provide a directory path, not a file path")
                .with_constraints("must be a directory"),
            );
        }

        // Note: forge_dir parent validation removed - the directory structure
        // (.dx/forge) will be created automatically if it doesn't exist
    }

    /// Validate numeric limits and ranges
    fn validate_limits(&mut self, config: &crate::core::ForgeConfig) {
        // Validate worker_threads
        if config.worker_threads == 0 {
            self.errors.push(
                ValidationError::new(
                    "worker_threads",
                    "Worker threads cannot be zero",
                )
                .with_suggestion("Set worker_threads to at least 1, or use num_cpus::get() for automatic detection")
                .with_constraints("1 to 256"),
            );
        } else if config.worker_threads > 256 {
            self.errors.push(
                ValidationError::new(
                    "worker_threads",
                    format!("Worker threads too high: {}", config.worker_threads),
                )
                .with_suggestion("Reduce worker_threads to a reasonable value (typically num_cpus)")
                .with_constraints("1 to 256"),
            );
        }
    }

    /// Validate a required string field
    pub fn validate_required_string(&mut self, field: &str, value: &str) {
        if value.is_empty() {
            self.errors.push(
                ValidationError::new(field, "Field is required but empty")
                    .with_suggestion(format!("Provide a value for {}", field))
                    .with_constraints("non-empty string"),
            );
        }
    }

    /// Validate a numeric range
    pub fn validate_range<T: PartialOrd + std::fmt::Display>(
        &mut self,
        field: &str,
        value: T,
        min: T,
        max: T,
    ) {
        if value < min || value > max {
            self.errors.push(
                ValidationError::new(field, format!("Value {} is out of range", value))
                    .with_suggestion(format!(
                        "Set {} to a value between {} and {}",
                        field, min, max
                    ))
                    .with_constraints(format!("{} to {}", min, max)),
            );
        }
    }

    /// Validate a path exists
    pub fn validate_path_exists(&mut self, field: &str, path: &Path) {
        if !path.exists() {
            self.errors.push(
                ValidationError::new(field, format!("Path does not exist: {}", path.display()))
                    .with_suggestion("Ensure the path exists or create it before validation")
                    .with_constraints("existing path"),
            );
        }
    }

    /// Validate a path is a directory
    pub fn validate_is_directory(&mut self, field: &str, path: &Path) {
        if path.exists() && !path.is_dir() {
            self.errors.push(
                ValidationError::new(field, format!("Path is not a directory: {}", path.display()))
                    .with_suggestion("Provide a directory path")
                    .with_constraints("directory path"),
            );
        }
    }

    /// Validate a network address
    pub fn validate_network_address(&mut self, field: &str, address: &str) {
        if address.is_empty() {
            self.errors.push(
                ValidationError::new(field, "Network address is empty")
                    .with_suggestion(
                        "Provide a valid address like '127.0.0.1:8080' or '[::1]:8080'",
                    )
                    .with_constraints("valid socket address"),
            );
            return;
        }

        if address.parse::<SocketAddr>().is_err() {
            self.errors.push(
                ValidationError::new(field, format!("Invalid network address: {}", address))
                    .with_suggestion("Use format 'host:port' like '127.0.0.1:8080' or '[::1]:8080'")
                    .with_constraints("valid socket address (ip:port)"),
            );
        }
    }

    /// Validate a URL
    pub fn validate_url(&mut self, field: &str, url: &str) {
        if url.is_empty() {
            self.errors.push(
                ValidationError::new(field, "URL is empty")
                    .with_suggestion("Provide a valid URL")
                    .with_constraints("valid URL"),
            );
            return;
        }

        // Basic URL validation
        if !url.starts_with("http://") && !url.starts_with("https://") {
            self.errors.push(
                ValidationError::new(field, format!("Invalid URL scheme: {}", url))
                    .with_suggestion("URL must start with 'http://' or 'https://'")
                    .with_constraints("http:// or https:// URL"),
            );
        }
    }

    /// Get all validation errors
    pub fn errors(&self) -> &[ValidationError] {
        &self.errors
    }

    /// Check if validation passed
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Convert to ValidationResult
    pub fn into_result(self) -> ValidationResult {
        if self.errors.is_empty() {
            ValidationResult::Valid
        } else {
            ValidationResult::Invalid(self.errors)
        }
    }
}

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_validation_error_display() {
        let error = ValidationError::new("test_field", "test message")
            .with_suggestion("test suggestion")
            .with_constraints("1 to 10");

        let display = error.to_string();
        assert!(display.contains("test_field"));
        assert!(display.contains("test message"));
        assert!(display.contains("1 to 10"));
    }

    #[test]
    fn test_validate_required_string() {
        let mut validator = ConfigValidator::new();

        validator.validate_required_string("name", "valid");
        assert!(validator.is_valid());

        validator.validate_required_string("empty", "");
        assert!(!validator.is_valid());
        assert_eq!(validator.errors().len(), 1);
    }

    #[test]
    fn test_validate_range() {
        let mut validator = ConfigValidator::new();

        validator.validate_range("threads", 4, 1, 256);
        assert!(validator.is_valid());

        validator.validate_range("threads", 0, 1, 256);
        assert!(!validator.is_valid());

        let mut validator2 = ConfigValidator::new();
        validator2.validate_range("threads", 300, 1, 256);
        assert!(!validator2.is_valid());
    }

    #[test]
    fn test_validate_network_address() {
        let mut validator = ConfigValidator::new();

        validator.validate_network_address("addr", "127.0.0.1:8080");
        assert!(validator.is_valid());

        let mut validator2 = ConfigValidator::new();
        validator2.validate_network_address("addr", "[::1]:8080");
        assert!(validator2.is_valid());

        let mut validator3 = ConfigValidator::new();
        validator3.validate_network_address("addr", "invalid");
        assert!(!validator3.is_valid());

        let mut validator4 = ConfigValidator::new();
        validator4.validate_network_address("addr", "");
        assert!(!validator4.is_valid());
    }

    #[test]
    fn test_validate_url() {
        let mut validator = ConfigValidator::new();

        validator.validate_url("url", "https://example.com");
        assert!(validator.is_valid());

        let mut validator2 = ConfigValidator::new();
        validator2.validate_url("url", "http://localhost:8080");
        assert!(validator2.is_valid());

        let mut validator3 = ConfigValidator::new();
        validator3.validate_url("url", "ftp://invalid.com");
        assert!(!validator3.is_valid());

        let mut validator4 = ConfigValidator::new();
        validator4.validate_url("url", "");
        assert!(!validator4.is_valid());
    }

    #[test]
    fn test_validation_result() {
        let valid = ValidationResult::Valid;
        assert!(valid.is_valid());
        assert!(valid.errors().is_none());

        let invalid = ValidationResult::Invalid(vec![ValidationError::new("field", "error")]);
        assert!(!invalid.is_valid());
        assert!(invalid.errors().is_some());
        assert_eq!(invalid.errors().unwrap().len(), 1);
    }
}

/// Property-based tests for configuration validation
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use std::path::PathBuf;

    /// Generate arbitrary field names
    fn field_name_strategy() -> impl Strategy<Value = String> {
        "[a-z_]{1,20}".prop_map(|s| s.to_string())
    }

    /// Generate arbitrary error messages
    fn error_message_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 ]{1,100}".prop_map(|s| s.to_string())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 15: Configuration Validation Completeness
        /// For any configuration with invalid values (missing required fields,
        /// out-of-range values, non-existent paths, malformed addresses),
        /// the validator SHALL return an error containing a description of
        /// each invalid field and the valid constraints.
        #[test]
        fn prop_validation_completeness_required_fields(
            field in field_name_strategy(),
            value in prop::option::of("[a-zA-Z0-9]{0,50}"),
        ) {
            let mut validator = ConfigValidator::new();

            let actual_value = value.unwrap_or_default();
            validator.validate_required_string(&field, &actual_value);

            if actual_value.is_empty() {
                // Empty values should produce an error
                prop_assert!(!validator.is_valid(),
                    "Empty required field should fail validation");

                let errors = validator.errors();
                prop_assert!(!errors.is_empty(),
                    "Should have at least one error");

                // Error should contain field name
                let error = &errors[0];
                prop_assert_eq!(&error.field, &field,
                    "Error should reference the correct field");

                // Error should have a message
                prop_assert!(!error.message.is_empty(),
                    "Error should have a message");

                // Error should have constraints
                prop_assert!(error.constraints.is_some(),
                    "Error should have constraints");
            } else {
                // Non-empty values should pass
                prop_assert!(validator.is_valid(),
                    "Non-empty required field should pass validation");
            }
        }

        /// Property 15 (continued): Range validation
        #[test]
        fn prop_validation_completeness_range(
            field in field_name_strategy(),
            value in 0i64..1000i64,
            min in 0i64..100i64,
            max in 100i64..500i64,
        ) {
            let mut validator = ConfigValidator::new();
            validator.validate_range(&field, value, min, max);

            if value < min || value > max {
                // Out of range should produce an error
                prop_assert!(!validator.is_valid(),
                    "Out of range value {} should fail (range: {}-{})", value, min, max);

                let errors = validator.errors();
                prop_assert!(!errors.is_empty(),
                    "Should have at least one error");

                let error = &errors[0];
                prop_assert_eq!(&error.field, &field,
                    "Error should reference the correct field");

                // Error should describe the valid range
                prop_assert!(error.constraints.is_some(),
                    "Error should have constraints describing valid range");

                let constraints = error.constraints.as_ref().unwrap();
                prop_assert!(constraints.contains(&min.to_string()),
                    "Constraints should mention min value");
                prop_assert!(constraints.contains(&max.to_string()),
                    "Constraints should mention max value");
            } else {
                // In range should pass
                prop_assert!(validator.is_valid(),
                    "In range value {} should pass (range: {}-{})", value, min, max);
            }
        }

        /// Property 15 (continued): Path validation
        #[test]
        fn prop_validation_completeness_path(
            field in field_name_strategy(),
        ) {
            let mut validator = ConfigValidator::new();

            // Non-existent path should fail
            let non_existent = PathBuf::from("/this/path/definitely/does/not/exist/12345");
            validator.validate_path_exists(&field, &non_existent);

            prop_assert!(!validator.is_valid(),
                "Non-existent path should fail validation");

            let errors = validator.errors();
            prop_assert!(!errors.is_empty(),
                "Should have at least one error");

            let error = &errors[0];
            prop_assert_eq!(&error.field, &field,
                "Error should reference the correct field");

            // Error message should mention the path
            prop_assert!(error.message.contains("does not exist") ||
                        error.message.contains(&non_existent.display().to_string()),
                "Error should describe the path issue");

            // Error should have constraints
            prop_assert!(error.constraints.is_some(),
                "Error should have constraints");
        }

        /// Property 15 (continued): Network address validation
        #[test]
        fn prop_validation_completeness_network_address(
            field in field_name_strategy(),
            port in 1u16..65535u16,
        ) {
            // Valid IPv4 address
            let mut validator = ConfigValidator::new();
            let valid_addr = format!("127.0.0.1:{}", port);
            validator.validate_network_address(&field, &valid_addr);
            prop_assert!(validator.is_valid(),
                "Valid IPv4 address should pass: {}", valid_addr);

            // Valid IPv6 address
            let mut validator = ConfigValidator::new();
            let valid_ipv6 = format!("[::1]:{}", port);
            validator.validate_network_address(&field, &valid_ipv6);
            prop_assert!(validator.is_valid(),
                "Valid IPv6 address should pass: {}", valid_ipv6);

            // Invalid address
            let mut validator = ConfigValidator::new();
            validator.validate_network_address(&field, "not-an-address");
            prop_assert!(!validator.is_valid(),
                "Invalid address should fail");

            let errors = validator.errors();
            prop_assert!(!errors.is_empty());

            let error = &errors[0];
            prop_assert_eq!(&error.field, &field);
            prop_assert!(error.constraints.is_some(),
                "Error should have constraints describing valid format");
        }

        /// Property 15 (continued): All errors are reported
        #[test]
        fn prop_validation_reports_all_errors(
            num_errors in 1..10usize,
        ) {
            let mut validator = ConfigValidator::new();

            // Add multiple validation errors
            for i in 0..num_errors {
                validator.validate_required_string(&format!("field_{}", i), "");
            }

            prop_assert!(!validator.is_valid());

            let errors = validator.errors();
            prop_assert_eq!(errors.len(), num_errors,
                "Should report all {} errors", num_errors);

            // Each error should have unique field name
            let fields: std::collections::HashSet<_> = errors.iter()
                .map(|e| e.field.clone())
                .collect();
            prop_assert_eq!(fields.len(), num_errors,
                "Each error should have unique field name");
        }

        /// Property 15 (continued): ValidationError has complete information
        #[test]
        fn prop_validation_error_completeness(
            field in field_name_strategy(),
            message in error_message_strategy(),
            suggestion in error_message_strategy(),
            constraints in error_message_strategy(),
        ) {
            let error = ValidationError::new(&field, &message)
                .with_suggestion(&suggestion)
                .with_constraints(&constraints);

            // All fields should be preserved
            prop_assert_eq!(&error.field, &field);
            prop_assert_eq!(&error.message, &message);
            prop_assert_eq!(&error.suggestion, &suggestion);
            prop_assert_eq!(error.constraints.as_ref(), Some(&constraints));

            // Display should include field and message
            let display = error.to_string();
            prop_assert!(display.contains(&field),
                "Display should contain field name");
            prop_assert!(display.contains(&message),
                "Display should contain message");
            prop_assert!(display.contains(&constraints),
                "Display should contain constraints");
        }
    }

    /// Test ForgeConfig validation with valid config
    #[test]
    fn test_forge_config_validation_valid() {
        use std::env;

        // Use current directory which should exist
        let current_dir = env::current_dir().unwrap();
        let config = crate::core::ForgeConfig::new(&current_dir);

        let result = ConfigValidator::validate(&config);
        assert!(result.is_valid(), "Valid config should pass validation");
    }

    /// Test ForgeConfig validation with invalid worker_threads
    #[test]
    fn test_forge_config_validation_invalid_threads() {
        use std::env;

        let current_dir = env::current_dir().unwrap();
        let mut config = crate::core::ForgeConfig::new(&current_dir);
        config.worker_threads = 0;

        let result = ConfigValidator::validate(&config);
        assert!(!result.is_valid(), "Zero worker_threads should fail");

        let errors = result.errors().unwrap();
        assert!(
            errors.iter().any(|e| e.field == "worker_threads"),
            "Should have error for worker_threads"
        );
    }
}
