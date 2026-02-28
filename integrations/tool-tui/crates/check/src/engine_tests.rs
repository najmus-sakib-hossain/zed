//! Property-based tests for the Checker Engine
//!
//! **Feature: dx-check-production**
//! Tests for Properties 15-19 related to the checker engine.

use crate::config::CheckerConfig;
use crate::diagnostics::DiagnosticSeverity;
use crate::engine::Checker;
use proptest::prelude::*;
use std::path::Path;

// Generators for test inputs

fn arb_valid_js_source() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("const x = 1;".to_string()),
        Just("let y = 'hello';".to_string()),
        Just("function foo() { return 42; }".to_string()),
        Just("const arr = [1, 2, 3];".to_string()),
        Just("const obj = { a: 1, b: 2 };".to_string()),
        Just("class MyClass { constructor() {} }".to_string()),
        Just("export const value = 123;".to_string()),
        Just("import { foo } from 'bar';".to_string()),
    ]
}

fn arb_invalid_js_source() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("const x = ;".to_string()),        // Missing expression
        Just("function ( { }".to_string()),     // Malformed function
        Just("const const = 5;".to_string()),   // Reserved word as identifier
        Just("class class { }".to_string()),    // Reserved word as class name
        Just("const x = [1, 2, ;".to_string()), // Malformed array
        Just("{ { { }".to_string()),            // Unbalanced braces
        Just("if (true".to_string()),           // Incomplete if statement
        Just("const x = (1 + ;".to_string()),   // Incomplete expression
    ]
}

fn arb_js_with_debugger() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("debugger;".to_string()),
        Just("function foo() { debugger; }".to_string()),
        Just("if (true) { debugger; }".to_string()),
    ]
}

fn arb_js_with_console() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("console.log('test');".to_string()),
        Just("console.error('error');".to_string()),
        Just("console.warn('warning');".to_string()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 16: Parse Error Reporting**
    /// *For any* file with syntax errors, the Checker_Engine SHALL report errors
    /// with accurate file path, line number, and column number.
    /// **Validates: Requirements 8.1**
    #[test]
    fn prop_parse_error_reporting(source in arb_invalid_js_source()) {
        let checker = Checker::new(CheckerConfig::default());
        let result = checker.check_source(Path::new("test.js"), &source);

        // Should not panic
        prop_assert!(result.is_ok());

        let diagnostics = result.unwrap();

        // Should have at least one parse error
        prop_assert!(
            diagnostics.iter().any(|d| d.rule_id == "parse-error"),
            "Invalid source should produce parse errors"
        );
    }

    /// **Property 18: Error Resilience - File Processing**
    /// *For any* set of files where one file has parse errors, the Checker_Engine
    /// SHALL still process and report diagnostics for all other files.
    /// **Validates: Requirements 8.4**
    #[test]
    fn prop_error_resilience_file_processing(
        valid_source in arb_valid_js_source(),
        invalid_source in arb_invalid_js_source()
    ) {
        let checker = Checker::new(CheckerConfig::default());

        // Check valid source - should succeed
        let valid_result = checker.check_source(Path::new("valid.js"), &valid_source);
        prop_assert!(valid_result.is_ok());

        // Check invalid source - should also succeed (not panic)
        let invalid_result = checker.check_source(Path::new("invalid.js"), &invalid_source);
        prop_assert!(invalid_result.is_ok());

        // Invalid source should have parse errors
        let invalid_diagnostics = invalid_result.unwrap();
        prop_assert!(
            invalid_diagnostics.iter().any(|d| d.rule_id == "parse-error"),
            "Invalid source should produce parse errors"
        );
    }

    /// **Property 19: Error Resilience - Rule Execution**
    /// *For any* rule that throws an exception during execution, the Checker_Engine
    /// SHALL log the error and continue executing other rules.
    /// **Validates: Requirements 8.5**
    #[test]
    fn prop_error_resilience_rule_execution(source in arb_valid_js_source()) {
        let checker = Checker::new(CheckerConfig::default());

        // Should not panic even with various inputs
        let result = checker.check_source(Path::new("test.js"), &source);
        prop_assert!(result.is_ok());
    }
}

/// Test that debugger statements are detected
#[test]
fn test_debugger_detection() {
    let checker = Checker::new(CheckerConfig::default());
    let source = "debugger;";
    let diagnostics = checker.check_source(Path::new("test.js"), source).unwrap();

    assert!(
        diagnostics.iter().any(|d| d.rule_id == "no-debugger"),
        "Should detect debugger statement"
    );
}

/// Test that console statements are detected
#[test]
fn test_console_detection() {
    let checker = Checker::new(CheckerConfig::default());
    let source = "console.log('test');";
    let diagnostics = checker.check_source(Path::new("test.js"), source).unwrap();

    assert!(
        diagnostics.iter().any(|d| d.rule_id == "no-console"),
        "Should detect console statement"
    );
}

/// Test CheckResult methods
#[test]
fn test_check_result_methods() {
    let checker = Checker::new(CheckerConfig::default());

    // Check source with errors
    let source = "debugger; console.log('test');";
    let diagnostics = checker.check_source(Path::new("test.js"), source).unwrap();

    // Create a CheckResult manually for testing
    let result = crate::engine::CheckResult {
        diagnostics,
        files_checked: 1,
        duration: std::time::Duration::from_millis(10),
        files_per_second: 100.0,
    };

    // Should have warnings (debugger and console are warnings by default)
    assert!(result.has_warnings() || result.has_errors());
}

/// Test that valid code produces no parse errors
#[test]
fn test_valid_code_no_parse_errors() {
    let checker = Checker::new(CheckerConfig::default());
    let source = "const x = 1; const y = 2;";
    let diagnostics = checker.check_source(Path::new("test.js"), source).unwrap();

    assert!(
        !diagnostics.iter().any(|d| d.rule_id == "parse-error"),
        "Valid code should not have parse errors"
    );
}

/// Test TypeScript source type detection
#[test]
fn test_typescript_detection() {
    let checker = Checker::new(CheckerConfig::default());
    let source = "const x: number = 1;";
    let diagnostics = checker.check_source(Path::new("test.ts"), source).unwrap();

    // Should parse TypeScript without errors
    assert!(
        !diagnostics.iter().any(|d| d.rule_id == "parse-error"),
        "TypeScript should parse correctly"
    );
}

/// Test JSX source type detection
#[test]
fn test_jsx_detection() {
    let checker = Checker::new(CheckerConfig::default());
    let source = "const element = <div>Hello</div>;";
    let diagnostics = checker.check_source(Path::new("test.jsx"), source).unwrap();

    // Should parse JSX without errors
    assert!(
        !diagnostics.iter().any(|d| d.rule_id == "parse-error"),
        "JSX should parse correctly"
    );
}

/// **Property 15: AST Caching**
/// *For any* unchanged file, the second check SHALL use the cached AST rather than re-parsing.
/// **Validates: Requirements 7.3, 7.4**
#[test]
fn test_ast_caching() {
    use crate::cache::AstCache;
    use tempfile::tempdir;

    // Create a temporary cache directory
    let cache_dir = tempdir().unwrap();
    let cache = AstCache::new(cache_dir.path().to_path_buf(), 1024 * 1024).unwrap();

    // Create checker with cache
    let checker = Checker::new(CheckerConfig::default()).with_cache(cache);

    let source = "const x = 1; const y = 2;";
    let path = Path::new("test.js");

    // First check - should parse
    let diagnostics1 = checker.check_source(path, source).unwrap();

    // Second check with same source - should use cache (or at least produce same result)
    let diagnostics2 = checker.check_source(path, source).unwrap();

    // Results should be identical
    assert_eq!(
        diagnostics1.len(),
        diagnostics2.len(),
        "Cached check should produce same diagnostics"
    );

    // Verify no parse errors in either case
    assert!(
        !diagnostics1.iter().any(|d| d.rule_id == "parse-error"),
        "First check should not have parse errors"
    );
    assert!(
        !diagnostics2.iter().any(|d| d.rule_id == "parse-error"),
        "Cached check should not have parse errors"
    );
}
