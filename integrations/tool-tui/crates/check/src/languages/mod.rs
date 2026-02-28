//! Multi-Language Formatter and Linter Support
//!
//! This module provides a unified interface for formatting and linting
//! multiple programming languages including Python, C/C++, Go, Rust,
//! PHP, Kotlin, Markdown, and TOML.

pub mod cpp;
pub mod css;
pub mod diagnostic;
pub mod external_tools;
pub mod go;
pub mod html;
pub mod javascript;
pub mod json;
pub mod kotlin;
pub mod markdown;
#[cfg(test)]
mod mode_tests;
pub mod php;
pub mod python;
pub mod rust_lang;
pub mod toml_lang;
pub mod yaml;

use std::path::Path;

pub use cpp::CppHandler;
pub use css::CssHandler;
pub use diagnostic::{Diagnostic, Severity};
pub use external_tools::{
    ExternalToolManager, InstallError, OperatingSystem, PackageManager, ToolCache, ToolVersion,
};
pub use go::GoHandler;
pub use html::HtmlHandler;
pub use javascript::JavaScriptHandler;
pub use json::JsonHandler;
pub use kotlin::KotlinHandler;
pub use markdown::MarkdownHandler;
pub use php::PhpHandler;
pub use python::PythonHandler;
pub use rust_lang::RustHandler;
pub use toml_lang::TomlHandler;
pub use yaml::YamlHandler;

/// Result type for file processing operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    /// File was changed (formatted/fixed)
    Changed,
    /// File was unchanged
    Unchanged,
    /// File was ignored (unsupported or excluded)
    Ignored,
    /// Processing failed with error
    Error(Diagnostic),
}

impl FileStatus {
    /// Returns true if the file was changed
    #[must_use]
    pub fn is_changed(&self) -> bool {
        matches!(self, FileStatus::Changed)
    }

    /// Returns true if the file was unchanged
    #[must_use]
    pub fn is_unchanged(&self) -> bool {
        matches!(self, FileStatus::Unchanged)
    }

    /// Returns true if the file was ignored
    #[must_use]
    pub fn is_ignored(&self) -> bool {
        matches!(self, FileStatus::Ignored)
    }

    /// Returns true if processing resulted in an error
    #[must_use]
    pub fn is_error(&self) -> bool {
        matches!(self, FileStatus::Error(_))
    }

    /// Returns the error diagnostic if this is an error status
    #[must_use]
    pub fn error(&self) -> Option<&Diagnostic> {
        match self {
            FileStatus::Error(d) => Some(d),
            _ => None,
        }
    }
}

/// Trait for language-specific handlers
///
/// All language handlers implement this trait for consistency.
/// Each handler is responsible for formatting and linting files
/// of a specific programming language.
pub trait LanguageHandler: Send + Sync {
    /// Returns the file extensions this handler supports (without the dot)
    ///
    /// # Example
    /// ```ignore
    /// fn extensions(&self) -> &[&str] {
    ///     &["py", "pyi"]
    /// }
    /// ```
    fn extensions(&self) -> &[&str];

    /// Format a file, returning the formatted content or status
    ///
    /// # Arguments
    /// * `path` - Path to the file being formatted
    /// * `content` - The file content to format
    /// * `write` - If true, write changes to disk; if false, just check
    ///
    /// # Returns
    /// * `Ok(FileStatus::Changed)` - File was formatted and differs from original
    /// * `Ok(FileStatus::Unchanged)` - File was already properly formatted
    /// * `Err(Diagnostic)` - Formatting failed with an error
    fn format(&self, path: &Path, content: &str, write: bool) -> Result<FileStatus, Diagnostic>;

    /// Lint a file, returning diagnostics
    ///
    /// # Arguments
    /// * `path` - Path to the file being linted
    /// * `content` - The file content to lint
    ///
    /// # Returns
    /// * `Ok(Vec<Diagnostic>)` - List of lint diagnostics (may be empty)
    /// * `Err(Diagnostic)` - Linting failed with an error
    fn lint(&self, path: &Path, content: &str) -> Result<Vec<Diagnostic>, Diagnostic>;

    /// Check a file (lint + format check)
    ///
    /// # Arguments
    /// * `path` - Path to the file being checked
    /// * `content` - The file content to check
    /// * `write` - If true, apply fixes; if false, just report
    ///
    /// # Returns
    /// * `Ok(FileStatus)` - Result of the check operation
    /// * `Err(Diagnostic)` - Check failed with an error
    fn check(&self, path: &Path, content: &str, write: bool) -> Result<FileStatus, Diagnostic>;

    /// Returns the name of this language handler
    fn name(&self) -> &str;
}

/// File processor that routes files to appropriate language handlers
pub struct FileProcessor {
    handlers: Vec<Box<dyn LanguageHandler>>,
}

impl FileProcessor {
    /// Create a new file processor with no handlers
    #[must_use]
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    /// Register a language handler
    pub fn register<H: LanguageHandler + 'static>(&mut self, handler: H) {
        self.handlers.push(Box::new(handler));
    }

    /// Get the handler for a file based on its extension
    ///
    /// # Arguments
    /// * `path` - Path to the file
    ///
    /// # Returns
    /// * `Some(&dyn LanguageHandler)` - Handler for this file type
    /// * `None` - No handler registered for this file type
    #[must_use]
    pub fn get_handler(&self, path: &Path) -> Option<&dyn LanguageHandler> {
        let ext = path.extension()?.to_str()?;
        self.handlers
            .iter()
            .find(|h| h.extensions().contains(&ext))
            .map(std::convert::AsRef::as_ref)
    }

    /// Check if a file is supported by any registered handler
    #[must_use]
    pub fn is_supported(&self, path: &Path) -> bool {
        self.get_handler(path).is_some()
    }

    /// Get all supported extensions
    #[must_use]
    pub fn supported_extensions(&self) -> Vec<&str> {
        self.handlers.iter().flat_map(|h| h.extensions().iter().copied()).collect()
    }

    /// Format a file using the appropriate handler
    ///
    /// # Arguments
    /// * `path` - Path to the file
    /// * `content` - File content
    /// * `write` - Whether to write changes to disk
    ///
    /// # Returns
    /// * `Ok(FileStatus)` - Result of formatting
    /// * `Err(Diagnostic)` - No handler found or formatting failed
    pub fn format(
        &self,
        path: &Path,
        content: &str,
        write: bool,
    ) -> Result<FileStatus, Diagnostic> {
        match self.get_handler(path) {
            Some(handler) => handler.format(path, content, write),
            None => Ok(FileStatus::Ignored),
        }
    }

    /// Lint a file using the appropriate handler
    ///
    /// # Arguments
    /// * `path` - Path to the file
    /// * `content` - File content
    ///
    /// # Returns
    /// * `Ok(Vec<Diagnostic>)` - Lint diagnostics
    /// * `Err(Diagnostic)` - No handler found or linting failed
    pub fn lint(&self, path: &Path, content: &str) -> Result<Vec<Diagnostic>, Diagnostic> {
        match self.get_handler(path) {
            Some(handler) => handler.lint(path, content),
            None => Ok(Vec::new()),
        }
    }

    /// Check a file using the appropriate handler
    ///
    /// # Arguments
    /// * `path` - Path to the file
    /// * `content` - File content
    /// * `write` - Whether to apply fixes
    ///
    /// # Returns
    /// * `Ok(FileStatus)` - Result of check
    /// * `Err(Diagnostic)` - No handler found or check failed
    pub fn check(&self, path: &Path, content: &str, write: bool) -> Result<FileStatus, Diagnostic> {
        match self.get_handler(path) {
            Some(handler) => handler.check(path, content, write),
            None => Ok(FileStatus::Ignored),
        }
    }

    /// Get the number of registered handlers
    #[must_use]
    pub fn handler_count(&self) -> usize {
        self.handlers.len()
    }
}

impl Default for FileProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mock handler for testing
    struct MockHandler {
        extensions: Vec<&'static str>,
        name: &'static str,
    }

    impl MockHandler {
        fn new(extensions: Vec<&'static str>, name: &'static str) -> Self {
            Self { extensions, name }
        }
    }

    impl LanguageHandler for MockHandler {
        fn extensions(&self) -> &[&str] {
            &self.extensions
        }

        fn format(
            &self,
            _path: &Path,
            _content: &str,
            _write: bool,
        ) -> Result<FileStatus, Diagnostic> {
            Ok(FileStatus::Unchanged)
        }

        fn lint(&self, _path: &Path, _content: &str) -> Result<Vec<Diagnostic>, Diagnostic> {
            Ok(Vec::new())
        }

        fn check(
            &self,
            _path: &Path,
            _content: &str,
            _write: bool,
        ) -> Result<FileStatus, Diagnostic> {
            Ok(FileStatus::Unchanged)
        }

        fn name(&self) -> &str {
            self.name
        }
    }

    #[test]
    fn test_file_status_methods() {
        assert!(FileStatus::Changed.is_changed());
        assert!(!FileStatus::Changed.is_unchanged());

        assert!(FileStatus::Unchanged.is_unchanged());
        assert!(!FileStatus::Unchanged.is_changed());

        assert!(FileStatus::Ignored.is_ignored());

        let error = FileStatus::Error(Diagnostic::error("test.py", "Test error", "format/python"));
        assert!(error.is_error());
        assert!(error.error().is_some());
    }

    #[test]
    fn test_file_processor_new() {
        let processor = FileProcessor::new();
        assert_eq!(processor.handler_count(), 0);
    }

    #[test]
    fn test_file_processor_register() {
        let mut processor = FileProcessor::new();
        processor.register(MockHandler::new(vec!["py", "pyi"], "python"));
        assert_eq!(processor.handler_count(), 1);
    }

    #[test]
    fn test_file_processor_get_handler() {
        let mut processor = FileProcessor::new();
        processor.register(MockHandler::new(vec!["py", "pyi"], "python"));
        processor.register(MockHandler::new(vec!["rs"], "rust"));

        let py_path = Path::new("test.py");
        let rs_path = Path::new("test.rs");
        let unknown_path = Path::new("test.xyz");

        assert!(processor.get_handler(py_path).is_some());
        assert!(processor.get_handler(rs_path).is_some());
        assert!(processor.get_handler(unknown_path).is_none());
    }

    #[test]
    fn test_file_processor_is_supported() {
        let mut processor = FileProcessor::new();
        processor.register(MockHandler::new(vec!["py"], "python"));

        assert!(processor.is_supported(Path::new("test.py")));
        assert!(!processor.is_supported(Path::new("test.rs")));
    }

    #[test]
    fn test_file_processor_supported_extensions() {
        let mut processor = FileProcessor::new();
        processor.register(MockHandler::new(vec!["py", "pyi"], "python"));
        processor.register(MockHandler::new(vec!["rs"], "rust"));

        let extensions = processor.supported_extensions();
        assert!(extensions.contains(&"py"));
        assert!(extensions.contains(&"pyi"));
        assert!(extensions.contains(&"rs"));
    }

    #[test]
    fn test_file_processor_format_unsupported() {
        let processor = FileProcessor::new();
        let result = processor.format(Path::new("test.xyz"), "", false);
        assert!(matches!(result, Ok(FileStatus::Ignored)));
    }

    #[test]
    fn test_file_processor_lint_unsupported() {
        let processor = FileProcessor::new();
        let result = processor.lint(Path::new("test.xyz"), "");
        assert!(matches!(result, Ok(diagnostics) if diagnostics.is_empty()));
    }

    #[test]
    fn test_file_processor_check_unsupported() {
        let processor = FileProcessor::new();
        let result = processor.check(Path::new("test.xyz"), "", false);
        assert!(matches!(result, Ok(FileStatus::Ignored)));
    }

    // Task 1.4: Unit tests for language detection

    #[test]
    fn test_extension_mapping_to_handlers() {
        let mut processor = FileProcessor::new();

        // Register handlers for different languages
        processor.register(MockHandler::new(vec!["py", "pyi"], "python"));
        processor.register(MockHandler::new(vec!["rs"], "rust"));
        processor.register(MockHandler::new(vec!["js", "jsx"], "javascript"));
        processor.register(MockHandler::new(vec!["md", "markdown"], "markdown"));

        // Test that each extension maps to the correct handler
        let test_cases = vec![
            ("test.py", Some("python")),
            ("test.pyi", Some("python")),
            ("test.rs", Some("rust")),
            ("test.js", Some("javascript")),
            ("test.jsx", Some("javascript")),
            ("test.md", Some("markdown")),
            ("test.markdown", Some("markdown")),
        ];

        for (path_str, expected_name) in test_cases {
            let path = Path::new(path_str);
            let handler = processor.get_handler(path);

            match expected_name {
                Some(name) => {
                    assert!(handler.is_some(), "Expected handler for {}", path_str);
                    assert_eq!(handler.unwrap().name(), name, "Wrong handler for {}", path_str);
                }
                None => {
                    assert!(handler.is_none(), "Expected no handler for {}", path_str);
                }
            }
        }
    }

    #[test]
    fn test_unknown_extension_handling() {
        let mut processor = FileProcessor::new();

        // Register some handlers
        processor.register(MockHandler::new(vec!["py"], "python"));
        processor.register(MockHandler::new(vec!["rs"], "rust"));

        // Test unknown extensions
        let unknown_extensions = vec![
            "test.xyz",
            "test.unknown",
            "test.abc123",
            "test.foo",
            "file.bar",
            "document.baz",
        ];

        for path_str in unknown_extensions {
            let path = Path::new(path_str);

            // get_handler should return None
            assert!(
                processor.get_handler(path).is_none(),
                "Expected no handler for unknown extension: {}",
                path_str
            );

            // is_supported should return false
            assert!(
                !processor.is_supported(path),
                "Expected unsupported for unknown extension: {}",
                path_str
            );

            // format should return Ignored
            let format_result = processor.format(path, "", false);
            assert!(
                matches!(format_result, Ok(FileStatus::Ignored)),
                "Expected Ignored status for format on unknown extension: {}",
                path_str
            );

            // lint should return empty diagnostics
            let lint_result = processor.lint(path, "");
            assert!(
                matches!(lint_result, Ok(ref diagnostics) if diagnostics.is_empty()),
                "Expected empty diagnostics for lint on unknown extension: {}",
                path_str
            );

            // check should return Ignored
            let check_result = processor.check(path, "", false);
            assert!(
                matches!(check_result, Ok(FileStatus::Ignored)),
                "Expected Ignored status for check on unknown extension: {}",
                path_str
            );
        }
    }

    #[test]
    fn test_case_insensitive_extension_matching() {
        let mut processor = FileProcessor::new();

        // Register handlers with lowercase extensions
        processor.register(MockHandler::new(vec!["py"], "python"));
        processor.register(MockHandler::new(vec!["rs"], "rust"));
        processor.register(MockHandler::new(vec!["js"], "javascript"));

        // Test various case combinations
        let test_cases = vec![
            // Lowercase (should work)
            ("test.py", true),
            ("test.rs", true),
            ("test.js", true),
            // Uppercase (currently won't work - extensions are case-sensitive)
            ("test.PY", false),
            ("test.RS", false),
            ("test.JS", false),
            // Mixed case (currently won't work)
            ("test.Py", false),
            ("test.Rs", false),
            ("test.Js", false),
            ("test.pY", false),
        ];

        for (path_str, should_be_supported) in test_cases {
            let path = Path::new(path_str);
            let is_supported = processor.is_supported(path);

            assert_eq!(
                is_supported, should_be_supported,
                "Extension matching for {} - expected {}, got {}",
                path_str, should_be_supported, is_supported
            );
        }
    }

    #[test]
    fn test_no_extension_handling() {
        let mut processor = FileProcessor::new();
        processor.register(MockHandler::new(vec!["py"], "python"));

        // Files without extensions
        let no_ext_files = vec![
            "Makefile",
            "README",
            "LICENSE",
            "Dockerfile",
            "file_without_ext",
        ];

        for path_str in no_ext_files {
            let path = Path::new(path_str);

            assert!(
                processor.get_handler(path).is_none(),
                "Expected no handler for file without extension: {}",
                path_str
            );

            assert!(
                !processor.is_supported(path),
                "Expected unsupported for file without extension: {}",
                path_str
            );
        }
    }

    #[test]
    fn test_multiple_dots_in_filename() {
        let mut processor = FileProcessor::new();
        processor.register(MockHandler::new(vec!["py"], "python"));
        processor.register(MockHandler::new(vec!["js"], "javascript"));

        // Files with multiple dots - should use the last extension
        let test_cases = vec![
            ("test.min.js", Some("javascript")),
            ("config.test.py", Some("python")),
            ("file.backup.py", Some("python")),
            ("script.v2.js", Some("javascript")),
            ("data.2024.01.py", Some("python")),
        ];

        for (path_str, expected_name) in test_cases {
            let path = Path::new(path_str);
            let handler = processor.get_handler(path);

            match expected_name {
                Some(name) => {
                    assert!(handler.is_some(), "Expected handler for {}", path_str);
                    assert_eq!(handler.unwrap().name(), name, "Wrong handler for {}", path_str);
                }
                None => {
                    assert!(handler.is_none(), "Expected no handler for {}", path_str);
                }
            }
        }
    }

    #[test]
    fn test_handler_priority_with_overlapping_extensions() {
        let mut processor = FileProcessor::new();

        // Register two handlers with the same extension (last one wins)
        processor.register(MockHandler::new(vec!["txt"], "handler1"));
        processor.register(MockHandler::new(vec!["txt"], "handler2"));

        let path = Path::new("test.txt");
        let handler = processor.get_handler(path);

        // The last registered handler should be used
        assert!(handler.is_some());
        assert_eq!(handler.unwrap().name(), "handler2");
    }

    #[test]
    fn test_empty_extension() {
        let mut processor = FileProcessor::new();
        processor.register(MockHandler::new(vec!["py"], "python"));

        // File ending with a dot but no extension
        let path = Path::new("test.");

        // This should not match any handler
        assert!(processor.get_handler(path).is_none());
        assert!(!processor.is_supported(path));
    }
}
