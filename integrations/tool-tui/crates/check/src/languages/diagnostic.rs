//! Unified Diagnostic Structure for Multi-Language Support
//!
//! This module provides a consistent diagnostic format across all
//! supported programming languages.

use std::fmt;

/// Diagnostic severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Error - definite problem that must be fixed
    Error,
    /// Warning - potential issue that should be reviewed
    Warning,
    /// Info - informational message
    Info,
}

impl Severity {
    /// Returns the string representation of the severity
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }

    /// Returns a symbol for terminal output
    #[must_use]
    pub fn symbol(&self) -> &'static str {
        match self {
            Severity::Error => "✖",
            Severity::Warning => "⚠",
            Severity::Info => "ℹ",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Unified diagnostic format across all languages
///
/// This structure provides a consistent way to report errors, warnings,
/// and informational messages from formatters and linters across all
/// supported programming languages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// File path where the diagnostic occurred
    pub file_path: String,
    /// Diagnostic message
    pub message: String,
    /// Line number (1-indexed, optional)
    pub line: Option<usize>,
    /// Column number (1-indexed, optional)
    pub column: Option<usize>,
    /// Severity level
    pub severity: Severity,
    /// Category (e.g., "format/python", "lint/cpp")
    pub category: String,
    /// Rule name (for lint diagnostics, optional)
    pub rule: Option<String>,
}

impl Diagnostic {
    /// Create a new diagnostic with all fields
    pub fn new(
        file_path: impl Into<String>,
        message: impl Into<String>,
        severity: Severity,
        category: impl Into<String>,
    ) -> Self {
        Self {
            file_path: file_path.into(),
            message: message.into(),
            line: None,
            column: None,
            severity,
            category: category.into(),
            rule: None,
        }
    }

    /// Create an error diagnostic
    pub fn error(
        file_path: impl Into<String>,
        message: impl Into<String>,
        category: impl Into<String>,
    ) -> Self {
        Self::new(file_path, message, Severity::Error, category)
    }

    /// Create a warning diagnostic
    pub fn warning(
        file_path: impl Into<String>,
        message: impl Into<String>,
        category: impl Into<String>,
    ) -> Self {
        Self::new(file_path, message, Severity::Warning, category)
    }

    /// Create an info diagnostic
    pub fn info(
        file_path: impl Into<String>,
        message: impl Into<String>,
        category: impl Into<String>,
    ) -> Self {
        Self::new(file_path, message, Severity::Info, category)
    }

    /// Set the line number (1-indexed)
    #[must_use]
    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Set the column number (1-indexed)
    #[must_use]
    pub fn with_column(mut self, column: usize) -> Self {
        self.column = Some(column);
        self
    }

    /// Set both line and column (1-indexed)
    #[must_use]
    pub fn with_location(mut self, line: usize, column: usize) -> Self {
        self.line = Some(line);
        self.column = Some(column);
        self
    }

    /// Set the rule name
    pub fn with_rule(mut self, rule: impl Into<String>) -> Self {
        self.rule = Some(rule.into());
        self
    }

    /// Check if this diagnostic has a valid structure
    ///
    /// A valid diagnostic must have:
    /// - Non-empty `file_path`
    /// - Non-empty message
    /// - Non-empty category matching pattern "{operation}/{language}"
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.file_path.is_empty()
            && !self.message.is_empty()
            && !self.category.is_empty()
            && self.category.contains('/')
    }

    /// Check if the category matches the expected pattern "{operation}/{language}"
    #[must_use]
    pub fn has_valid_category(&self) -> bool {
        if self.category.is_empty() {
            return false;
        }
        let parts: Vec<&str> = self.category.split('/').collect();
        parts.len() == 2 && !parts[0].is_empty() && !parts[1].is_empty()
    }

    /// Get the operation part of the category (e.g., "format", "lint", "io")
    #[must_use]
    pub fn operation(&self) -> Option<&str> {
        self.category.split('/').next()
    }

    /// Get the language part of the category (e.g., "python", "cpp", "rust")
    #[must_use]
    pub fn language(&self) -> Option<&str> {
        self.category.split('/').nth(1)
    }

    /// Format the location string (e.g., "file.py:10:5" or "file.py:10" or "file.py")
    #[must_use]
    pub fn location_string(&self) -> String {
        match (self.line, self.column) {
            (Some(line), Some(col)) => format!("{}:{}:{}", self.file_path, line, col),
            (Some(line), None) => format!("{}:{}", self.file_path, line),
            _ => self.file_path.clone(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format: severity[category]: message
        //   --> file:line:column
        //   rule: rule_name (if present)

        write!(
            f,
            "{} {}[{}]: {}",
            self.severity.symbol(),
            self.severity,
            self.category,
            self.message
        )?;

        write!(f, "\n  --> {}", self.location_string())?;

        if let Some(ref rule) = self.rule {
            write!(f, "\n  rule: {rule}")?;
        }

        Ok(())
    }
}

/// Builder for constructing diagnostics with a fluent API
#[derive(Debug, Clone, Default)]
pub struct DiagnosticBuilder {
    file_path: Option<String>,
    message: Option<String>,
    line: Option<usize>,
    column: Option<usize>,
    severity: Option<Severity>,
    category: Option<String>,
    rule: Option<String>,
}

impl DiagnosticBuilder {
    /// Create a new diagnostic builder
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the file path
    pub fn file_path(mut self, file_path: impl Into<String>) -> Self {
        self.file_path = Some(file_path.into());
        self
    }

    /// Set the message
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Set the line number (1-indexed)
    #[must_use]
    pub fn line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Set the column number (1-indexed)
    #[must_use]
    pub fn column(mut self, column: usize) -> Self {
        self.column = Some(column);
        self
    }

    /// Set the severity
    #[must_use]
    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = Some(severity);
        self
    }

    /// Set the category
    pub fn category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Set the rule name
    pub fn rule(mut self, rule: impl Into<String>) -> Self {
        self.rule = Some(rule.into());
        self
    }

    /// Build the diagnostic
    ///
    /// # Panics
    /// Panics if required fields (`file_path`, message, severity, category) are not set
    #[must_use]
    pub fn build(self) -> Diagnostic {
        Diagnostic {
            file_path: self.file_path.expect("file_path is required"),
            message: self.message.expect("message is required"),
            line: self.line,
            column: self.column,
            severity: self.severity.expect("severity is required"),
            category: self.category.expect("category is required"),
            rule: self.rule,
        }
    }

    /// Try to build the diagnostic, returning None if required fields are missing
    #[must_use]
    pub fn try_build(self) -> Option<Diagnostic> {
        Some(Diagnostic {
            file_path: self.file_path?,
            message: self.message?,
            line: self.line,
            column: self.column,
            severity: self.severity?,
            category: self.category?,
            rule: self.rule,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_as_str() {
        assert_eq!(Severity::Error.as_str(), "error");
        assert_eq!(Severity::Warning.as_str(), "warning");
        assert_eq!(Severity::Info.as_str(), "info");
    }

    #[test]
    fn test_severity_symbol() {
        assert_eq!(Severity::Error.symbol(), "✖");
        assert_eq!(Severity::Warning.symbol(), "⚠");
        assert_eq!(Severity::Info.symbol(), "ℹ");
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(format!("{}", Severity::Error), "error");
        assert_eq!(format!("{}", Severity::Warning), "warning");
        assert_eq!(format!("{}", Severity::Info), "info");
    }

    #[test]
    fn test_diagnostic_new() {
        let diag = Diagnostic::new("test.py", "Test message", Severity::Error, "format/python");
        assert_eq!(diag.file_path, "test.py");
        assert_eq!(diag.message, "Test message");
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.category, "format/python");
        assert!(diag.line.is_none());
        assert!(diag.column.is_none());
        assert!(diag.rule.is_none());
    }

    #[test]
    fn test_diagnostic_error() {
        let diag = Diagnostic::error("test.py", "Syntax error", "lint/python");
        assert_eq!(diag.severity, Severity::Error);
    }

    #[test]
    fn test_diagnostic_warning() {
        let diag = Diagnostic::warning("test.py", "Unused variable", "lint/python");
        assert_eq!(diag.severity, Severity::Warning);
    }

    #[test]
    fn test_diagnostic_info() {
        let diag = Diagnostic::info("test.py", "File formatted", "format/python");
        assert_eq!(diag.severity, Severity::Info);
    }

    #[test]
    fn test_diagnostic_with_location() {
        let diag = Diagnostic::error("test.py", "Error", "lint/python").with_location(10, 5);
        assert_eq!(diag.line, Some(10));
        assert_eq!(diag.column, Some(5));
    }

    #[test]
    fn test_diagnostic_with_rule() {
        let diag =
            Diagnostic::warning("test.py", "Warning", "lint/python").with_rule("no-unused-vars");
        assert_eq!(diag.rule, Some("no-unused-vars".to_string()));
    }

    #[test]
    fn test_diagnostic_is_valid() {
        let valid = Diagnostic::error("test.py", "Error", "format/python");
        assert!(valid.is_valid());

        let invalid_empty_path = Diagnostic::error("", "Error", "format/python");
        assert!(!invalid_empty_path.is_valid());

        let invalid_empty_message = Diagnostic::error("test.py", "", "format/python");
        assert!(!invalid_empty_message.is_valid());

        let invalid_empty_category = Diagnostic::new("test.py", "Error", Severity::Error, "");
        assert!(!invalid_empty_category.is_valid());

        let invalid_no_slash = Diagnostic::new("test.py", "Error", Severity::Error, "format");
        assert!(!invalid_no_slash.is_valid());
    }

    #[test]
    fn test_diagnostic_has_valid_category() {
        let valid = Diagnostic::error("test.py", "Error", "format/python");
        assert!(valid.has_valid_category());

        let invalid = Diagnostic::new("test.py", "Error", Severity::Error, "invalid");
        assert!(!invalid.has_valid_category());

        let invalid_empty_parts = Diagnostic::new("test.py", "Error", Severity::Error, "/python");
        assert!(!invalid_empty_parts.has_valid_category());
    }

    #[test]
    fn test_diagnostic_operation_and_language() {
        let diag = Diagnostic::error("test.py", "Error", "lint/python");
        assert_eq!(diag.operation(), Some("lint"));
        assert_eq!(diag.language(), Some("python"));
    }

    #[test]
    fn test_diagnostic_location_string() {
        let diag_full = Diagnostic::error("test.py", "Error", "lint/python").with_location(10, 5);
        assert_eq!(diag_full.location_string(), "test.py:10:5");

        let diag_line = Diagnostic::error("test.py", "Error", "lint/python").with_line(10);
        assert_eq!(diag_line.location_string(), "test.py:10");

        let diag_none = Diagnostic::error("test.py", "Error", "lint/python");
        assert_eq!(diag_none.location_string(), "test.py");
    }

    #[test]
    fn test_diagnostic_display() {
        let diag = Diagnostic::error("test.py", "Syntax error", "lint/python")
            .with_location(10, 5)
            .with_rule("syntax");

        let output = format!("{}", diag);
        assert!(output.contains("error"));
        assert!(output.contains("lint/python"));
        assert!(output.contains("Syntax error"));
        assert!(output.contains("test.py:10:5"));
        assert!(output.contains("rule: syntax"));
    }

    #[test]
    fn test_diagnostic_builder() {
        let diag = DiagnosticBuilder::new()
            .file_path("test.py")
            .message("Test error")
            .severity(Severity::Error)
            .category("format/python")
            .line(10)
            .column(5)
            .rule("test-rule")
            .build();

        assert_eq!(diag.file_path, "test.py");
        assert_eq!(diag.message, "Test error");
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(diag.category, "format/python");
        assert_eq!(diag.line, Some(10));
        assert_eq!(diag.column, Some(5));
        assert_eq!(diag.rule, Some("test-rule".to_string()));
    }

    #[test]
    fn test_diagnostic_builder_try_build() {
        let complete = DiagnosticBuilder::new()
            .file_path("test.py")
            .message("Error")
            .severity(Severity::Error)
            .category("lint/python")
            .try_build();
        assert!(complete.is_some());

        let incomplete = DiagnosticBuilder::new().file_path("test.py").try_build();
        assert!(incomplete.is_none());
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // Generators for diagnostic components
    fn arb_severity() -> impl Strategy<Value = Severity> {
        prop_oneof![
            Just(Severity::Error),
            Just(Severity::Warning),
            Just(Severity::Info),
        ]
    }

    fn arb_file_path() -> impl Strategy<Value = String> {
        "[a-z]{1,10}\\.(py|pyi|c|cpp|cc|cxx|h|hpp|hxx|go|rs|php|kt|kts|md|markdown|toml)"
            .prop_map(String::from)
    }

    fn arb_message() -> impl Strategy<Value = String> {
        "[A-Za-z0-9 ]{1,100}".prop_map(String::from)
    }

    fn arb_operation() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("format".to_string()),
            Just("lint".to_string()),
            Just("io".to_string()),
        ]
    }

    fn arb_language() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("python".to_string()),
            Just("cpp".to_string()),
            Just("go".to_string()),
            Just("rust".to_string()),
            Just("php".to_string()),
            Just("kotlin".to_string()),
            Just("markdown".to_string()),
            Just("toml".to_string()),
        ]
    }

    fn arb_category() -> impl Strategy<Value = String> {
        (arb_operation(), arb_language()).prop_map(|(op, lang)| format!("{}/{}", op, lang))
    }

    fn arb_line() -> impl Strategy<Value = Option<usize>> {
        prop_oneof![Just(None), (1usize..10000).prop_map(Some),]
    }

    fn arb_column() -> impl Strategy<Value = Option<usize>> {
        prop_oneof![Just(None), (1usize..1000).prop_map(Some),]
    }

    fn arb_rule() -> impl Strategy<Value = Option<String>> {
        prop_oneof![Just(None), "[a-z][a-z0-9-]{0,20}".prop_map(|s| Some(s)),]
    }

    fn arb_diagnostic() -> impl Strategy<Value = Diagnostic> {
        (
            arb_file_path(),
            arb_message(),
            arb_line(),
            arb_column(),
            arb_severity(),
            arb_category(),
            arb_rule(),
        )
            .prop_map(|(file_path, message, line, column, severity, category, rule)| {
                Diagnostic {
                    file_path,
                    message,
                    line,
                    column,
                    severity,
                    category,
                    rule,
                }
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: multi-language-formatter-linter, Property 4: Diagnostic Structure Completeness**
        /// *For any* diagnostic produced by the system, it SHALL contain a non-empty file_path,
        /// a non-empty message, and a valid category string matching the pattern "{operation}/{language}".
        /// **Validates: Requirements 1.4, 2.4, 6.4, 9.4, 11.1, 11.2, 11.3, 11.4**
        #[test]
        fn prop_diagnostic_structure_completeness(diagnostic in arb_diagnostic()) {
            // All diagnostics must have non-empty file_path
            prop_assert!(
                !diagnostic.file_path.is_empty(),
                "File path must not be empty"
            );

            // All diagnostics must have non-empty message
            prop_assert!(
                !diagnostic.message.is_empty(),
                "Message must not be empty"
            );

            // All diagnostics must have non-empty category
            prop_assert!(
                !diagnostic.category.is_empty(),
                "Category must not be empty"
            );

            // Category must match pattern "{operation}/{language}"
            prop_assert!(
                diagnostic.has_valid_category(),
                "Category must match pattern '{{operation}}/{{language}}', got: {}",
                diagnostic.category
            );

            // Category must have exactly two parts separated by '/'
            let parts: Vec<&str> = diagnostic.category.split('/').collect();
            prop_assert_eq!(
                parts.len(),
                2,
                "Category must have exactly two parts separated by '/'"
            );

            // Both parts must be non-empty
            prop_assert!(
                !parts[0].is_empty() && !parts[1].is_empty(),
                "Both operation and language parts must be non-empty"
            );

            // is_valid() should return true for well-formed diagnostics
            prop_assert!(
                diagnostic.is_valid(),
                "Diagnostic should be valid"
            );

            // Severity must be one of the valid values
            let valid_severity = matches!(
                diagnostic.severity,
                Severity::Error | Severity::Warning | Severity::Info
            );
            prop_assert!(valid_severity, "Severity must be a valid value");

            // operation() and language() should return the correct parts
            prop_assert_eq!(
                diagnostic.operation(),
                Some(parts[0]),
                "operation() should return the first part of category"
            );
            prop_assert_eq!(
                diagnostic.language(),
                Some(parts[1]),
                "language() should return the second part of category"
            );
        }

        /// **Feature: multi-language-formatter-linter, Property 4 (continued): Display Format**
        /// *For any* diagnostic, the Display output SHALL contain the severity, category, message,
        /// and location information.
        /// **Validates: Requirements 11.1, 11.2, 11.3, 11.4**
        #[test]
        fn prop_diagnostic_display_contains_required_info(diagnostic in arb_diagnostic()) {
            let display_output = format!("{}", diagnostic);

            // Display must contain severity
            prop_assert!(
                display_output.contains(diagnostic.severity.as_str()),
                "Display output must contain severity"
            );

            // Display must contain category
            prop_assert!(
                display_output.contains(&diagnostic.category),
                "Display output must contain category"
            );

            // Display must contain message
            prop_assert!(
                display_output.contains(&diagnostic.message),
                "Display output must contain message"
            );

            // Display must contain file path
            prop_assert!(
                display_output.contains(&diagnostic.file_path),
                "Display output must contain file path"
            );

            // If rule is present, display must contain it
            if let Some(ref rule) = diagnostic.rule {
                prop_assert!(
                    display_output.contains(rule),
                    "Display output must contain rule when present"
                );
            }
        }

        /// **Feature: multi-language-formatter-linter, Property 4 (continued): Location String Format**
        /// *For any* diagnostic with location information, the location_string() SHALL correctly
        /// format the location as "file:line:column", "file:line", or "file".
        /// **Validates: Requirements 11.3**
        #[test]
        fn prop_diagnostic_location_string_format(diagnostic in arb_diagnostic()) {
            let location = diagnostic.location_string();

            // Location must always contain file path
            prop_assert!(
                location.starts_with(&diagnostic.file_path),
                "Location string must start with file path"
            );

            match (diagnostic.line, diagnostic.column) {
                (Some(line), Some(col)) => {
                    // Full location: file:line:column
                    let expected = format!("{}:{}:{}", diagnostic.file_path, line, col);
                    prop_assert_eq!(
                        location,
                        expected,
                        "Location with line and column must be 'file:line:column'"
                    );
                }
                (Some(line), None) => {
                    // Partial location: file:line
                    let expected = format!("{}:{}", diagnostic.file_path, line);
                    prop_assert_eq!(
                        location,
                        expected,
                        "Location with only line must be 'file:line'"
                    );
                }
                (None, _) => {
                    // File only
                    prop_assert_eq!(
                        location,
                        diagnostic.file_path,
                        "Location without line must be just file path"
                    );
                }
            }
        }
    }
}
