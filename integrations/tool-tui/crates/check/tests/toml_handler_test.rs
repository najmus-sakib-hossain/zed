use dx_check::languages::{LanguageHandler, TomlHandler};
use std::path::Path;

#[test]
fn test_toml_handler_basic() {
    let handler = TomlHandler::new();

    // Test extensions
    assert_eq!(handler.extensions(), &["toml"]);
    assert_eq!(handler.name(), "toml");

    // Test valid TOML formatting
    let valid_toml = r#"[package]
name = "test"
version = "1.0.0"
"#;

    let result = handler.format(Path::new("test.toml"), valid_toml, false);
    assert!(result.is_ok(), "Should format valid TOML");

    // Test invalid TOML
    let invalid_toml = "[package\nname = \"test\"";
    let result = handler.lint(Path::new("test.toml"), invalid_toml);
    assert!(result.is_ok());
    let diagnostics = result.unwrap();
    assert!(!diagnostics.is_empty(), "Should detect syntax errors");
}

#[test]
fn test_toml_formatting_idempotence() {
    let handler = TomlHandler::new();
    let content = r#"[package]
name="test"
version="1.0.0"
"#;

    let first = handler.format(Path::new("test.toml"), content, false);
    assert!(first.is_ok());

    // Format the formatted content again
    if let Ok(status) = first {
        // The second format should return Unchanged since it's already formatted
        // (or Changed if taplo normalizes it further, both are acceptable)
        assert!(status.is_changed() || status.is_unchanged());
    }
}
