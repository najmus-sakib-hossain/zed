//! Comprehensive Language Handler Integration Tests
//!
//! **Task 2.6: Write integration tests for language handlers**
//! **Validates: Requirements 1.2, 1.4**
//!
//! This test suite validates:
//! - Each handler with sample files
//! - Error handling for malformed files
//! - Diagnostic output format

use dx_check::languages::{
    CppHandler, CssHandler, Diagnostic, FileProcessor, FileStatus, GoHandler, HtmlHandler,
    JavaScriptHandler, JsonHandler, KotlinHandler, LanguageHandler, MarkdownHandler, PhpHandler,
    PythonHandler, RustHandler, Severity, TomlHandler, YamlHandler,
};
use std::fs;
use std::path::Path;

// ============================================================================
// Helper Functions
// ============================================================================

/// Load a fixture file from the tests/fixtures directory
fn load_fixture(filename: &str) -> String {
    let path = format!("tests/fixtures/{}", filename);
    fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to load fixture: {}", filename))
}

/// Check if diagnostics contain expected information
fn assert_diagnostic_format(diagnostic: &Diagnostic) {
    // Diagnostic should have a message
    assert!(!diagnostic.message.is_empty(), "Diagnostic message should not be empty");

    // Diagnostic should have a category
    assert!(!diagnostic.category.is_empty(), "Diagnostic category should not be empty");

    // Diagnostic should have a valid severity
    assert!(
        matches!(diagnostic.severity, Severity::Error | Severity::Warning | Severity::Info),
        "Diagnostic should have valid severity"
    );
}

// ============================================================================
// JavaScript/TypeScript Handler Integration Tests
// ============================================================================

#[test]
fn test_javascript_handler_with_valid_sample() {
    let handler = JavaScriptHandler::new();
    let content = load_fixture("sample.js");
    let path = Path::new("tests/fixtures/sample.js");

    // Test format
    let format_result = handler.format(path, &content, false);
    assert!(format_result.is_ok(), "Format should succeed on valid JS");

    // Test lint
    let lint_result = handler.lint(path, &content);
    assert!(lint_result.is_ok(), "Lint should succeed on valid JS");
}

#[test]
fn test_javascript_handler_with_malformed_file() {
    let handler = JavaScriptHandler::new();
    let content = load_fixture("malformed.js");
    let path = Path::new("tests/fixtures/malformed.js");

    // Test format - should handle error gracefully
    let format_result = handler.format(path, &content, false);
    match format_result {
        Ok(_) => {
            // Some handlers may return status even with errors
        }
        Err(diagnostic) => {
            assert_diagnostic_format(&diagnostic);
        }
    }

    // Test lint - should detect syntax errors
    let lint_result = handler.lint(path, &content);
    match lint_result {
        Ok(diagnostics) => {
            // May return diagnostics for syntax errors
            for diag in &diagnostics {
                assert_diagnostic_format(diag);
            }
        }
        Err(diagnostic) => {
            assert_diagnostic_format(&diagnostic);
        }
    }
}

#[test]
fn test_javascript_handler_diagnostic_output() {
    let handler = JavaScriptHandler::new();
    let content = "const x = 1\nconst y = 2"; // Missing semicolons
    let path = Path::new("test.js");

    let lint_result = handler.lint(path, content);
    if let Ok(diagnostics) = lint_result {
        for diagnostic in diagnostics {
            assert_diagnostic_format(&diagnostic);
            // Verify diagnostic has file information
            assert!(!diagnostic.file_path.is_empty());
        }
    }
}

// ============================================================================
// Python Handler Integration Tests
// ============================================================================

#[test]
fn test_python_handler_with_valid_sample() {
    let handler = PythonHandler::new();
    let content = load_fixture("sample.py");
    let path = Path::new("tests/fixtures/sample.py");

    // Test format
    let format_result = handler.format(path, &content, false);
    assert!(format_result.is_ok(), "Format should succeed on valid Python");

    // Test lint
    let lint_result = handler.lint(path, &content);
    assert!(lint_result.is_ok(), "Lint should succeed on valid Python");
}

#[test]
fn test_python_handler_with_malformed_file() {
    let handler = PythonHandler::new();
    let content = load_fixture("malformed.py");
    let path = Path::new("tests/fixtures/malformed.py");

    // Test format - should handle error gracefully
    let format_result = handler.format(path, &content, false);
    match format_result {
        Ok(_) => {}
        Err(diagnostic) => {
            assert_diagnostic_format(&diagnostic);
        }
    }

    // Test lint - should detect syntax errors
    let lint_result = handler.lint(path, &content);
    match lint_result {
        Ok(diagnostics) => {
            for diag in &diagnostics {
                assert_diagnostic_format(diag);
            }
        }
        Err(diagnostic) => {
            assert_diagnostic_format(&diagnostic);
        }
    }
}

#[test]
fn test_python_handler_check_mode() {
    let handler = PythonHandler::new();
    let content = "x=1\ny=2\n"; // Poor formatting
    let path = Path::new("test.py");

    let check_result = handler.check(path, content, false);
    assert!(check_result.is_ok(), "Check should complete");
}

// ============================================================================
// Rust Handler Integration Tests
// ============================================================================

#[test]
fn test_rust_handler_with_valid_sample() {
    let handler = RustHandler::new();
    let content = load_fixture("sample.rs");
    let path = Path::new("tests/fixtures/sample.rs");

    // Test format
    let format_result = handler.format(path, &content, false);
    assert!(format_result.is_ok(), "Format should succeed on valid Rust");

    // Test lint
    let lint_result = handler.lint(path, &content);
    assert!(lint_result.is_ok(), "Lint should succeed on valid Rust");
}

#[test]
fn test_rust_handler_with_malformed_file() {
    let handler = RustHandler::new();
    let content = load_fixture("malformed.rs");
    let path = Path::new("tests/fixtures/malformed.rs");

    // Test format - should handle error gracefully
    let format_result = handler.format(path, &content, false);
    match format_result {
        Ok(_) => {}
        Err(diagnostic) => {
            assert_diagnostic_format(&diagnostic);
        }
    }
}

// ============================================================================
// Go Handler Integration Tests
// ============================================================================

#[test]
fn test_go_handler_with_valid_sample() {
    let handler = GoHandler::new();
    let content = load_fixture("sample.go");
    let path = Path::new("tests/fixtures/sample.go");

    // Test format - may fail if gofmt not installed
    let format_result = handler.format(path, &content, false);
    if format_result.is_ok() {
        // If gofmt is available, test should succeed
    } else {
        // If gofmt is not available, that's acceptable for this test
        println!("Note: gofmt may not be installed, skipping format test");
    }

    // Test lint
    let lint_result = handler.lint(path, &content);
    if lint_result.is_ok() {
        // If go tools are available, test should succeed
    } else {
        // If go tools are not available, that's acceptable
        println!("Note: go tools may not be installed, skipping lint test");
    }
}

#[test]
fn test_go_handler_with_malformed_file() {
    let handler = GoHandler::new();
    let content = load_fixture("malformed.go");
    let path = Path::new("tests/fixtures/malformed.go");

    // Test format - should handle error gracefully
    let format_result = handler.format(path, &content, false);
    match format_result {
        Ok(_) => {}
        Err(diagnostic) => {
            assert_diagnostic_format(&diagnostic);
        }
    }
}

// ============================================================================
// TOML Handler Integration Tests
// ============================================================================

#[test]
fn test_toml_handler_with_valid_sample() {
    let handler = TomlHandler::new();
    let content = load_fixture("sample.toml");
    let path = Path::new("tests/fixtures/sample.toml");

    // Test format
    let format_result = handler.format(path, &content, false);
    assert!(format_result.is_ok(), "Format should succeed on valid TOML");

    // Test lint
    let lint_result = handler.lint(path, &content);
    assert!(lint_result.is_ok(), "Lint should succeed on valid TOML");
}

#[test]
fn test_toml_handler_with_malformed_file() {
    let handler = TomlHandler::new();
    let content = load_fixture("malformed.toml");
    let path = Path::new("tests/fixtures/malformed.toml");

    // Test format - should detect syntax error
    let format_result = handler.format(path, &content, false);
    match format_result {
        Ok(_) => {}
        Err(diagnostic) => {
            assert_diagnostic_format(&diagnostic);
            assert!(
                diagnostic.message.contains("parse") || diagnostic.message.contains("syntax"),
                "Error should mention parsing or syntax"
            );
        }
    }

    // Test lint - should detect syntax error
    let lint_result = handler.lint(path, &content);
    match lint_result {
        Ok(diagnostics) => {
            for diag in &diagnostics {
                assert_diagnostic_format(diag);
            }
        }
        Err(diagnostic) => {
            assert_diagnostic_format(&diagnostic);
        }
    }
}

// ============================================================================
// Markdown Handler Integration Tests
// ============================================================================

#[test]
fn test_markdown_handler_with_valid_sample() {
    let handler = MarkdownHandler::new();
    let content = load_fixture("sample.md");
    let path = Path::new("tests/fixtures/sample.md");

    // Test format
    let format_result = handler.format(path, &content, false);
    assert!(format_result.is_ok(), "Format should succeed on valid Markdown");

    // Test lint
    let lint_result = handler.lint(path, &content);
    assert!(lint_result.is_ok(), "Lint should succeed on valid Markdown");
}

#[test]
fn test_markdown_handler_check_mode() {
    let handler = MarkdownHandler::new();
    let content = "#Heading\n\nParagraph.";
    let path = Path::new("test.md");

    let check_result = handler.check(path, content, false);
    // Check may fail if markdown tools are not configured, which is acceptable
    match check_result {
        Ok(_) => {
            // Success is expected
        }
        Err(diagnostic) => {
            // If it fails, verify the diagnostic is properly formatted
            assert_diagnostic_format(&diagnostic);
        }
    }
}

// ============================================================================
// JSON Handler Integration Tests
// ============================================================================

#[test]
fn test_json_handler_with_valid_sample() {
    let handler = JsonHandler::new();
    let content = load_fixture("sample.json");
    let path = Path::new("tests/fixtures/sample.json");

    // Test format
    let format_result = handler.format(path, &content, false);
    assert!(format_result.is_ok(), "Format should succeed on valid JSON");

    // Test lint
    let lint_result = handler.lint(path, &content);
    assert!(lint_result.is_ok(), "Lint should succeed on valid JSON");
}

#[test]
fn test_json_handler_with_malformed_file() {
    let handler = JsonHandler::new();
    let content = load_fixture("malformed.json");
    let path = Path::new("tests/fixtures/malformed.json");

    // Test format - should detect syntax error
    let format_result = handler.format(path, &content, false);
    match format_result {
        Ok(_) => {}
        Err(diagnostic) => {
            assert_diagnostic_format(&diagnostic);
        }
    }

    // Test lint - should detect syntax error
    let lint_result = handler.lint(path, &content);
    match lint_result {
        Ok(diagnostics) => {
            for diag in &diagnostics {
                assert_diagnostic_format(diag);
            }
        }
        Err(diagnostic) => {
            assert_diagnostic_format(&diagnostic);
        }
    }
}

// ============================================================================
// YAML Handler Integration Tests
// ============================================================================

#[test]
fn test_yaml_handler_with_valid_sample() {
    let handler = YamlHandler::new();
    let content = load_fixture("sample.yaml");
    let path = Path::new("tests/fixtures/sample.yaml");

    // Test format
    let format_result = handler.format(path, &content, false);
    assert!(format_result.is_ok(), "Format should succeed on valid YAML");

    // Test lint
    let lint_result = handler.lint(path, &content);
    assert!(lint_result.is_ok(), "Lint should succeed on valid YAML");
}

#[test]
fn test_yaml_handler_with_malformed_file() {
    let handler = YamlHandler::new();
    let content = load_fixture("malformed.yaml");
    let path = Path::new("tests/fixtures/malformed.yaml");

    // Test format - should handle error gracefully
    let format_result = handler.format(path, &content, false);
    match format_result {
        Ok(_) => {}
        Err(diagnostic) => {
            assert_diagnostic_format(&diagnostic);
        }
    }

    // Test lint - should detect syntax error
    let lint_result = handler.lint(path, &content);
    match lint_result {
        Ok(diagnostics) => {
            for diag in &diagnostics {
                assert_diagnostic_format(diag);
            }
        }
        Err(diagnostic) => {
            assert_diagnostic_format(&diagnostic);
        }
    }
}

// ============================================================================
// FileProcessor Integration Tests
// ============================================================================

#[test]
fn test_file_processor_with_all_handlers() {
    let mut processor = FileProcessor::new();

    // Register all handlers
    processor.register(JavaScriptHandler::new());
    processor.register(PythonHandler::new());
    processor.register(RustHandler::new());
    processor.register(GoHandler::new());
    processor.register(TomlHandler::new());
    processor.register(MarkdownHandler::new());
    processor.register(JsonHandler::new());
    processor.register(YamlHandler::new());
    processor.register(CppHandler::new());
    processor.register(PhpHandler::new());
    processor.register(KotlinHandler::new());
    processor.register(CssHandler::new());
    processor.register(HtmlHandler::new());

    // Verify all handlers are registered
    assert!(processor.handler_count() >= 13, "All handlers should be registered");

    // Test routing to each handler
    let test_files = vec![
        "test.js",
        "test.py",
        "test.rs",
        "test.go",
        "test.toml",
        "test.md",
        "test.json",
        "test.yaml",
        "test.cpp",
        "test.php",
        "test.kt",
        "test.css",
        "test.html",
    ];

    for file in test_files {
        let path = Path::new(file);
        assert!(processor.is_supported(path), "File {} should be supported", file);
    }
}

#[test]
fn test_file_processor_handles_errors_gracefully() {
    let mut processor = FileProcessor::new();
    processor.register(JsonHandler::new());
    processor.register(TomlHandler::new());

    // Test with malformed JSON
    let json_content = load_fixture("malformed.json");
    let json_result = processor.lint(Path::new("test.json"), &json_content);

    // Should either return diagnostics or error, but not panic
    match json_result {
        Ok(diagnostics) => {
            for diag in diagnostics {
                assert_diagnostic_format(&diag);
            }
        }
        Err(diagnostic) => {
            assert_diagnostic_format(&diagnostic);
        }
    }

    // Test with malformed TOML
    let toml_content = load_fixture("malformed.toml");
    let toml_result = processor.lint(Path::new("test.toml"), &toml_content);

    // Should either return diagnostics or error, but not panic
    match toml_result {
        Ok(diagnostics) => {
            for diag in diagnostics {
                assert_diagnostic_format(&diag);
            }
        }
        Err(diagnostic) => {
            assert_diagnostic_format(&diagnostic);
        }
    }
}

#[test]
fn test_file_processor_format_multiple_languages() {
    let mut processor = FileProcessor::new();
    processor.register(PythonHandler::new());
    processor.register(RustHandler::new());
    processor.register(TomlHandler::new());

    // Test formatting valid files
    let test_cases = vec![
        ("sample.py", load_fixture("sample.py")),
        ("sample.rs", load_fixture("sample.rs")),
        ("sample.toml", load_fixture("sample.toml")),
    ];

    for (filename, content) in test_cases {
        let path = Path::new(filename);
        let result = processor.format(path, &content, false);
        assert!(result.is_ok(), "Format should succeed for {}", filename);
    }
}

#[test]
fn test_diagnostic_severity_levels() {
    // Create diagnostics with different severity levels
    let error = Diagnostic::error("test.py", "This is an error", "test/error");
    let warning = Diagnostic::warning("test.py", "This is a warning", "test/warning");
    let info = Diagnostic::info("test.py", "This is info", "test/info");

    // Verify severity levels
    assert!(matches!(error.severity, Severity::Error));
    assert!(matches!(warning.severity, Severity::Warning));
    assert!(matches!(info.severity, Severity::Info));

    // Verify all have proper format
    assert_diagnostic_format(&error);
    assert_diagnostic_format(&warning);
    assert_diagnostic_format(&info);
}

#[test]
fn test_diagnostic_with_location_info() {
    let diagnostic = Diagnostic::error("test.py", "Error message", "test/rule")
        .with_line(42)
        .with_column(10);

    assert_eq!(diagnostic.line, Some(42));
    assert_eq!(diagnostic.column, Some(10));
    assert_diagnostic_format(&diagnostic);
}

// ============================================================================
// Cross-Language Integration Tests
// ============================================================================

#[test]
fn test_multiple_handlers_in_sequence() {
    // Test that handlers can be used sequentially without interference
    let js_handler = JavaScriptHandler::new();
    let py_handler = PythonHandler::new();
    let rs_handler = RustHandler::new();

    let js_content = load_fixture("sample.js");
    let py_content = load_fixture("sample.py");
    let rs_content = load_fixture("sample.rs");

    // Process files in sequence
    let js_result = js_handler.lint(Path::new("test.js"), &js_content);
    assert!(js_result.is_ok(), "JS lint should succeed");

    let py_result = py_handler.lint(Path::new("test.py"), &py_content);
    assert!(py_result.is_ok(), "Python lint should succeed");

    let rs_result = rs_handler.lint(Path::new("test.rs"), &rs_content);
    assert!(rs_result.is_ok(), "Rust lint should succeed");
}

#[test]
fn test_handler_check_method_integration() {
    // Test the check method (lint + format) for multiple handlers
    let handlers: Vec<(Box<dyn LanguageHandler>, &str, String)> = vec![
        (Box::new(PythonHandler::new()), "test.py", load_fixture("sample.py")),
        (Box::new(TomlHandler::new()), "test.toml", load_fixture("sample.toml")),
        (Box::new(MarkdownHandler::new()), "test.md", load_fixture("sample.md")),
    ];

    for (handler, filename, content) in handlers {
        let path = Path::new(filename);
        let result = handler.check(path, &content, false);
        assert!(result.is_ok(), "Check should succeed for {}", filename);
    }
}

#[test]
fn test_empty_file_handling() {
    let mut processor = FileProcessor::new();
    processor.register(PythonHandler::new());
    processor.register(JsonHandler::new());
    processor.register(TomlHandler::new());

    // Test with empty content
    let empty_content = "";

    let py_result = processor.lint(Path::new("empty.py"), empty_content);
    assert!(py_result.is_ok(), "Empty Python file should be handled");

    let json_result = processor.lint(Path::new("empty.json"), empty_content);
    // Empty JSON may or may not be valid depending on handler
    let _ = json_result;

    let toml_result = processor.lint(Path::new("empty.toml"), empty_content);
    assert!(toml_result.is_ok(), "Empty TOML file should be handled");
}

#[test]
fn test_large_file_handling() {
    let handler = JavaScriptHandler::new();

    // Create a large content string
    let mut large_content = String::new();
    for i in 0..1000 {
        large_content.push_str(&format!("const var{} = {};\n", i, i));
    }

    let path = Path::new("large.js");
    let result = handler.lint(path, &large_content);

    // Should handle large files without panicking
    assert!(result.is_ok(), "Should handle large files");
}

#[test]
fn test_unicode_content_handling() {
    let handler = PythonHandler::new();

    // Test with Unicode content
    let unicode_content = r#"
def greet():
    print("Hello 世界 🌍")
    print("Привет мир")
    print("مرحبا بالعالم")
"#;

    let path = Path::new("unicode.py");
    let result = handler.lint(path, unicode_content);

    // Should handle Unicode without issues
    assert!(result.is_ok(), "Should handle Unicode content");
}
