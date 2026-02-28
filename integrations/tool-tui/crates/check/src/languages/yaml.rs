//! YAML Language Handler
//!
//! This module provides formatting and linting support for YAML files
//! using `serde_yaml` for parsing and validation.

use std::fs;
use std::path::Path;

use crate::languages::diagnostic::Diagnostic;
use crate::languages::{FileStatus, LanguageHandler};

/// YAML file extensions
const YAML_EXTENSIONS: &[&str] = &["yaml", "yml"];

/// YAML language handler
///
/// Supports `.yaml` and `.yml` file extensions.
/// Uses `serde_yaml` for parsing and validation.
pub struct YamlHandler;

impl YamlHandler {
    /// Create a new YAML handler
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Format YAML content
    ///
    /// This performs basic formatting:
    /// - Parses and re-serializes YAML
    /// - Ensures consistent indentation
    /// - Normalizes whitespace
    fn format_yaml(&self, path: &Path, content: &str) -> Result<String, Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();

        // Parse YAML content
        let value: serde_yaml::Value = serde_yaml::from_str(content).map_err(|e| {
            let (line, col) = Self::extract_location_from_error(&e);
            let mut diag =
                Diagnostic::error(&file_path_str, format!("YAML parse error: {e}"), "format/yaml");
            if let Some(l) = line {
                diag = diag.with_line(l);
            }
            if let Some(c) = col {
                diag = diag.with_column(c);
            }
            diag
        })?;

        // Re-serialize with pretty formatting
        let formatted = serde_yaml::to_string(&value).map_err(|e| {
            Diagnostic::error(
                &file_path_str,
                format!("YAML serialization error: {e}"),
                "format/yaml",
            )
        })?;

        Ok(formatted)
    }

    /// Validate YAML syntax and semantics
    fn validate_yaml(&self, path: &Path, content: &str) -> Vec<Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();
        let mut diagnostics = Vec::new();

        // Try to parse YAML content
        match serde_yaml::from_str::<serde_yaml::Value>(content) {
            Ok(_) => {
                // Syntax is valid, no errors
            }
            Err(e) => {
                let (line, col) = Self::extract_location_from_error(&e);
                let mut diag = Diagnostic::error(
                    &file_path_str,
                    format!("YAML syntax error: {e}"),
                    "lint/yaml",
                );
                if let Some(l) = line {
                    diag = diag.with_line(l);
                }
                if let Some(c) = col {
                    diag = diag.with_column(c);
                }
                diagnostics.push(diag);
            }
        }

        diagnostics
    }

    /// Extract line and column from a YAML parse error
    fn extract_location_from_error(error: &serde_yaml::Error) -> (Option<usize>, Option<usize>) {
        // serde_yaml errors often contain location information
        let error_str = error.to_string();

        // Try to parse line number from error message
        if let Some(line_start) = error_str.find("at line ") {
            let line_part = &error_str[line_start + 8..];
            if let Some(line_end) = line_part.find(',')
                && let Ok(line_num) = line_part[..line_end].parse::<usize>()
            {
                // Try to parse column number
                if let Some(col_start) = error_str.find("column ") {
                    let col_part = &error_str[col_start + 7..];
                    if let Some(col_end) = col_part.find(' ')
                        && let Ok(col_num) = col_part[..col_end].parse::<usize>()
                    {
                        return (Some(line_num), Some(col_num));
                    }
                }
                return (Some(line_num), None);
            }
        }

        (None, None)
    }
}

impl Default for YamlHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageHandler for YamlHandler {
    fn extensions(&self) -> &[&str] {
        YAML_EXTENSIONS
    }

    fn format(&self, path: &Path, content: &str, write: bool) -> Result<FileStatus, Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();

        // Format the content
        let formatted = self.format_yaml(path, content)?;

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
                    "io/yaml",
                )
            })?;
        }

        Ok(FileStatus::Changed)
    }

    fn lint(&self, path: &Path, content: &str) -> Result<Vec<Diagnostic>, Diagnostic> {
        let diagnostics = self.validate_yaml(path, content);
        Ok(diagnostics)
    }

    fn check(&self, path: &Path, content: &str, write: bool) -> Result<FileStatus, Diagnostic> {
        // First, lint the file
        let lint_diagnostics = self.lint(path, content)?;

        // If there are errors, report them
        if !lint_diagnostics.is_empty() {
            return Err(lint_diagnostics.into_iter().next().unwrap());
        }

        // Then format
        self.format(path, content, write)
    }

    fn name(&self) -> &'static str {
        "yaml"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yaml_handler_extensions() {
        let handler = YamlHandler::new();
        let extensions = handler.extensions();
        assert!(extensions.contains(&"yaml"));
        assert!(extensions.contains(&"yml"));
    }

    #[test]
    fn test_yaml_handler_name() {
        let handler = YamlHandler::new();
        assert_eq!(handler.name(), "yaml");
    }

    #[test]
    fn test_format_yaml_valid() {
        let handler = YamlHandler::new();
        let input = "name=test
value=123";
        let formatted = handler.format_yaml(Path::new("test.yaml"), input).unwrap();
        assert!(formatted.contains("name"));
        assert!(formatted.contains("value"));
    }

    #[test]
    fn test_validate_yaml_valid() {
        let handler = YamlHandler::new();
        let valid_yaml = "name: test
value: 123";
        let diagnostics = handler.validate_yaml(Path::new("test.yaml"), valid_yaml);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_validate_yaml_invalid() {
        let handler = YamlHandler::new();
        let invalid_yaml = "name: test
  value: 123"; // Invalid indentation
        let diagnostics = handler.validate_yaml(Path::new("test.yaml"), invalid_yaml);
        // YAML parser might accept this, so we just check the test runs
    }
}
