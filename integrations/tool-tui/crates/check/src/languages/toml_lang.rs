//! TOML Language Handler
//!
//! This module provides formatting and linting support for TOML files
//! using the taplo crate for high-performance TOML formatting and linting.

use std::fs;
use std::path::Path;

use crate::languages::diagnostic::Diagnostic;
use crate::languages::{FileStatus, LanguageHandler};

/// TOML file extension
const TOML_EXTENSION: &str = "toml";

/// TOML language handler using taplo
///
/// Supports `.toml` file extension.
/// Uses taplo for high-performance TOML formatting and linting.
pub struct TomlHandler;

impl TomlHandler {
    /// Create a new TOML handler
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Format TOML content using taplo
    fn format_toml(&self, path: &Path, content: &str) -> Result<String, Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();

        // Parse the TOML content using taplo
        let parse_result = taplo::parser::parse(content);

        // Check for syntax errors
        if !parse_result.errors.is_empty() {
            let first_error = &parse_result.errors[0];
            let range = first_error.range;

            // Convert byte offset to line/column
            let (line, col) = Self::offset_to_line_col(content, range.start().into());

            let mut diag = Diagnostic::error(
                &file_path_str,
                format!("TOML parse error: {first_error}"),
                "format/toml",
            );
            if let Some(l) = line {
                diag = diag.with_line(l);
            }
            if let Some(c) = col {
                diag = diag.with_column(c);
            }
            return Err(diag);
        }

        // Format using taplo with default options
        let options = taplo::formatter::Options::default();
        let formatted = taplo::formatter::format(content, options);

        Ok(formatted)
    }

    /// Lint TOML content using taplo
    fn lint_toml(&self, path: &Path, content: &str) -> Vec<Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();
        let mut diagnostics = Vec::new();

        // Parse the TOML content
        let parse_result = taplo::parser::parse(content);

        // Convert parse errors to diagnostics
        for error in &parse_result.errors {
            let range = error.range;
            let (line, col) = Self::offset_to_line_col(content, range.start().into());

            let mut diag =
                Diagnostic::error(&file_path_str, format!("Syntax error: {error}"), "lint/toml")
                    .with_rule("taplo/syntax-error");

            if let Some(l) = line {
                diag = diag.with_line(l);
            }
            if let Some(c) = col {
                diag = diag.with_column(c);
            }

            diagnostics.push(diag);
        }

        diagnostics
    }

    /// Convert byte offset to line and column numbers
    fn offset_to_line_col(content: &str, offset: usize) -> (Option<usize>, Option<usize>) {
        let mut line = 1;
        let mut col = 1;

        for (i, ch) in content.chars().enumerate() {
            if i >= offset {
                break;
            }

            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }

        (Some(line), Some(col))
    }
}

impl Default for TomlHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageHandler for TomlHandler {
    fn extensions(&self) -> &[&str] {
        &[TOML_EXTENSION]
    }

    fn format(&self, path: &Path, content: &str, write: bool) -> Result<FileStatus, Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();

        // Format the content
        let formatted = self.format_toml(path, content)?;

        // Check if content changed
        if formatted == content {
            return Ok(FileStatus::Unchanged);
        }

        // Write if requested
        if write {
            fs::write(path, &formatted).map_err(|e| {
                Diagnostic::error(
                    &file_path_str,
                    format!("Failed to write formatted content: {e}"),
                    "io/toml",
                )
            })?;
        }

        Ok(FileStatus::Changed)
    }

    fn lint(&self, path: &Path, content: &str) -> Result<Vec<Diagnostic>, Diagnostic> {
        let diagnostics = self.lint_toml(path, content);
        Ok(diagnostics)
    }

    fn check(&self, path: &Path, content: &str, write: bool) -> Result<FileStatus, Diagnostic> {
        // First, lint the file
        let lint_diagnostics = self.lint(path, content)?;

        // If there are syntax errors, report them
        if !lint_diagnostics.is_empty() {
            // Return the first error as the main error
            return Err(lint_diagnostics.into_iter().next().unwrap());
        }

        // Then format
        self.format(path, content, write)
    }

    fn name(&self) -> &'static str {
        "toml"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toml_handler_extensions() {
        let handler = TomlHandler::new();
        let extensions = handler.extensions();
        assert!(extensions.contains(&"toml"));
        assert_eq!(extensions.len(), 1);
    }

    #[test]
    fn test_toml_handler_name() {
        let handler = TomlHandler::new();
        assert_eq!(handler.name(), "toml");
    }

    #[test]
    fn test_format_valid_toml() {
        let handler = TomlHandler::new();
        let content = r#"[package]
name="test"
version="1.0.0"
"#;
        let result = handler.format_toml(Path::new("test.toml"), content);
        assert!(result.is_ok());
        let formatted = result.unwrap();
        // Should be properly formatted by taplo
        assert!(formatted.contains("[package]"));
        assert!(formatted.contains("name"));
    }

    #[test]
    fn test_format_invalid_toml() {
        let handler = TomlHandler::new();
        let content = r#"[package
name = "test"
"#;
        let result = handler.format_toml(Path::new("test.toml"), content);
        assert!(result.is_err());
    }

    #[test]
    fn test_lint_valid_toml() {
        let handler = TomlHandler::new();
        let content = r#"[package]
name = "test"
version = "1.0.0"

[dependencies]
serde = "1.0"
"#;
        let diagnostics = handler.lint_toml(Path::new("test.toml"), content);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_lint_invalid_toml_unclosed_bracket() {
        let handler = TomlHandler::new();
        let content = r#"[package
name = "test"
"#;
        let diagnostics = handler.lint_toml(Path::new("test.toml"), content);
        assert!(!diagnostics.is_empty());
        let diag = &diagnostics[0];
        assert_eq!(diag.category, "lint/toml");
    }

    #[test]
    fn test_lint_duplicate_keys() {
        let handler = TomlHandler::new();
        let content = r#"[package]
name = "test"
name = "test2"
"#;
        let diagnostics = handler.lint_toml(Path::new("test.toml"), content);
        assert!(!diagnostics.is_empty());
        // Should detect duplicate key
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("Duplicate") || d.message.contains("duplicate"))
        );
    }

    #[test]
    fn test_lint_invalid_toml_bad_value() {
        let handler = TomlHandler::new();
        let content = r#"[package]
name = 
"#;
        let diagnostics = handler.lint_toml(Path::new("test.toml"), content);
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn test_offset_to_line_col() {
        let content = "line1\nline2\nline3";

        // Start of file
        let (line, col) = TomlHandler::offset_to_line_col(content, 0);
        assert_eq!(line, Some(1));
        assert_eq!(col, Some(1));

        // Start of second line
        let (line, col) = TomlHandler::offset_to_line_col(content, 6);
        assert_eq!(line, Some(2));
        assert_eq!(col, Some(1));

        // Middle of second line
        let (line, col) = TomlHandler::offset_to_line_col(content, 8);
        assert_eq!(line, Some(2));
        assert_eq!(col, Some(3));
    }

    #[test]
    fn test_lint_returns_diagnostics() {
        let handler = TomlHandler::new();
        let content = "[invalid";
        let result = handler.lint(Path::new("test.toml"), content);
        assert!(result.is_ok());
        let diagnostics = result.unwrap();
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn test_check_valid_toml() {
        let handler = TomlHandler::new();
        let content = "[package]\nname = \"test\"\n";
        let result = handler.check(Path::new("test.toml"), content, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_invalid_toml() {
        let handler = TomlHandler::new();
        let content = "[invalid";
        let result = handler.check(Path::new("test.toml"), content, false);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generator for valid TOML keys (simple identifiers)
    fn arb_toml_key() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{0,10}".prop_map(String::from)
    }

    /// Generator for simple TOML string values
    fn arb_toml_string_value() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 ]{0,20}".prop_map(|s| format!("\"{}\"", s))
    }

    /// Generator for simple TOML integer values
    fn arb_toml_int_value() -> impl Strategy<Value = String> {
        (1i64..1000).prop_map(|n| n.to_string())
    }

    /// Generator for simple TOML boolean values
    fn arb_toml_bool_value() -> impl Strategy<Value = String> {
        prop_oneof![Just("true".to_string()), Just("false".to_string()),]
    }

    /// Generator for simple TOML values
    fn arb_toml_value() -> impl Strategy<Value = String> {
        prop_oneof![
            arb_toml_string_value(),
            arb_toml_int_value(),
            arb_toml_bool_value(),
        ]
    }

    /// Generator for simple TOML key-value pairs
    fn arb_toml_kv() -> impl Strategy<Value = String> {
        (arb_toml_key(), arb_toml_value()).prop_map(|(k, v)| format!("{} = {}", k, v))
    }

    /// Generator for valid TOML content
    fn arb_valid_toml() -> impl Strategy<Value = String> {
        prop_oneof![
            // Simple key-value
            arb_toml_kv().prop_map(|kv| format!("{}\n", kv)),
            // Table with unique key-values (use index to ensure uniqueness)
            (arb_toml_key(), prop::collection::vec(arb_toml_value(), 1..5)).prop_map(
                |(table, values)| {
                    let kvs: Vec<String> = values
                        .iter()
                        .enumerate()
                        .map(|(i, v)| format!("key{} = {}", i, v))
                        .collect();
                    format!("[{}]\n{}\n", table, kvs.join("\n"))
                }
            ),
            // Multiple tables with unique names
            (arb_toml_key(), arb_toml_kv()).prop_map(|(t1, kv1)| {
                format!("[{}]\n{}\n\n[other_table]\nvalue = 42\n", t1, kv1)
            }),
            // Nested table
            (arb_toml_key(), arb_toml_key(), arb_toml_kv()).prop_filter_map(
                "tables must be different",
                |(t1, t2, kv)| {
                    if t1 != t2 {
                        Some(format!("[{}.{}]\n{}\n", t1, t2, kv))
                    } else {
                        None
                    }
                }
            ),
        ]
    }

    /// Generator for invalid TOML content
    fn arb_invalid_toml() -> impl Strategy<Value = String> {
        prop_oneof![
            // Unclosed bracket
            arb_toml_key().prop_map(|k| format!("[{}\n", k)),
            // Missing value
            arb_toml_key().prop_map(|k| format!("{} = \n", k)),
            // Unclosed string
            arb_toml_key().prop_map(|k| format!("{} = \"unclosed\n", k)),
            // Double equals
            arb_toml_key().prop_map(|k| format!("{} == \"value\"\n", k)),
            // Invalid table header (empty)
            Just("[]\nkey = \"value\"\n".to_string()),
            // Unclosed array
            arb_toml_key().prop_map(|k| format!("{} = [1, 2, 3\n", k)),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: multi-language-formatter-linter, Property 2: Formatting Round-Trip Consistency (TOML)**
        /// *For any* valid TOML source file, formatting the file and then formatting the result
        /// again SHALL produce identical output (idempotence: format(format(x)) == format(x)).
        /// **Validates: Requirements 1.1, 1.2**
        #[test]
        fn prop_toml_formatting_idempotence(content in arb_valid_toml()) {
            let handler = TomlHandler::new();
            let path = Path::new("test.toml");

            // First, verify the content is valid TOML
            let diagnostics = handler.lint_toml(path, &content);
            if !diagnostics.is_empty() {
                // Skip invalid content
                return Ok(());
            }

            // Format once
            let first_format = match handler.format_toml(path, &content) {
                Ok(formatted) => formatted,
                Err(_) => {
                    // If formatting fails, skip this test case
                    return Ok(());
                }
            };

            // Format again
            let second_format = match handler.format_toml(path, &first_format) {
                Ok(formatted) => formatted,
                Err(_) => {
                    // If formatting fails, skip this test case
                    return Ok(());
                }
            };

            // Idempotence: format(format(x)) == format(x)
            prop_assert_eq!(
                first_format,
                second_format,
                "Formatting should be idempotent: format(format(x)) == format(x)"
            );
        }

        /// **Feature: multi-language-formatter-linter, Property 3: Syntax Validation Correctness (TOML)**
        /// *For any* source file, if the file contains valid syntax for TOML, linting SHALL NOT
        /// report syntax errors. If the file contains invalid syntax, linting SHALL report at
        /// least one syntax error.
        /// **Validates: Requirements 1.1, 1.2**
        #[test]
        fn prop_toml_syntax_validation_valid(content in arb_valid_toml()) {
            let handler = TomlHandler::new();
            let path = Path::new("test.toml");

            // Valid TOML content should produce no syntax errors
            let diagnostics = handler.lint_toml(path, &content);

            prop_assert!(
                diagnostics.is_empty(),
                "Valid TOML content should not produce syntax errors, but got: {:?}",
                diagnostics
            );
        }

        /// **Feature: multi-language-formatter-linter, Property 3: Syntax Validation Correctness (TOML)**
        /// *For any* source file with invalid syntax, linting SHALL report at least one syntax error.
        /// **Validates: Requirements 1.1, 1.2**
        #[test]
        fn prop_toml_syntax_validation_invalid(content in arb_invalid_toml()) {
            let handler = TomlHandler::new();
            let path = Path::new("test.toml");

            // Invalid TOML content should produce at least one syntax error
            let diagnostics = handler.lint_toml(path, &content);

            prop_assert!(
                !diagnostics.is_empty(),
                "Invalid TOML content should produce syntax errors, but got none for: {:?}",
                content
            );

            // All diagnostics should have the correct category
            for diag in &diagnostics {
                prop_assert_eq!(
                    &diag.category,
                    "lint/toml",
                    "Syntax error diagnostics should have category 'lint/toml'"
                );
            }
        }
    }
}
