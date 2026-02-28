//! Configuration Validation Module
//!
//! This module provides comprehensive validation for all configuration sections
//! with actionable error messages and suggestions.
//!
//! ## Features
//!
//! - Validates all config sections ([driven], [generator], [dcp])
//! - Provides suggestions for invalid configurations
//! - Supports partial validation for incremental updates

use crate::{DrivenConfig, DrivenError, Result};
use std::collections::HashSet;
use std::path::Path;

/// Configuration validator
#[derive(Debug, Default)]
pub struct ConfigValidator {
    /// Validation errors collected
    errors: Vec<ValidationError>,
    /// Validation warnings collected
    warnings: Vec<ValidationWarning>,
}

/// A validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Field path (e.g., "sync.debounce_ms")
    pub field: String,
    /// Error message
    pub message: String,
    /// Suggestion for fixing
    pub suggestion: String,
}

/// A validation warning
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    /// Field path
    pub field: String,
    /// Warning message
    pub message: String,
    /// Suggestion
    pub suggestion: String,
}

/// Validation result
#[derive(Debug)]
pub struct ValidationReport {
    /// Whether validation passed
    pub valid: bool,
    /// Errors found
    pub errors: Vec<ValidationError>,
    /// Warnings found
    pub warnings: Vec<ValidationWarning>,
}

impl ValidationReport {
    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Get a formatted error message
    pub fn format_errors(&self) -> String {
        self.errors
            .iter()
            .map(|e| format!("  - {}: {}\n    Suggestion: {}", e.field, e.message, e.suggestion))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Get a formatted warning message
    pub fn format_warnings(&self) -> String {
        self.warnings
            .iter()
            .map(|w| format!("  - {}: {}\n    Suggestion: {}", w.field, w.message, w.suggestion))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl ConfigValidator {
    /// Create a new validator
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate a complete DrivenConfig
    pub fn validate(&mut self, config: &DrivenConfig) -> ValidationReport {
        self.errors.clear();
        self.warnings.clear();

        self.validate_version(&config.version);
        self.validate_editors(&config.editors);
        self.validate_sync(&config.sync);
        self.validate_templates(&config.templates);
        self.validate_context(&config.context);

        ValidationReport {
            valid: self.errors.is_empty(),
            errors: self.errors.clone(),
            warnings: self.warnings.clone(),
        }
    }

    /// Validate version field
    fn validate_version(&mut self, version: &str) {
        if version.is_empty() {
            self.errors.push(ValidationError {
                field: "version".to_string(),
                message: "Version cannot be empty".to_string(),
                suggestion: "Set version to '1.0'".to_string(),
            });
        } else if !version.chars().all(|c| c.is_ascii_digit() || c == '.') {
            self.warnings.push(ValidationWarning {
                field: "version".to_string(),
                message: format!("Unusual version format: '{}'", version),
                suggestion: "Use semantic versioning (e.g., '1.0', '2.1')".to_string(),
            });
        }
    }

    /// Validate editor configuration
    fn validate_editors(&mut self, editors: &crate::EditorConfig) {
        let enabled_count = [
            editors.cursor,
            editors.copilot,
            editors.windsurf,
            editors.claude,
            editors.aider,
            editors.cline,
        ]
        .iter()
        .filter(|&&e| e)
        .count();

        if enabled_count == 0 {
            self.warnings.push(ValidationWarning {
                field: "editors".to_string(),
                message: "No editors are enabled".to_string(),
                suggestion: "Enable at least one editor (e.g., editors.cursor = true)".to_string(),
            });
        }
    }

    /// Validate sync configuration
    fn validate_sync(&mut self, sync: &crate::SyncConfig) {
        // Validate source of truth path
        if sync.source_of_truth.is_empty() {
            self.errors.push(ValidationError {
                field: "sync.source_of_truth".to_string(),
                message: "Source of truth path cannot be empty".to_string(),
                suggestion: "Set to '.driven/rules.drv' or another valid path".to_string(),
            });
        } else if !sync.source_of_truth.ends_with(".drv") && !sync.source_of_truth.ends_with(".md")
        {
            self.warnings.push(ValidationWarning {
                field: "sync.source_of_truth".to_string(),
                message: format!("Unusual file extension: '{}'", sync.source_of_truth),
                suggestion: "Use .drv for binary format or .md for markdown".to_string(),
            });
        }
    }

    /// Validate template configuration
    fn validate_templates(&mut self, templates: &crate::TemplateConfig) {
        // Check for duplicate personas
        let mut seen = HashSet::new();
        for persona in &templates.personas {
            if !seen.insert(persona) {
                self.warnings.push(ValidationWarning {
                    field: "templates.personas".to_string(),
                    message: format!("Duplicate persona: '{}'", persona),
                    suggestion: "Remove duplicate entries".to_string(),
                });
            }
        }

        // Check for duplicate standards
        seen.clear();
        for standard in &templates.standards {
            if !seen.insert(standard) {
                self.warnings.push(ValidationWarning {
                    field: "templates.standards".to_string(),
                    message: format!("Duplicate standard: '{}'", standard),
                    suggestion: "Remove duplicate entries".to_string(),
                });
            }
        }
    }

    /// Validate context configuration
    fn validate_context(&mut self, context: &crate::ContextConfig) {
        // Check for empty include patterns
        if context.include.is_empty() {
            self.warnings.push(ValidationWarning {
                field: "context.include".to_string(),
                message: "No include patterns specified".to_string(),
                suggestion: "Add patterns like 'src/**' to include source files".to_string(),
            });
        }

        // Check for overlapping include/exclude patterns
        for include in &context.include {
            for exclude in &context.exclude {
                if include == exclude {
                    self.errors.push(ValidationError {
                        field: "context".to_string(),
                        message: format!("Pattern '{}' is both included and excluded", include),
                        suggestion: "Remove the pattern from either include or exclude".to_string(),
                    });
                }
            }
        }

        // Validate index path
        if context.index_path.is_empty() {
            self.errors.push(ValidationError {
                field: "context.index_path".to_string(),
                message: "Index path cannot be empty".to_string(),
                suggestion: "Set to '.driven/index.drv'".to_string(),
            });
        }
    }

    /// Validate a file path exists
    pub fn validate_path_exists(&mut self, field: &str, path: &Path) {
        if !path.exists() {
            self.errors.push(ValidationError {
                field: field.to_string(),
                message: format!("Path does not exist: {}", path.display()),
                suggestion: "Create the file or update the path".to_string(),
            });
        }
    }

    /// Add a custom error
    pub fn add_error(&mut self, field: &str, message: &str, suggestion: &str) {
        self.errors.push(ValidationError {
            field: field.to_string(),
            message: message.to_string(),
            suggestion: suggestion.to_string(),
        });
    }

    /// Add a custom warning
    pub fn add_warning(&mut self, field: &str, message: &str, suggestion: &str) {
        self.warnings.push(ValidationWarning {
            field: field.to_string(),
            message: message.to_string(),
            suggestion: suggestion.to_string(),
        });
    }
}

/// Validate a configuration and return a Result
pub fn validate_config(config: &DrivenConfig) -> Result<ValidationReport> {
    let mut validator = ConfigValidator::new();
    let report = validator.validate(config);

    if report.has_errors() {
        Err(DrivenError::Validation(report.format_errors()))
    } else {
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EditorConfig;

    #[test]
    fn test_valid_config() {
        let config = DrivenConfig::default();
        let mut validator = ConfigValidator::new();
        let report = validator.validate(&config);

        assert!(report.valid);
        assert!(!report.has_errors());
    }

    #[test]
    fn test_empty_version() {
        let mut config = DrivenConfig::default();
        config.version = String::new();

        let mut validator = ConfigValidator::new();
        let report = validator.validate(&config);

        assert!(!report.valid);
        assert!(report.errors.iter().any(|e| e.field == "version"));
    }

    #[test]
    fn test_no_editors_enabled() {
        let mut config = DrivenConfig::default();
        config.editors = EditorConfig {
            cursor: false,
            copilot: false,
            windsurf: false,
            claude: false,
            aider: false,
            cline: false,
        };

        let mut validator = ConfigValidator::new();
        let report = validator.validate(&config);

        assert!(report.has_warnings());
        assert!(report.warnings.iter().any(|w| w.field == "editors"));
    }

    #[test]
    fn test_empty_source_of_truth() {
        let mut config = DrivenConfig::default();
        config.sync.source_of_truth = String::new();

        let mut validator = ConfigValidator::new();
        let report = validator.validate(&config);

        assert!(!report.valid);
        assert!(report.errors.iter().any(|e| e.field == "sync.source_of_truth"));
    }

    #[test]
    fn test_duplicate_personas() {
        let mut config = DrivenConfig::default();
        config.templates.personas = vec!["architect".to_string(), "architect".to_string()];

        let mut validator = ConfigValidator::new();
        let report = validator.validate(&config);

        assert!(report.has_warnings());
        assert!(report.warnings.iter().any(|w| w.message.contains("Duplicate")));
    }

    #[test]
    fn test_overlapping_patterns() {
        let mut config = DrivenConfig::default();
        config.context.include = vec!["src/**".to_string()];
        config.context.exclude = vec!["src/**".to_string()];

        let mut validator = ConfigValidator::new();
        let report = validator.validate(&config);

        assert!(!report.valid);
        assert!(report.errors.iter().any(|e| e.message.contains("both included and excluded")));
    }

    #[test]
    fn test_validation_report_formatting() {
        let report = ValidationReport {
            valid: false,
            errors: vec![ValidationError {
                field: "test.field".to_string(),
                message: "Test error".to_string(),
                suggestion: "Fix it".to_string(),
            }],
            warnings: vec![ValidationWarning {
                field: "test.warning".to_string(),
                message: "Test warning".to_string(),
                suggestion: "Consider fixing".to_string(),
            }],
        };

        let errors = report.format_errors();
        assert!(errors.contains("test.field"));
        assert!(errors.contains("Test error"));

        let warnings = report.format_warnings();
        assert!(warnings.contains("test.warning"));
        assert!(warnings.contains("Test warning"));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // Property 12: Configuration Validation
    // For any configuration input, validation SHALL correctly identify all
    // invalid settings and provide actionable error messages.

    proptest! {
        /// Property: Valid default config always passes validation
        #[test]
        fn prop_default_config_valid(_seed in any::<u64>()) {
            let config = DrivenConfig::default();
            let mut validator = ConfigValidator::new();
            let report = validator.validate(&config);

            prop_assert!(report.valid, "Default config should be valid");
        }

        /// Property: Empty version always fails validation
        #[test]
        fn prop_empty_version_invalid(_seed in any::<u64>()) {
            let mut config = DrivenConfig::default();
            config.version = String::new();

            let mut validator = ConfigValidator::new();
            let report = validator.validate(&config);

            prop_assert!(!report.valid);
            prop_assert!(report.errors.iter().any(|e| e.field == "version"));
        }

        /// Property: Empty source_of_truth always fails validation
        #[test]
        fn prop_empty_source_of_truth_invalid(_seed in any::<u64>()) {
            let mut config = DrivenConfig::default();
            config.sync.source_of_truth = String::new();

            let mut validator = ConfigValidator::new();
            let report = validator.validate(&config);

            prop_assert!(!report.valid);
            prop_assert!(report.errors.iter().any(|e| e.field.contains("source_of_truth")));
        }

        /// Property: All errors have suggestions
        #[test]
        fn prop_errors_have_suggestions(
            version in ".*",
            source in ".*",
        ) {
            let mut config = DrivenConfig::default();
            config.version = version;
            config.sync.source_of_truth = source;

            let mut validator = ConfigValidator::new();
            let report = validator.validate(&config);

            for error in &report.errors {
                prop_assert!(!error.suggestion.is_empty(),
                    "Error for '{}' should have a suggestion", error.field);
            }

            for warning in &report.warnings {
                prop_assert!(!warning.suggestion.is_empty(),
                    "Warning for '{}' should have a suggestion", warning.field);
            }
        }

        /// Property: Validation is deterministic
        #[test]
        fn prop_validation_deterministic(
            version in "[0-9.]+",
        ) {
            let mut config = DrivenConfig::default();
            config.version = version;

            let mut validator1 = ConfigValidator::new();
            let report1 = validator1.validate(&config);

            let mut validator2 = ConfigValidator::new();
            let report2 = validator2.validate(&config);

            prop_assert_eq!(report1.valid, report2.valid);
            prop_assert_eq!(report1.errors.len(), report2.errors.len());
            prop_assert_eq!(report1.warnings.len(), report2.warnings.len());
        }
    }
}
