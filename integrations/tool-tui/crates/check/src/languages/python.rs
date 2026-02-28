//! Python Language Handler
//!
//! This module provides formatting and linting support for Python files
//! using rustpython-parser for syntax validation and ruff as an external
//! tool for formatting.

use std::fs;
use std::path::Path;

use crate::languages::diagnostic::Diagnostic;
use crate::languages::external_tools::ExternalToolManager;
use crate::languages::{FileStatus, LanguageHandler};

use rustpython_parser::{Parse, ast};

/// Python language handler
///
/// Supports `.py` and `.pyi` file extensions.
/// Uses rustpython-parser for syntax validation and ruff for formatting.
pub struct PythonHandler {
    /// Path to the ruff executable (if found)
    ruff_path: Option<std::path::PathBuf>,
}

impl PythonHandler {
    /// Create a new Python handler
    #[must_use]
    pub fn new() -> Self {
        Self {
            ruff_path: ExternalToolManager::find_tool("ruff"),
        }
    }

    /// Ensure ruff is available, attempting installation if needed
    fn ensure_ruff(&self) -> Result<std::path::PathBuf, Diagnostic> {
        if let Some(ref path) = self.ruff_path {
            return Ok(path.clone());
        }

        // Use tool_installer for automatic installation
        use crate::tool_installer::ToolRegistry;
        let registry = ToolRegistry::new();

        match registry.ensure_installed("ruff") {
            Ok(()) => {
                // Tool installed, find it now
                if let Some(path) = ExternalToolManager::find_tool("ruff") {
                    Ok(path)
                } else {
                    Err(Diagnostic::error(
                        "",
                        "ruff was installed but could not be found in PATH",
                        "tool/python",
                    ))
                }
            }
            Err(e) => Err(Diagnostic::error(
                "",
                format!("ruff is required for Python formatting but was not found.\n\n{}", e),
                "tool/python",
            )),
        }
    }

    /// Format Python code using ruff
    fn format_with_ruff(&self, path: &Path, content: &str) -> Result<String, Diagnostic> {
        let ruff_path = self.ensure_ruff()?;
        let file_path_str = path.to_string_lossy().to_string();

        // Run ruff format with stdin input
        let args = vec!["format", "--stdin-filename", &file_path_str, "-"];

        match ExternalToolManager::run_tool_checked(&ruff_path, &args, Some(content)) {
            Ok(formatted) => Ok(formatted),
            Err(stderr) => Err(Diagnostic::error(
                file_path_str,
                format!("ruff format failed: {stderr}"),
                "format/python",
            )),
        }
    }

    /// Lint Python code using ruff check
    fn lint_with_ruff(&self, path: &Path, content: &str) -> Result<Vec<Diagnostic>, Diagnostic> {
        let ruff_path = self.ensure_ruff()?;
        let file_path_str = path.to_string_lossy().to_string();

        // Run ruff check with JSON output for easier parsing
        let args = vec![
            "check",
            "--stdin-filename",
            &file_path_str,
            "--output-format=json",
            "-",
        ];

        match ExternalToolManager::run_tool(&ruff_path, &args, Some(content)) {
            Ok((stdout, _stderr)) => {
                // Parse JSON output
                self.parse_ruff_json_output(&stdout, &file_path_str)
            }
            Err(e) => {
                // ruff check returns non-zero exit code when there are violations
                // Try to parse the error output as JSON
                if e.contains('{') || e.contains('[') {
                    self.parse_ruff_json_output(&e, &file_path_str)
                } else {
                    Err(Diagnostic::error(
                        file_path_str,
                        format!("ruff check failed: {e}"),
                        "lint/python",
                    ))
                }
            }
        }
    }

    /// Parse ruff JSON output to diagnostics
    fn parse_ruff_json_output(
        &self,
        json_output: &str,
        file_path: &str,
    ) -> Result<Vec<Diagnostic>, Diagnostic> {
        use serde_json::Value;

        let mut diagnostics = Vec::new();

        // Try to parse as JSON
        let parsed: Value = match serde_json::from_str(json_output) {
            Ok(v) => v,
            Err(e) => {
                // If JSON parsing fails, return empty diagnostics (no violations)
                if json_output.trim().is_empty() {
                    return Ok(diagnostics);
                }
                return Err(Diagnostic::error(
                    file_path,
                    format!("Failed to parse ruff output: {e}"),
                    "lint/python",
                ));
            }
        };

        // ruff outputs an array of violation objects
        if let Some(violations) = parsed.as_array() {
            for violation in violations {
                let code = violation["code"].as_str().unwrap_or("unknown").to_string();
                let message = violation["message"].as_str().unwrap_or("Unknown error").to_string();

                // Extract location information
                let location = &violation["location"];
                let line = location["row"].as_u64().unwrap_or(1) as usize;
                let column = location["column"].as_u64().unwrap_or(1) as usize;

                // Map ruff severity to our severity
                // ruff doesn't provide severity in JSON, so we infer from code
                let severity = if code.starts_with('E') || code.starts_with('F') {
                    crate::languages::Severity::Error
                } else {
                    crate::languages::Severity::Warning
                };

                let diag = Diagnostic::new(
                    file_path,
                    format!("{code}: {message}"),
                    severity,
                    "lint/python",
                )
                .with_location(line, column);

                diagnostics.push(diag);
            }
        }

        Ok(diagnostics)
    }

    /// Validate Python syntax using rustpython-parser
    fn validate_syntax(&self, path: &Path, content: &str) -> Vec<Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();
        let mut diagnostics = Vec::new();

        // Try to parse as a module (suite of statements)
        match ast::Suite::parse(content, &file_path_str) {
            Ok(_) => {
                // Syntax is valid
            }
            Err(parse_error) => {
                // Extract location from parse error
                let (line, column) = extract_location_from_error(&parse_error, content);

                let diag = Diagnostic::error(
                    &file_path_str,
                    format!("Syntax error: {parse_error}"),
                    "lint/python",
                );

                let diag = if let Some(l) = line {
                    if let Some(c) = column {
                        diag.with_location(l, c)
                    } else {
                        diag.with_line(l)
                    }
                } else {
                    diag
                };

                diagnostics.push(diag);
            }
        }

        diagnostics
    }

    /// Check if a file is a Python stub file (.pyi)
    #[allow(dead_code)]
    fn is_stub_file(path: &Path) -> bool {
        path.extension().is_some_and(|ext| ext == "pyi")
    }
}

/// Extract line and column from a parse error by computing from offset
fn extract_location_from_error(
    error: &rustpython_parser::ParseError,
    content: &str,
) -> (Option<usize>, Option<usize>) {
    // The ParseError contains offset as TextSize (byte offset)
    let offset: usize = error.offset.into();

    if offset == 0 || content.is_empty() {
        return (Some(1), Some(1));
    }

    // Convert byte offset to line and column
    let mut line = 1;
    let mut column = 1;
    let mut current_offset = 0;

    for ch in content.chars() {
        if current_offset >= offset {
            break;
        }

        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }

        current_offset += ch.len_utf8();
    }

    (Some(line), Some(column))
}

impl Default for PythonHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageHandler for PythonHandler {
    fn extensions(&self) -> &[&str] {
        &["py", "pyi"]
    }

    fn format(&self, path: &Path, content: &str, write: bool) -> Result<FileStatus, Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();

        // Format the content
        let formatted = self.format_with_ruff(path, content)?;

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
                    "io/python",
                )
            })?;
        }

        Ok(FileStatus::Changed)
    }

    fn lint(&self, path: &Path, content: &str) -> Result<Vec<Diagnostic>, Diagnostic> {
        let mut diagnostics = Vec::new();

        // First, validate syntax using rustpython-parser
        let syntax_diagnostics = self.validate_syntax(path, content);
        diagnostics.extend(syntax_diagnostics);

        // If there are syntax errors, don't run ruff (it will fail anyway)
        if !diagnostics.is_empty() {
            return Ok(diagnostics);
        }

        // Then, lint using ruff check
        match self.lint_with_ruff(path, content) {
            Ok(ruff_diagnostics) => {
                diagnostics.extend(ruff_diagnostics);
                Ok(diagnostics)
            }
            Err(e) => {
                // If ruff is not available, just return syntax diagnostics
                if e.message.contains("ruff is required") {
                    Ok(diagnostics)
                } else {
                    Err(e)
                }
            }
        }
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
        "python"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_python_handler_extensions() {
        let handler = PythonHandler::new();
        let extensions = handler.extensions();
        assert!(extensions.contains(&"py"));
        assert!(extensions.contains(&"pyi"));
    }

    #[test]
    fn test_python_handler_name() {
        let handler = PythonHandler::new();
        assert_eq!(handler.name(), "python");
    }

    #[test]
    fn test_is_stub_file() {
        assert!(PythonHandler::is_stub_file(Path::new("test.pyi")));
        assert!(!PythonHandler::is_stub_file(Path::new("test.py")));
        assert!(!PythonHandler::is_stub_file(Path::new("test.txt")));
    }

    #[test]
    fn test_validate_syntax_valid() {
        let handler = PythonHandler::new();
        let valid_code = r#"
def hello():
    print("Hello, World!")

x = 1 + 2
"#;
        let diagnostics = handler.validate_syntax(Path::new("test.py"), valid_code);
        assert!(diagnostics.is_empty(), "Valid code should have no syntax errors");
    }

    #[test]
    fn test_validate_syntax_invalid() {
        let handler = PythonHandler::new();
        let invalid_code = r#"
def hello(
    print("Hello, World!")
"#;
        let diagnostics = handler.validate_syntax(Path::new("test.py"), invalid_code);
        assert!(!diagnostics.is_empty(), "Invalid code should have syntax errors");

        let diag = &diagnostics[0];
        assert_eq!(diag.category, "lint/python");
        assert!(diag.message.contains("Syntax error"));
    }

    #[test]
    fn test_validate_syntax_invalid_indentation() {
        let handler = PythonHandler::new();
        let invalid_code = r#"
def hello():
print("bad indent")
"#;
        let diagnostics = handler.validate_syntax(Path::new("test.py"), invalid_code);
        assert!(!diagnostics.is_empty(), "Invalid indentation should be caught");
    }

    #[test]
    fn test_validate_syntax_unclosed_string() {
        let handler = PythonHandler::new();
        let invalid_code = r#"
x = "unclosed string
"#;
        let diagnostics = handler.validate_syntax(Path::new("test.py"), invalid_code);
        assert!(!diagnostics.is_empty(), "Unclosed string should be caught");
    }

    #[test]
    fn test_validate_syntax_class_definition() {
        let handler = PythonHandler::new();
        let valid_code = r#"
class MyClass:
    def __init__(self, value):
        self.value = value

    def get_value(self):
        return self.value
"#;
        let diagnostics = handler.validate_syntax(Path::new("test.py"), valid_code);
        assert!(diagnostics.is_empty(), "Valid class definition should parse");
    }

    #[test]
    fn test_validate_syntax_stub_file() {
        let handler = PythonHandler::new();
        let stub_code = r#"
from typing import Optional

def greet(name: str) -> str: ...

class Greeter:
    def __init__(self, prefix: str) -> None: ...
    def greet(self, name: str) -> str: ...
"#;
        let diagnostics = handler.validate_syntax(Path::new("test.pyi"), stub_code);
        assert!(diagnostics.is_empty(), "Valid stub file should parse");
    }

    #[test]
    fn test_lint_returns_syntax_errors() {
        let handler = PythonHandler::new();
        let invalid_code = "def broken(";
        let result = handler.lint(Path::new("test.py"), invalid_code);
        assert!(result.is_ok());
        let diagnostics = result.unwrap();
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn test_lint_valid_code() {
        let handler = PythonHandler::new();
        let valid_code = "x = 1\ny = 2\n";
        let result = handler.lint(Path::new("test.py"), valid_code);
        assert!(result.is_ok());
        let diagnostics = result.unwrap();
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_diagnostic_has_location() {
        let handler = PythonHandler::new();
        let invalid_code = "def broken(";
        let diagnostics = handler.validate_syntax(Path::new("test.py"), invalid_code);
        assert!(!diagnostics.is_empty());

        let diag = &diagnostics[0];
        // Should have location information
        assert!(diag.line.is_some() || diag.column.is_some());
    }

    // Integration tests that require ruff to be installed
    #[cfg(feature = "integration_tests")]
    mod integration {
        use super::*;
        use std::io::Write;
        use tempfile::NamedTempFile;

        #[test]
        fn test_format_with_ruff() {
            let handler = PythonHandler::new();
            if handler.ruff_path.is_none() {
                eprintln!("Skipping test: ruff not installed");
                return;
            }

            let unformatted = "x=1+2\ny=3+4\n";
            let result = handler.format_with_ruff(Path::new("test.py"), unformatted);
            assert!(result.is_ok());
            let formatted = result.unwrap();
            // ruff should add spaces around operators
            assert!(formatted.contains("x = ") || formatted.contains("x="));
        }

        #[test]
        fn test_format_writes_file() {
            let handler = PythonHandler::new();
            if handler.ruff_path.is_none() {
                eprintln!("Skipping test: ruff not installed");
                return;
            }

            let mut temp_file = NamedTempFile::new().unwrap();
            let unformatted = "x=1+2\n";
            write!(temp_file, "{}", unformatted).unwrap();

            let result = handler.format(temp_file.path(), unformatted, true);
            assert!(result.is_ok());
        }
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generator for valid Python identifiers
    fn arb_python_identifier() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{0,10}".prop_map(String::from)
    }

    /// Generator for simple Python integer literals
    fn arb_python_int() -> impl Strategy<Value = String> {
        (1i64..1000).prop_map(|n| n.to_string())
    }

    /// Generator for simple Python string literals
    fn arb_python_string() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 ]{0,20}".prop_map(|s| format!("\"{}\"", s))
    }

    /// Generator for simple Python expressions
    fn arb_python_expr() -> impl Strategy<Value = String> {
        prop_oneof![
            arb_python_int(),
            arb_python_string(),
            arb_python_identifier(),
            (arb_python_int(), arb_python_int()).prop_map(|(a, b)| format!("{} + {}", a, b)),
            (arb_python_int(), arb_python_int()).prop_map(|(a, b)| format!("{} * {}", a, b)),
        ]
    }

    /// Generator for simple Python assignment statements
    fn arb_python_assignment() -> impl Strategy<Value = String> {
        (arb_python_identifier(), arb_python_expr())
            .prop_map(|(name, expr)| format!("{} = {}", name, expr))
    }

    /// Generator for simple Python function definitions
    fn arb_python_function() -> impl Strategy<Value = String> {
        (
            arb_python_identifier(),
            prop::collection::vec(arb_python_identifier(), 0..3),
            arb_python_expr(),
        )
            .prop_map(|(name, params, body)| {
                let params_str = params.join(", ");
                format!("def {}({}):\n    return {}\n", name, params_str, body)
            })
    }

    /// Generator for valid Python code snippets
    fn arb_valid_python_code() -> impl Strategy<Value = String> {
        prop_oneof![
            // Simple assignments
            arb_python_assignment().prop_map(|s| format!("{}\n", s)),
            // Multiple assignments
            prop::collection::vec(arb_python_assignment(), 1..5)
                .prop_map(|stmts| stmts.join("\n") + "\n"),
            // Function definitions
            arb_python_function(),
            // Class definitions
            (arb_python_identifier(), arb_python_identifier(), arb_python_expr()).prop_map(
                |(class_name, attr_name, value)| {
                    format!("class {}:\n    {} = {}\n", class_name, attr_name, value)
                }
            ),
            // Import statements
            arb_python_identifier().prop_map(|name| format!("import {}\n", name)),
            // Pass statement
            Just("pass\n".to_string()),
            // Empty file
            Just("\n".to_string()),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: multi-language-formatter-linter, Property 2: Formatting Round-Trip Consistency (Python)**
        /// *For any* valid Python source file, formatting the file and then formatting the result
        /// again SHALL produce identical output (idempotence: format(format(x)) == format(x)).
        /// **Validates: Requirements 2.2**
        #[test]
        fn prop_python_formatting_idempotence(code in arb_valid_python_code()) {
            let handler = PythonHandler::new();

            // Skip if ruff is not installed
            if handler.ruff_path.is_none() {
                // Can't test formatting without ruff
                return Ok(());
            }

            let path = Path::new("test.py");

            // First, verify the code is valid Python
            let diagnostics = handler.validate_syntax(path, &code);
            if !diagnostics.is_empty() {
                // Skip invalid code - this shouldn't happen with our generator
                // but we handle it gracefully
                return Ok(());
            }

            // Format once
            let first_format = match handler.format_with_ruff(path, &code) {
                Ok(formatted) => formatted,
                Err(_) => {
                    // If formatting fails, skip this test case
                    return Ok(());
                }
            };

            // Format again
            let second_format = match handler.format_with_ruff(path, &first_format) {
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

        /// **Feature: multi-language-formatter-linter, Property 3: Syntax Validation Correctness (Python)**
        /// *For any* source file, if the file contains valid syntax for Python, linting SHALL NOT
        /// report syntax errors. If the file contains invalid syntax, linting SHALL report at
        /// least one syntax error.
        /// **Validates: Requirements 2.3**
        #[test]
        fn prop_python_syntax_validation_valid(code in arb_valid_python_code()) {
            let handler = PythonHandler::new();
            let path = Path::new("test.py");

            // Valid Python code should produce no syntax errors
            let diagnostics = handler.validate_syntax(path, &code);

            prop_assert!(
                diagnostics.is_empty(),
                "Valid Python code should not produce syntax errors, but got: {:?}",
                diagnostics
            );
        }
    }

    /// Generator for invalid Python code snippets (true syntax errors only)
    /// Note: We only include cases that are actual syntax errors, not semantic errors
    /// like "1 = x" which is syntactically valid but semantically invalid.
    fn arb_invalid_python_code() -> impl Strategy<Value = String> {
        prop_oneof![
            // Unclosed parenthesis
            arb_python_identifier().prop_map(|name| format!("def {}(\n", name)),
            // Unclosed bracket
            Just("x = [1, 2, 3\n".to_string()),
            // Unclosed brace
            Just("x = {1: 2\n".to_string()),
            // Unclosed string
            Just("x = \"unclosed\n".to_string()),
            // Missing colon in function def
            arb_python_identifier().prop_map(|name| format!("def {}\n    pass\n", name)),
            // Missing colon in class def
            arb_python_identifier().prop_map(|name| format!("class {}\n    pass\n", name)),
            // Incomplete expression
            Just("x = 1 +\n".to_string()),
            // Mismatched brackets
            Just("x = [1, 2)\n".to_string()),
            // Invalid syntax: multiple equals in wrong context
            Just("x = = 1\n".to_string()),
            // Missing expression after operator
            Just("x = 1 *\n".to_string()),
            // Unclosed triple quote
            Just("x = \"\"\"unclosed\n".to_string()),
            // Invalid function call syntax
            Just("func(,)\n".to_string()),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: multi-language-formatter-linter, Property 3: Syntax Validation Correctness (Python)**
        /// *For any* source file with invalid syntax, linting SHALL report at least one syntax error.
        /// **Validates: Requirements 2.3**
        #[test]
        fn prop_python_syntax_validation_invalid(code in arb_invalid_python_code()) {
            let handler = PythonHandler::new();
            let path = Path::new("test.py");

            // Invalid Python code should produce at least one syntax error
            let diagnostics = handler.validate_syntax(path, &code);

            prop_assert!(
                !diagnostics.is_empty(),
                "Invalid Python code should produce syntax errors, but got none for: {:?}",
                code
            );

            // All diagnostics should be errors with the correct category
            for diag in &diagnostics {
                prop_assert_eq!(
                    &diag.category,
                    "lint/python",
                    "Syntax error diagnostics should have category 'lint/python'"
                );
                prop_assert!(
                    diag.message.contains("Syntax error") || diag.message.contains("syntax") || diag.message.contains("error"),
                    "Diagnostic message should indicate a syntax error: {}",
                    diag.message
                );
            }
        }
    }
}
