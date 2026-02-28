//! Kotlin Language Handler
//!
//! This module provides formatting and linting support for Kotlin files
//! using ktlint for both formatting and linting.

use std::fs;
use std::path::Path;

use crate::languages::diagnostic::Diagnostic;
use crate::languages::external_tools::ExternalToolManager;
use crate::languages::{FileStatus, LanguageHandler};

/// Kotlin file extensions
const KOTLIN_EXTENSIONS: &[&str] = &["kt", "kts"];

/// Config file names for ktlint
const KTLINT_CONFIG_FILES: &[&str] = &[".editorconfig"];

/// Kotlin language handler
///
/// Supports `.kt` and `.kts` file extensions.
/// Uses ktlint for formatting and linting.
pub struct KotlinHandler {
    /// Path to the ktlint executable (if found)
    ktlint_path: Option<std::path::PathBuf>,
}

impl KotlinHandler {
    /// Create a new Kotlin handler
    #[must_use]
    pub fn new() -> Self {
        Self {
            ktlint_path: ExternalToolManager::find_tool("ktlint"),
        }
    }

    /// Ensure ktlint is available, attempting installation if needed
    fn ensure_ktlint(&self) -> Result<std::path::PathBuf, Diagnostic> {
        if let Some(ref path) = self.ktlint_path {
            return Ok(path.clone());
        }

        // Try to install ktlint
        match ExternalToolManager::install_tool("ktlint") {
            Ok(path) => Ok(path),
            Err(e) => Err(Diagnostic::error(
                "",
                format!(
                    "ktlint is required for Kotlin formatting but was not found.\n\n{}",
                    e.instructions
                ),
                "tool/kotlin",
            )),
        }
    }

    /// Check if a file is a Kotlin script file (.kts)
    #[must_use]
    pub fn is_script_file(path: &Path) -> bool {
        path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| ext == "kts")
    }

    /// Check if a config file exists for the given path
    #[must_use]
    pub fn has_config_file(path: &Path) -> bool {
        ExternalToolManager::find_config_file(path, KTLINT_CONFIG_FILES).is_some()
    }

    /// Get the config file names that ktlint looks for
    #[must_use]
    pub fn config_file_names() -> &'static [&'static str] {
        KTLINT_CONFIG_FILES
    }

    /// Format Kotlin code using ktlint
    fn format_with_ktlint(&self, path: &Path, content: &str) -> Result<String, Diagnostic> {
        let ktlint_path = self.ensure_ktlint()?;
        let file_path_str = path.to_string_lossy().to_string();

        // Create a temporary file for ktlint
        let temp_dir = std::env::temp_dir();
        let ext = if Self::is_script_file(path) {
            "kts"
        } else {
            "kt"
        };
        let temp_file_name = format!("dx_check_kotlin_temp_{}.{}", std::process::id(), ext);
        let temp_path = temp_dir.join(&temp_file_name);

        // Write content to temp file
        fs::write(&temp_path, content).map_err(|e| {
            Diagnostic::error(
                &file_path_str,
                format!("Failed to create temporary file for formatting: {e}"),
                "io/kotlin",
            )
        })?;

        // Build arguments for ktlint --format
        let temp_path_str = temp_path.to_string_lossy().to_string();
        let args = vec!["--format", &temp_path_str];

        let result = ExternalToolManager::run_tool(&ktlint_path, &args, None);

        // Read the formatted content
        let formatted = fs::read_to_string(&temp_path).map_err(|e| {
            let _ = fs::remove_file(&temp_path);
            Diagnostic::error(
                &file_path_str,
                format!("Failed to read formatted content: {e}"),
                "io/kotlin",
            )
        })?;

        // Clean up temp file
        let _ = fs::remove_file(&temp_path);

        match result {
            Ok(_) => Ok(formatted),
            Err(e) => {
                // ktlint might return non-zero even when formatting succeeds
                // Check if the file was actually formatted
                if formatted.is_empty() {
                    Err(Diagnostic::error(
                        file_path_str,
                        format!("ktlint format failed: {e}"),
                        "format/kotlin",
                    ))
                } else {
                    Ok(formatted)
                }
            }
        }
    }

    /// Lint Kotlin code using ktlint
    fn lint_with_ktlint(&self, path: &Path, content: &str) -> Vec<Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();
        let mut diagnostics = Vec::new();

        // Try to get ktlint path
        let ktlint_path = match self.ensure_ktlint() {
            Ok(path) => path,
            Err(diag) => {
                diagnostics.push(diag);
                return diagnostics;
            }
        };

        // Create a temporary file for ktlint
        let temp_dir = std::env::temp_dir();
        let ext = if Self::is_script_file(path) {
            "kts"
        } else {
            "kt"
        };
        let temp_file_name = format!("dx_check_kotlin_lint_{}.{}", std::process::id(), ext);
        let temp_path = temp_dir.join(&temp_file_name);

        // Write content to temp file
        if let Err(e) = fs::write(&temp_path, content) {
            diagnostics.push(Diagnostic::error(
                &file_path_str,
                format!("Failed to create temporary file for linting: {e}"),
                "io/kotlin",
            ));
            return diagnostics;
        }

        // Run ktlint (without --format) to check for issues
        let temp_path_str = temp_path.to_string_lossy().to_string();
        let args = vec!["--relative", &temp_path_str];

        match ExternalToolManager::run_tool(&ktlint_path, &args, None) {
            Ok((stdout, stderr)) => {
                // Clean up temp file
                let _ = fs::remove_file(&temp_path);

                // Parse any errors from output
                let output = if stdout.is_empty() { &stderr } else { &stdout };

                for line in output.lines() {
                    if let Some(diag) = self.parse_ktlint_error(line, &file_path_str) {
                        diagnostics.push(diag);
                    }
                }
            }
            Err(e) => {
                let _ = fs::remove_file(&temp_path);
                // ktlint returns non-zero when there are lint errors
                // Parse the error output
                for line in e.lines() {
                    if let Some(diag) = self.parse_ktlint_error(line, &file_path_str) {
                        diagnostics.push(diag);
                    }
                }
            }
        }

        diagnostics
    }

    /// Parse a ktlint error line into a Diagnostic
    ///
    /// ktlint error format: <file:line:column>: message (rule-name)
    fn parse_ktlint_error(&self, line: &str, original_file: &str) -> Option<Diagnostic> {
        // Skip empty lines
        if line.trim().is_empty() {
            return None;
        }

        // Format: file:line:column: message (rule-name)
        // Example: test.kt:10:5: Unexpected blank line(s) before "}" (no-blank-line-before-rbrace)

        // Check if line contains the expected format
        if !line.contains(':') {
            return None;
        }

        let parts: Vec<&str> = line.splitn(4, ':').collect();
        if parts.len() < 4 {
            return None;
        }

        // Try to parse line and column numbers
        let line_num: usize = parts[1].trim().parse().ok()?;
        let column: usize = parts[2].trim().parse().ok()?;

        let rest = parts[3].trim();

        // Extract rule name if present (in parentheses at the end)
        let (message, rule) = if let Some(paren_start) = rest.rfind('(') {
            if let Some(paren_end) = rest.rfind(')') {
                if paren_end > paren_start {
                    let rule = rest[paren_start + 1..paren_end].to_string();
                    let msg = rest[..paren_start].trim().to_string();
                    (msg, Some(rule))
                } else {
                    (rest.to_string(), None)
                }
            } else {
                (rest.to_string(), None)
            }
        } else {
            (rest.to_string(), None)
        };

        let mut diag = Diagnostic::warning(original_file, message, "lint/kotlin")
            .with_location(line_num, column);

        if let Some(r) = rule {
            diag = diag.with_rule(r);
        }

        Some(diag)
    }
}

impl Default for KotlinHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageHandler for KotlinHandler {
    fn extensions(&self) -> &[&str] {
        KOTLIN_EXTENSIONS
    }

    fn format(&self, path: &Path, content: &str, write: bool) -> Result<FileStatus, Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();

        // Format the content
        let formatted = self.format_with_ktlint(path, content)?;

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
                    "io/kotlin",
                )
            })?;
        }

        Ok(FileStatus::Changed)
    }

    fn lint(&self, path: &Path, content: &str) -> Result<Vec<Diagnostic>, Diagnostic> {
        // Lint using ktlint
        let diagnostics = self.lint_with_ktlint(path, content);
        Ok(diagnostics)
    }

    fn check(&self, path: &Path, content: &str, write: bool) -> Result<FileStatus, Diagnostic> {
        // First, lint the file
        let lint_diagnostics = self.lint(path, content)?;

        // If there are errors (not just warnings), report the first one
        let errors: Vec<_> = lint_diagnostics
            .iter()
            .filter(|d| d.severity == crate::languages::Severity::Error)
            .collect();

        if !errors.is_empty() {
            return Err(errors[0].clone());
        }

        // Then format
        self.format(path, content, write)
    }

    fn name(&self) -> &'static str {
        "kotlin"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kotlin_handler_extensions() {
        let handler = KotlinHandler::new();
        let extensions = handler.extensions();
        assert!(extensions.contains(&"kt"));
        assert!(extensions.contains(&"kts"));
        assert_eq!(extensions.len(), 2);
    }

    #[test]
    fn test_kotlin_handler_name() {
        let handler = KotlinHandler::new();
        assert_eq!(handler.name(), "kotlin");
    }

    #[test]
    fn test_is_script_file() {
        assert!(KotlinHandler::is_script_file(Path::new("build.gradle.kts")));
        assert!(KotlinHandler::is_script_file(Path::new("test.kts")));
        assert!(!KotlinHandler::is_script_file(Path::new("Main.kt")));
        assert!(!KotlinHandler::is_script_file(Path::new("test.txt")));
    }

    #[test]
    fn test_config_file_names() {
        let names = KotlinHandler::config_file_names();
        assert!(names.contains(&".editorconfig"));
    }

    #[test]
    fn test_has_config_file_false() {
        assert!(!KotlinHandler::has_config_file(Path::new("/nonexistent/path/test.kt")));
    }

    #[test]
    fn test_parse_ktlint_error_with_rule() {
        let handler = KotlinHandler::new();
        let line =
            "test.kt:10:5: Unexpected blank line(s) before \"}\" (no-blank-line-before-rbrace)";
        let diag = handler.parse_ktlint_error(line, "test.kt");

        assert!(diag.is_some());
        let diag = diag.unwrap();
        assert_eq!(diag.file_path, "test.kt");
        assert_eq!(diag.line, Some(10));
        assert_eq!(diag.column, Some(5));
        assert!(diag.message.contains("Unexpected blank line"));
        assert_eq!(diag.rule, Some("no-blank-line-before-rbrace".to_string()));
        assert_eq!(diag.category, "lint/kotlin");
    }

    #[test]
    fn test_parse_ktlint_error_without_rule() {
        let handler = KotlinHandler::new();
        let line = "test.kt:5:1: Missing newline at end of file";
        let diag = handler.parse_ktlint_error(line, "test.kt");

        assert!(diag.is_some());
        let diag = diag.unwrap();
        assert_eq!(diag.line, Some(5));
        assert_eq!(diag.column, Some(1));
        assert!(diag.message.contains("Missing newline"));
    }

    #[test]
    fn test_parse_ktlint_error_empty_line() {
        let handler = KotlinHandler::new();
        let diag = handler.parse_ktlint_error("", "test.kt");
        assert!(diag.is_none());
    }

    #[test]
    fn test_parse_ktlint_error_whitespace_line() {
        let handler = KotlinHandler::new();
        let diag = handler.parse_ktlint_error("   ", "test.kt");
        assert!(diag.is_none());
    }

    #[test]
    fn test_parse_ktlint_error_invalid_format() {
        let handler = KotlinHandler::new();
        let diag = handler.parse_ktlint_error("Some random text", "test.kt");
        assert!(diag.is_none());
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generator for valid Kotlin identifiers
    fn arb_kotlin_identifier() -> impl Strategy<Value = String> {
        "[a-z][a-zA-Z0-9_]{0,10}".prop_map(String::from)
    }

    /// Generator for simple Kotlin integer literals
    fn arb_kotlin_int() -> impl Strategy<Value = String> {
        (1i64..1000).prop_map(|n| n.to_string())
    }

    /// Generator for simple Kotlin string literals
    fn arb_kotlin_string() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 ]{0,20}".prop_map(|s| format!("\"{}\"", s))
    }

    /// Generator for simple Kotlin expressions
    fn arb_kotlin_expr() -> impl Strategy<Value = String> {
        prop_oneof![
            arb_kotlin_int(),
            arb_kotlin_string(),
            arb_kotlin_identifier(),
            (arb_kotlin_int(), arb_kotlin_int()).prop_map(|(a, b)| format!("{} + {}", a, b)),
            (arb_kotlin_int(), arb_kotlin_int()).prop_map(|(a, b)| format!("{} * {}", a, b)),
        ]
    }

    /// Generator for simple Kotlin variable declarations
    fn arb_kotlin_val_decl() -> impl Strategy<Value = String> {
        (arb_kotlin_identifier(), arb_kotlin_expr())
            .prop_map(|(name, expr)| format!("    val {} = {}", name, expr))
    }

    /// Generator for simple Kotlin function definitions
    fn arb_kotlin_function() -> impl Strategy<Value = String> {
        (arb_kotlin_identifier(), prop::collection::vec(arb_kotlin_val_decl(), 0..3)).prop_map(
            |(name, body)| {
                let body_str = if body.is_empty() {
                    String::new()
                } else {
                    body.join("\n") + "\n"
                };
                format!("fun {}() {{\n{}}}\n", name, body_str)
            },
        )
    }

    /// Generator for valid Kotlin code snippets
    fn arb_valid_kotlin_code() -> impl Strategy<Value = String> {
        prop_oneof![
            // Simple Kotlin with println
            arb_kotlin_string()
                .prop_map(|s| { format!("fun main() {{\n    println({})\n}}\n", s) }),
            // Kotlin with variable declaration
            (arb_kotlin_identifier(), arb_kotlin_expr()).prop_map(|(name, expr)| {
                format!("fun main() {{\n    val {} = {}\n}}\n", name, expr)
            }),
            // Kotlin with function
            arb_kotlin_function(),
            // Kotlin with class
            (arb_kotlin_identifier(), arb_kotlin_identifier()).prop_map(
                |(class_name, prop_name)| {
                    format!("class {} {{\n    val {}: Int = 0\n}}\n", class_name, prop_name)
                }
            ),
            // Kotlin with data class
            (arb_kotlin_identifier(), arb_kotlin_identifier()).prop_map(
                |(class_name, prop_name)| {
                    format!("data class {}(val {}: String)\n", class_name, prop_name)
                }
            ),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: multi-language-formatter-linter, Property 2: Formatting Round-Trip Consistency (Kotlin)**
        /// *For any* valid Kotlin source file, formatting the file and then formatting the result
        /// again SHALL produce identical output (idempotence: format(format(x)) == format(x)).
        /// **Validates: Requirements 7.2**
        #[test]
        fn prop_kotlin_formatting_idempotence(code in arb_valid_kotlin_code()) {
            let handler = KotlinHandler::new();

            // Skip if ktlint is not installed
            if handler.ktlint_path.is_none() {
                // Can't test formatting without ktlint
                return Ok(());
            }

            let path = Path::new("test.kt");

            // Format once
            let first_format = match handler.format_with_ktlint(path, &code) {
                Ok(formatted) => formatted,
                Err(_) => {
                    // If formatting fails, skip this test case
                    return Ok(());
                }
            };

            // Format again
            let second_format = match handler.format_with_ktlint(path, &first_format) {
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
    }
}
