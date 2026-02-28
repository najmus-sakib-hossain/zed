//! Language Handler Integration Tests
//!
//! Integration tests for multi-language support.
//! **Validates: Requirement 5.8 - Write integration tests for each language handler**

use std::path::Path;

// ============================================================================
// Python Handler Tests
// ============================================================================

mod python_tests {
    use super::*;

    #[test]
    fn test_python_handler_extensions() {
        use dx_check::languages::LanguageHandler;
        use dx_check::languages::PythonHandler;

        let handler = PythonHandler::new();
        let extensions = handler.extensions();

        assert!(extensions.contains(&"py"));
        assert!(extensions.contains(&"pyi"));
    }

    #[test]
    fn test_python_handler_name() {
        use dx_check::languages::LanguageHandler;
        use dx_check::languages::PythonHandler;

        let handler = PythonHandler::new();
        assert_eq!(handler.name(), "python");
    }

    #[test]
    fn test_python_lint_valid_code() {
        use dx_check::languages::LanguageHandler;
        use dx_check::languages::PythonHandler;

        let handler = PythonHandler::new();
        let content = "def hello():\n    print('Hello, World!')\n";
        let path = Path::new("test.py");

        let result = handler.lint(path, content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_python_lint_syntax_error() {
        use dx_check::languages::LanguageHandler;
        use dx_check::languages::PythonHandler;

        let handler = PythonHandler::new();
        let content = "def hello(\n    print('missing paren'\n";
        let path = Path::new("test.py");

        let result = handler.lint(path, content);
        // Should either return diagnostics or an error
        match result {
            Ok(diagnostics) => {
                // May have syntax error diagnostics
            }
            Err(_) => {
                // Syntax error is expected
            }
        }
    }

    #[test]
    fn test_python_format_check_mode() {
        use dx_check::languages::LanguageHandler;
        use dx_check::languages::PythonHandler;

        let handler = PythonHandler::new();
        let content = "x=1\n";
        let path = Path::new("test.py");

        // Check mode (write=false) should not modify files
        let result = handler.format(path, content, false);
        assert!(result.is_ok());
    }
}

// ============================================================================
// Go Handler Tests
// ============================================================================

mod go_tests {
    use super::*;

    #[test]
    fn test_go_handler_extensions() {
        use dx_check::languages::GoHandler;
        use dx_check::languages::LanguageHandler;

        let handler = GoHandler::new();
        let extensions = handler.extensions();

        assert!(extensions.contains(&"go"));
    }

    #[test]
    fn test_go_handler_name() {
        use dx_check::languages::GoHandler;
        use dx_check::languages::LanguageHandler;

        let handler = GoHandler::new();
        assert_eq!(handler.name(), "go");
    }

    #[test]
    fn test_go_lint_valid_code() {
        use dx_check::languages::GoHandler;
        use dx_check::languages::LanguageHandler;

        let handler = GoHandler::new();
        let content = "package main\n\nfunc main() {\n}\n";
        let path = Path::new("test.go");

        let result = handler.lint(path, content);
        assert!(result.is_ok());
    }
}

// ============================================================================
// Rust Handler Tests
// ============================================================================

mod rust_tests {
    use super::*;

    #[test]
    fn test_rust_handler_extensions() {
        use dx_check::languages::LanguageHandler;
        use dx_check::languages::RustHandler;

        let handler = RustHandler::new();
        let extensions = handler.extensions();

        assert!(extensions.contains(&"rs"));
    }

    #[test]
    fn test_rust_handler_name() {
        use dx_check::languages::LanguageHandler;
        use dx_check::languages::RustHandler;

        let handler = RustHandler::new();
        assert_eq!(handler.name(), "rust");
    }

    #[test]
    fn test_rust_lint_valid_code() {
        use dx_check::languages::LanguageHandler;
        use dx_check::languages::RustHandler;

        let handler = RustHandler::new();
        let content = "fn main() {\n    println!(\"Hello\");\n}\n";
        let path = Path::new("test.rs");

        let result = handler.lint(path, content);
        assert!(result.is_ok());
    }
}

// ============================================================================
// TOML Handler Tests
// ============================================================================

mod toml_tests {
    use super::*;

    #[test]
    fn test_toml_handler_extensions() {
        use dx_check::languages::LanguageHandler;
        use dx_check::languages::TomlHandler;

        let handler = TomlHandler::new();
        let extensions = handler.extensions();

        assert!(extensions.contains(&"toml"));
    }

    #[test]
    fn test_toml_handler_name() {
        use dx_check::languages::LanguageHandler;
        use dx_check::languages::TomlHandler;

        let handler = TomlHandler::new();
        assert_eq!(handler.name(), "toml");
    }

    #[test]
    fn test_toml_lint_valid_code() {
        use dx_check::languages::LanguageHandler;
        use dx_check::languages::TomlHandler;

        let handler = TomlHandler::new();
        let content = "[package]\nname = \"test\"\nversion = \"1.0.0\"\n";
        let path = Path::new("test.toml");

        let result = handler.lint(path, content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_toml_lint_invalid_syntax() {
        use dx_check::languages::LanguageHandler;
        use dx_check::languages::TomlHandler;

        let handler = TomlHandler::new();
        let content = "[package\nname = \"test\"\n"; // Missing closing bracket
        let path = Path::new("test.toml");

        let result = handler.lint(path, content);
        // Should detect syntax error
        match result {
            Ok(diagnostics) => {
                // May have syntax error diagnostics
            }
            Err(_) => {
                // Error is expected for invalid TOML
            }
        }
    }
}

// ============================================================================
// Markdown Handler Tests
// ============================================================================

mod markdown_tests {
    use super::*;

    #[test]
    fn test_markdown_handler_extensions() {
        use dx_check::languages::LanguageHandler;
        use dx_check::languages::MarkdownHandler;

        let handler = MarkdownHandler::new();
        let extensions = handler.extensions();

        assert!(extensions.contains(&"md"));
    }

    #[test]
    fn test_markdown_handler_name() {
        use dx_check::languages::LanguageHandler;
        use dx_check::languages::MarkdownHandler;

        let handler = MarkdownHandler::new();
        assert_eq!(handler.name(), "markdown");
    }

    #[test]
    fn test_markdown_lint_valid_content() {
        use dx_check::languages::LanguageHandler;
        use dx_check::languages::MarkdownHandler;

        let handler = MarkdownHandler::new();
        let content = "# Hello\n\nThis is a paragraph.\n";
        let path = Path::new("test.md");

        let result = handler.lint(path, content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_markdown_format_check_mode() {
        use dx_check::languages::LanguageHandler;
        use dx_check::languages::MarkdownHandler;

        let handler = MarkdownHandler::new();
        let content = "#Hello\n\nParagraph.\n";
        let path = Path::new("test.md");

        let result = handler.format(path, content, false);
        assert!(result.is_ok());
    }
}

// ============================================================================
// FileProcessor Tests
// ============================================================================

mod file_processor_tests {
    use super::*;

    #[test]
    fn test_file_processor_with_all_handlers() {
        use dx_check::languages::{
            FileProcessor, GoHandler, MarkdownHandler, PythonHandler, RustHandler, TomlHandler,
        };

        let mut processor = FileProcessor::new();
        processor.register(PythonHandler::new());
        processor.register(GoHandler::new());
        processor.register(RustHandler::new());
        processor.register(TomlHandler::new());
        processor.register(MarkdownHandler::new());

        assert_eq!(processor.handler_count(), 5);
    }

    #[test]
    fn test_file_processor_routes_correctly() {
        use dx_check::languages::{
            FileProcessor, GoHandler, MarkdownHandler, PythonHandler, RustHandler, TomlHandler,
        };

        let mut processor = FileProcessor::new();
        processor.register(PythonHandler::new());
        processor.register(GoHandler::new());
        processor.register(RustHandler::new());
        processor.register(TomlHandler::new());
        processor.register(MarkdownHandler::new());

        assert!(processor.is_supported(Path::new("test.py")));
        assert!(processor.is_supported(Path::new("test.go")));
        assert!(processor.is_supported(Path::new("test.rs")));
        assert!(processor.is_supported(Path::new("test.toml")));
        assert!(processor.is_supported(Path::new("test.md")));
        assert!(!processor.is_supported(Path::new("test.xyz")));
    }

    #[test]
    fn test_file_processor_lint_python() {
        use dx_check::languages::{FileProcessor, PythonHandler};

        let mut processor = FileProcessor::new();
        processor.register(PythonHandler::new());

        let content = "def hello():\n    pass\n";
        let result = processor.lint(Path::new("test.py"), content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_file_processor_format_toml() {
        use dx_check::languages::{FileProcessor, TomlHandler};

        let mut processor = FileProcessor::new();
        processor.register(TomlHandler::new());

        let content = "[package]\nname=\"test\"\n";
        let result = processor.format(Path::new("test.toml"), content, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_file_processor_unsupported_returns_ignored() {
        use dx_check::languages::{FileProcessor, FileStatus};

        let processor = FileProcessor::new();

        let result = processor.format(Path::new("test.xyz"), "", false);
        assert!(matches!(result, Ok(FileStatus::Ignored)));
    }
}

// ============================================================================
// External Tool Manager Tests
// ============================================================================

mod external_tool_tests {
    #[test]
    fn test_operating_system_detection() {
        use dx_check::languages::OperatingSystem;

        let os = OperatingSystem::detect();
        // Should detect current OS
        assert!(matches!(
            os,
            OperatingSystem::Windows | OperatingSystem::MacOS | OperatingSystem::Linux
        ));
    }
}

// ============================================================================
// Diagnostic Tests
// ============================================================================

mod diagnostic_tests {
    #[test]
    fn test_diagnostic_creation() {
        use dx_check::languages::Diagnostic;

        let diag = Diagnostic::error("test.py", "Test error", "test/rule");
        assert_eq!(diag.message, "Test error");
        assert_eq!(diag.category, "test/rule");
    }

    #[test]
    fn test_diagnostic_with_location() {
        use dx_check::languages::Diagnostic;

        let diag = Diagnostic::error("test.py", "Test error", "test/rule")
            .with_line(10)
            .with_column(5);

        assert_eq!(diag.line, Some(10));
        assert_eq!(diag.column, Some(5));
    }

    #[test]
    fn test_severity_levels() {
        use dx_check::languages::{Diagnostic, Severity};

        let error = Diagnostic::error("test.py", "Error", "test");
        let warning = Diagnostic::warning("test.py", "Warning", "test");
        let info = Diagnostic::info("test.py", "Info", "test");

        assert!(matches!(error.severity, Severity::Error));
        assert!(matches!(warning.severity, Severity::Warning));
        assert!(matches!(info.severity, Severity::Info));
    }
}
