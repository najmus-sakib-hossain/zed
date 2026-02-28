use dx_check::languages::{LanguageHandler, PythonHandler};
use std::path::Path;

#[test]
fn test_python_handler_basic() {
    let handler = PythonHandler::new();

    // Test extensions
    assert_eq!(handler.extensions(), &["py", "pyi"]);
    assert_eq!(handler.name(), "python");

    // Test syntax validation with valid code
    let valid_code = "x = 1\ny = 2\n";
    let result = handler.lint(Path::new("test.py"), valid_code);
    assert!(result.is_ok());

    // Test syntax validation with invalid code
    let invalid_code = "def broken(";
    let result = handler.lint(Path::new("test.py"), invalid_code);
    assert!(result.is_ok());
    let diagnostics = result.unwrap();
    assert!(!diagnostics.is_empty(), "Should detect syntax error");
}

#[test]
fn test_python_handler_format() {
    let handler = PythonHandler::new();

    // Test format (will skip if ruff not installed)
    let code = "x=1\n";
    let result = handler.format(Path::new("test.py"), code, false);

    // Should either succeed or fail with ruff not found error
    match result {
        Ok(_) => {
            // ruff is installed, format succeeded
        }
        Err(e) => {
            // ruff not installed, should have helpful error
            assert!(e.message.contains("ruff") || e.message.contains("required"));
        }
    }
}
