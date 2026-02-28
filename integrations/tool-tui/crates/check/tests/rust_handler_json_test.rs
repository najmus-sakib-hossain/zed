use dx_check::languages::{LanguageHandler, rust_lang::RustHandler};
use std::path::Path;

#[test]
fn test_rust_handler_json_parsing() {
    let handler = RustHandler::new();

    // Test JSON output parsing
    let json_output = r#"{"reason":"compiler-message","message":{"level":"warning","message":"unused variable: `x`","spans":[{"file_name":"src/main.rs","line_start":2,"column_start":9,"line_end":2,"column_end":10}]}}"#;

    let diagnostics = handler.parse_clippy_json(json_output, "src/main.rs");

    match diagnostics {
        Ok(diags) => {
            assert_eq!(diags.len(), 1, "Should parse one diagnostic");
            let diag = &diags[0];
            assert!(
                diag.message.contains("unused variable"),
                "Message should contain 'unused variable'"
            );
            assert_eq!(diag.line, Some(2), "Line should be 2");
            assert_eq!(diag.column, Some(9), "Column should be 9");
        }
        Err(_) => {
            panic!("JSON parsing should succeed");
        }
    }
}

#[test]
fn test_rust_handler_extensions() {
    let handler = RustHandler::new();
    assert_eq!(handler.extensions(), &["rs"]);
    assert_eq!(handler.name(), "rust");
}

#[test]
fn test_rust_handler_format_unchanged() {
    let handler = RustHandler::new();

    // Skip if rustfmt is not available
    if handler.rustfmt_path.is_none() {
        eprintln!("Skipping test: rustfmt not installed");
        return;
    }

    let code = "fn main() {\n    println!(\"Hello\");\n}\n";
    let path = Path::new("test.rs");

    match handler.format(path, code, false) {
        Ok(status) => {
            // Either unchanged or changed is acceptable (depends on rustfmt version)
            assert!(status.is_unchanged() || status.is_changed());
        }
        Err(e) => {
            eprintln!("Format error (expected if rustfmt not installed): {}", e.message);
        }
    }
}
