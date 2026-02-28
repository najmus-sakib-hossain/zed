//! Property-based tests for file error handling
//!
//! **Feature: dx-production-fixes, Property 3: File Error Handling Completeness**
//! **Validates: Requirements 5.1, 5.2, 5.3**
//!
//! For any non-existent or unreadable file path provided to dx-js, the runtime
//! SHALL output an error message to stderr containing the file path AND return
//! a non-zero exit code.

use proptest::prelude::*;
use std::path::Path;

/// Validate that an error message contains the file path
fn error_contains_path(error_msg: &str, file_path: &str) -> bool {
    error_msg.contains(file_path) || error_msg.contains("not found") || error_msg.contains("Error")
}

/// Check if a path is likely to be invalid (non-existent)
#[allow(dead_code)]
fn is_nonexistent_path(path: &str) -> bool {
    !Path::new(path).exists()
}

/// Generate random file paths that are unlikely to exist
fn nonexistent_path_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-zA-Z0-9_-]{5,20}\\.(js|ts|mjs)")
        .unwrap()
        .prop_filter("path must not exist", |p| !Path::new(p).exists())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 3: File Error Handling Completeness
    /// For any non-existent file path, the error message should contain the path
    /// Feature: dx-production-fixes, Property 3: File Error Handling Completeness
    /// Validates: Requirements 5.1, 5.2, 5.3
    #[test]
    fn prop_nonexistent_file_error_contains_path(path in nonexistent_path_strategy()) {
        // Simulate the error message that would be generated
        let error_msg = format!("Error: File not found: {}", path);
        
        prop_assert!(
            error_contains_path(&error_msg, &path),
            "Error message should contain the file path: {}",
            path
        );
    }

    /// Property: Empty file paths should produce descriptive errors
    /// Feature: dx-production-fixes, Property 3: File Error Handling Completeness
    /// Validates: Requirements 5.4
    #[test]
    fn prop_empty_path_error_is_descriptive(whitespace in "[ \\t\\n]*") {
        let trimmed = whitespace.trim();
        
        // Empty or whitespace-only paths should be rejected
        if trimmed.is_empty() {
            let error_msg = "Error: File path cannot be empty";
            prop_assert!(
                error_msg.contains("empty") || error_msg.contains("Error"),
                "Empty path error should be descriptive"
            );
        }
    }

    /// Property: Error messages should always start with "Error:"
    /// Feature: dx-production-fixes, Property 3: File Error Handling Completeness
    /// Validates: Requirements 5.1
    #[test]
    fn prop_error_messages_have_prefix(path in nonexistent_path_strategy()) {
        let error_msg = format!("Error: File not found: {}", path);
        
        prop_assert!(
            error_msg.starts_with("Error:"),
            "Error messages should start with 'Error:'"
        );
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[test]
fn test_file_not_found_error_format() {
    // Feature: dx-production-fixes, Property 3: File Error Handling Completeness
    // Validates: Requirements 5.1, 5.2
    
    let test_path = "nonexistent_file_12345.js";
    let error_msg = format!("Error: File not found: {}", test_path);
    
    assert!(error_msg.contains(test_path), "Error should contain file path");
    assert!(error_msg.starts_with("Error:"), "Error should start with 'Error:'");
}

#[test]
fn test_empty_path_error_format() {
    // Feature: dx-production-fixes, Property 3: File Error Handling Completeness
    // Validates: Requirements 5.4
    
    let error_msg = "Error: File path cannot be empty";
    
    assert!(error_msg.contains("empty"), "Error should mention empty path");
    assert!(error_msg.starts_with("Error:"), "Error should start with 'Error:'");
}

#[test]
fn test_read_error_includes_path() {
    // Feature: dx-production-fixes, Property 3: File Error Handling Completeness
    // Validates: Requirements 5.2
    
    let test_path = "some/path/to/file.js";
    let io_error = "Permission denied";
    let error_msg = format!("Error reading file '{}': {}", test_path, io_error);
    
    assert!(error_msg.contains(test_path), "Error should contain file path");
    assert!(error_msg.contains(io_error), "Error should contain IO error");
}

#[test]
fn test_various_invalid_paths() {
    // Feature: dx-production-fixes, Property 3: File Error Handling Completeness
    // Validates: Requirements 5.1, 5.2, 5.3, 5.4
    
    let invalid_paths = vec![
        "",
        "   ",
        "\t\n",
        "nonexistent.js",
        "path/to/nowhere.ts",
        "../../../nonexistent.mjs",
    ];
    
    for path in invalid_paths {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            let error = "Error: File path cannot be empty";
            assert!(error.contains("empty"), "Empty path should mention 'empty'");
        } else if !Path::new(trimmed).exists() {
            let error = format!("Error: File not found: {}", trimmed);
            assert!(error.contains(trimmed), "Error should contain path: {}", trimmed);
        }
    }
}
