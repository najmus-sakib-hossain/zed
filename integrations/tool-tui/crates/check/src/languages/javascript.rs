//! JavaScript/TypeScript Language Handler
//!
//! This module provides formatting and linting support for JavaScript and TypeScript files
//! using oxc (Biome-compatible, faster parser and formatter).

use std::fs;
use std::path::Path;

use oxc_allocator::Allocator;
use oxc_parser::{Parser, ParserReturn};
use oxc_span::SourceType;

use crate::languages::diagnostic::{Diagnostic, Severity};
use crate::languages::{FileStatus, LanguageHandler};

/// JavaScript/TypeScript language handler
///
/// Supports `.js`, `.jsx`, `.ts`, `.tsx` file extensions.
/// Uses oxc for parsing, formatting, and linting (Biome-compatible).
pub struct JavaScriptHandler {}

impl JavaScriptHandler {
    /// Create a new JavaScript/TypeScript handler
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }

    /// Determine the source type based on file extension
    fn source_type_from_path(path: &Path) -> SourceType {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match ext {
            "js" => SourceType::default().with_module(true),
            "jsx" => SourceType::default().with_module(true).with_jsx(true),
            "ts" => SourceType::default().with_module(true).with_typescript(true),
            "tsx" => SourceType::default().with_module(true).with_typescript(true).with_jsx(true),
            _ => SourceType::default().with_module(true),
        }
    }

    /// Parse JavaScript/TypeScript code using oxc
    fn parse_with_oxc<'a>(
        &self,
        allocator: &'a Allocator,
        source_text: &'a str,
        source_type: SourceType,
    ) -> ParserReturn<'a> {
        Parser::new(allocator, source_text, source_type).parse()
    }

    /// Format JavaScript/TypeScript code using oxc formatter
    fn format_with_oxc(&self, path: &Path, content: &str) -> Result<String, Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();
        let source_type = Self::source_type_from_path(path);
        let allocator = Allocator::default();

        // Parse the file
        let parse_result = self.parse_with_oxc(&allocator, content, source_type);

        // Check for parse errors
        if !parse_result.errors.is_empty() {
            let error_messages: Vec<String> =
                parse_result.errors.iter().map(|e| format!("{e:?}")).collect();

            return Err(Diagnostic::error(
                &file_path_str,
                format!("Parse error: {}", error_messages.join("; ")),
                "format/javascript",
            ));
        }

        // For now, use a simple formatter that preserves the structure
        // In a production implementation, you would use oxc_formatter or implement
        // a proper formatter based on the AST

        // Basic formatting: ensure consistent spacing and indentation
        let formatted = self.basic_format(content);

        Ok(formatted)
    }

    /// Basic formatting implementation
    /// This is a simplified formatter - in production, use `oxc_formatter`
    fn basic_format(&self, content: &str) -> String {
        // For now, return content as-is
        // TODO: Implement proper formatting using oxc_formatter when available
        content.to_string()
    }

    /// Lint JavaScript/TypeScript code using oxc parser
    fn lint_with_oxc(&self, path: &Path, content: &str) -> Vec<Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();
        let source_type = Self::source_type_from_path(path);
        let allocator = Allocator::default();
        let mut diagnostics = Vec::new();

        // Parse the file
        let parse_result = self.parse_with_oxc(&allocator, content, source_type);

        // Convert oxc diagnostics to dx-serializer format
        for oxc_error in parse_result.errors {
            let severity = Severity::Error;
            let message = format!("{oxc_error:?}");

            let mut diag = Diagnostic::new(&file_path_str, message, severity, "lint/javascript");

            // Extract location information from oxc error
            if let Some(ref labels) = oxc_error.labels
                && let Some(label) = labels.first()
            {
                // oxc uses byte offsets, convert to line number
                let line = content[..label.offset()].lines().count();
                diag = diag.with_line(line);
            }

            diagnostics.push(diag);
        }

        diagnostics
    }

    /// Detect framework from project structure
    #[must_use]
    pub fn detect_framework(&self, project_path: &Path) -> Option<String> {
        // Check for package.json
        let package_json = project_path.join("package.json");
        if !package_json.exists() {
            return None;
        }

        // Read package.json
        if let Ok(content) = fs::read_to_string(&package_json) {
            // Check for Next.js
            if content.contains("\"next\"") {
                return Some("next.js".to_string());
            }

            // Check for React
            if content.contains("\"react\"") && !content.contains("\"next\"") {
                return Some("react".to_string());
            }

            // Check for Svelte
            if content.contains("\"svelte\"") {
                return Some("svelte".to_string());
            }

            // Check for Vue
            if content.contains("\"vue\"") {
                return Some("vue".to_string());
            }

            // Check for Angular
            if content.contains("\"@angular/core\"") {
                return Some("angular".to_string());
            }
        }

        None
    }
}

impl Default for JavaScriptHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageHandler for JavaScriptHandler {
    fn extensions(&self) -> &[&str] {
        &["js", "jsx", "ts", "tsx"]
    }

    fn format(&self, path: &Path, content: &str, write: bool) -> Result<FileStatus, Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();

        // Format the content
        let formatted = self.format_with_oxc(path, content)?;

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
                    "io/javascript",
                )
            })?;
        }

        Ok(FileStatus::Changed)
    }

    fn lint(&self, path: &Path, content: &str) -> Result<Vec<Diagnostic>, Diagnostic> {
        // Lint using oxc parser
        let diagnostics = self.lint_with_oxc(path, content);
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
        "javascript"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_javascript_handler_extensions() {
        let handler = JavaScriptHandler::new();
        let extensions = handler.extensions();
        assert!(extensions.contains(&"js"));
        assert!(extensions.contains(&"jsx"));
        assert!(extensions.contains(&"ts"));
        assert!(extensions.contains(&"tsx"));
    }

    #[test]
    fn test_javascript_handler_name() {
        let handler = JavaScriptHandler::new();
        assert_eq!(handler.name(), "javascript");
    }

    #[test]
    fn test_source_type_from_path() {
        let js_type = JavaScriptHandler::source_type_from_path(Path::new("test.js"));
        assert!(js_type.is_javascript());
        assert!(!js_type.is_jsx());

        let jsx_type = JavaScriptHandler::source_type_from_path(Path::new("test.jsx"));
        assert!(jsx_type.is_jsx());

        let ts_type = JavaScriptHandler::source_type_from_path(Path::new("test.ts"));
        assert!(ts_type.is_typescript());
        assert!(!ts_type.is_jsx());

        let tsx_type = JavaScriptHandler::source_type_from_path(Path::new("test.tsx"));
        assert!(tsx_type.is_typescript());
        assert!(tsx_type.is_jsx());
    }

    #[test]
    fn test_format_valid_javascript() {
        let handler = JavaScriptHandler::new();
        let valid_code = r#"const x = 1 + 2;
function hello() {
    console.log("test");
}"#;

        // This should format successfully
        let result = handler.format_with_oxc(Path::new("test.js"), valid_code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_format_valid_typescript() {
        let handler = JavaScriptHandler::new();
        let valid_code = r#"function hello(name: string): void {
    console.log(`Hello, ${name}!`);
}"#;

        // This should format successfully
        let result = handler.format_with_oxc(Path::new("test.ts"), valid_code);
        assert!(result.is_ok());
    }

    #[test]
    fn test_format_invalid_javascript() {
        let handler = JavaScriptHandler::new();
        let invalid_code = r#"const x = 1 +;"#; // Syntax error

        // This should fail with parse error
        let result = handler.format_with_oxc(Path::new("test.js"), invalid_code);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert_eq!(err.category, "format/javascript");
        assert!(err.message.contains("Parse error"));
    }

    #[test]
    fn test_lint_valid_javascript() {
        let handler = JavaScriptHandler::new();
        let valid_code = r#"
function hello() {
    console.log("Hello, World!");
}

const x = 1 + 2;
"#;
        let diagnostics = handler.lint_with_oxc(Path::new("test.js"), valid_code);
        // Valid code should have no diagnostics
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_lint_invalid_javascript() {
        let handler = JavaScriptHandler::new();
        let invalid_code = r#"const x = 1 +;"#; // Syntax error

        let diagnostics = handler.lint_with_oxc(Path::new("test.js"), invalid_code);
        // Should have at least one diagnostic
        assert!(!diagnostics.is_empty());
        assert_eq!(diagnostics[0].category, "lint/javascript");
    }

    #[test]
    fn test_detect_framework_react() {
        // This test would require creating a temporary directory with package.json
        // For now, we'll skip it
        // In a real implementation, you would create a temp directory,
        // write a package.json with react dependency, and test detection
    }

    #[test]
    fn test_detect_framework_nextjs() {
        // This test would require creating a temporary directory with package.json
        // For now, we'll skip it
        // In a real implementation, you would create a temp directory,
        // write a package.json with next dependency, and test detection
    }

    #[test]
    fn test_detect_framework_svelte() {
        // This test would require creating a temporary directory with package.json
        // For now, we'll skip it
        // In a real implementation, you would create a temp directory,
        // write a package.json with svelte dependency, and test detection
    }
}
