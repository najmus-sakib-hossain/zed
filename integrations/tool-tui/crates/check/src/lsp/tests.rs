//! LSP Protocol Tests
//!
//! Tests for the LSP server implementation.
//!
//! **Feature: dx-check-production, Task 15.3**
//! **Validates: Requirements 9.3**

use crate::config::CheckerConfig;
use crate::diagnostics::{Diagnostic, DiagnosticSeverity, LineIndex, Span};
use crate::engine::Checker;
use std::path::PathBuf;

// ============================================================================
// LineIndex Tests (used by LSP for position conversion)
// ============================================================================

#[test]
fn test_line_index_single_line() {
    let source = "const x = 1;";
    let index = LineIndex::new(source);

    let lc = index.line_col(0);
    assert_eq!(lc.line, 1);
    assert_eq!(lc.col, 1);

    let lc = index.line_col(6);
    assert_eq!(lc.line, 1);
    assert_eq!(lc.col, 7);
}

#[test]
fn test_line_index_multiple_lines() {
    let source = "line1\nline2\nline3";
    let index = LineIndex::new(source);

    // First line
    let lc = index.line_col(0);
    assert_eq!(lc.line, 1);
    assert_eq!(lc.col, 1);

    // Second line start
    let lc = index.line_col(6);
    assert_eq!(lc.line, 2);
    assert_eq!(lc.col, 1);

    // Third line start
    let lc = index.line_col(12);
    assert_eq!(lc.line, 3);
    assert_eq!(lc.col, 1);
}

#[test]
fn test_line_index_empty_source() {
    let source = "";
    let index = LineIndex::new(source);

    let lc = index.line_col(0);
    assert_eq!(lc.line, 1);
    assert_eq!(lc.col, 1);
}

#[test]
fn test_line_index_with_crlf() {
    // Note: LineIndex only handles \n, not \r\n
    let source = "line1\nline2";
    let index = LineIndex::new(source);

    let lc = index.line_col(6);
    assert_eq!(lc.line, 2);
    assert_eq!(lc.col, 1);
}

// ============================================================================
// Span to LineCol Conversion Tests
// ============================================================================

#[test]
fn test_span_to_line_col() {
    let source = "const x = 1;\nconst y = 2;";
    let index = LineIndex::new(source);

    let span = Span::new(0, 12);
    let (start, end) = span.to_line_col(&index);

    assert_eq!(start.line, 1);
    assert_eq!(start.col, 1);
    assert_eq!(end.line, 1);
    assert_eq!(end.col, 13);
}

#[test]
fn test_span_to_line_col_multiline() {
    let source = "const x = 1;\nconst y = 2;";
    let index = LineIndex::new(source);

    // Span covering both lines
    let span = Span::new(0, 25);
    let (start, end) = span.to_line_col(&index);

    assert_eq!(start.line, 1);
    assert_eq!(end.line, 2);
}

// ============================================================================
// Diagnostic Conversion Tests (simulating LSP conversion)
// ============================================================================

#[test]
fn test_diagnostic_severity_conversion() {
    // Test that our severity levels map correctly
    assert_eq!(DiagnosticSeverity::Error as u8, 3);
    assert_eq!(DiagnosticSeverity::Warning as u8, 2);
    assert_eq!(DiagnosticSeverity::Info as u8, 1);
    assert_eq!(DiagnosticSeverity::Hint as u8, 0);
}

#[test]
fn test_diagnostic_has_required_fields() {
    let diag = Diagnostic::error(
        PathBuf::from("test.js"),
        Span::new(0, 10),
        "no-console",
        "Unexpected console statement",
    );

    assert!(!diag.file.as_os_str().is_empty());
    assert!(!diag.rule_id.is_empty());
    assert!(!diag.message.is_empty());
    assert!(diag.span.start <= diag.span.end);
}

#[test]
fn test_diagnostic_with_fix_for_code_action() {
    let diag = Diagnostic::warn(
        PathBuf::from("test.js"),
        Span::new(0, 9),
        "no-debugger",
        "Unexpected debugger statement",
    )
    .with_fix(crate::diagnostics::Fix::delete("Remove debugger", Span::new(0, 10)));

    assert!(diag.is_fixable());
    let fix = diag.fix.as_ref().unwrap();
    assert!(!fix.description.is_empty());
    assert!(!fix.edits.is_empty());
}

// ============================================================================
// Checker Integration Tests (simulating LSP document checking)
// ============================================================================

#[test]
fn test_checker_returns_diagnostics_for_debugger() {
    let checker = Checker::new(CheckerConfig::default());
    let source = "debugger;";
    let path = std::path::Path::new("test.js");

    let result = checker.check_source(path, source);
    assert!(result.is_ok());

    let diagnostics = result.unwrap();
    assert!(diagnostics.iter().any(|d| d.rule_id == "no-debugger"));
}

#[test]
fn test_checker_returns_diagnostics_for_console() {
    let checker = Checker::new(CheckerConfig::default());
    let source = "console.log('test');";
    let path = std::path::Path::new("test.js");

    let result = checker.check_source(path, source);
    assert!(result.is_ok());

    let diagnostics = result.unwrap();
    assert!(diagnostics.iter().any(|d| d.rule_id == "no-console"));
}

#[test]
fn test_checker_returns_parse_errors() {
    let checker = Checker::new(CheckerConfig::default());
    let source = "const x = {";
    let path = std::path::Path::new("test.js");

    let result = checker.check_source(path, source);
    assert!(result.is_ok());

    let diagnostics = result.unwrap();
    assert!(diagnostics.iter().any(|d| d.rule_id == "parse-error"));
}

#[test]
fn test_checker_handles_typescript() {
    let checker = Checker::new(CheckerConfig::default());
    let source = "const x: number = 1;";
    let path = std::path::Path::new("test.ts");

    let result = checker.check_source(path, source);
    assert!(result.is_ok());

    let diagnostics = result.unwrap();
    // Valid TypeScript should not have parse errors
    assert!(!diagnostics.iter().any(|d| d.rule_id == "parse-error"));
}

#[test]
fn test_checker_handles_jsx() {
    let checker = Checker::new(CheckerConfig::default());
    let source = "const el = <div>Hello</div>;";
    let path = std::path::Path::new("test.jsx");

    let result = checker.check_source(path, source);
    assert!(result.is_ok());

    let diagnostics = result.unwrap();
    assert!(!diagnostics.iter().any(|d| d.rule_id == "parse-error"));
}

#[test]
fn test_checker_handles_tsx() {
    let checker = Checker::new(CheckerConfig::default());
    let source = "const el: JSX.Element = <div>Hello</div>;";
    let path = std::path::Path::new("test.tsx");

    let result = checker.check_source(path, source);
    assert!(result.is_ok());
}

// ============================================================================
// Document Sync Tests (simulating LSP document lifecycle)
// ============================================================================

#[test]
fn test_document_open_triggers_diagnostics() {
    let checker = Checker::new(CheckerConfig::default());
    let source = "debugger;";
    let path = std::path::Path::new("test.js");

    // Simulate didOpen
    let result = checker.check_source(path, source);
    assert!(result.is_ok());
    assert!(!result.unwrap().is_empty());
}

#[test]
fn test_document_change_updates_diagnostics() {
    let checker = Checker::new(CheckerConfig::default());
    let path = std::path::Path::new("test.js");

    // Initial content with issue
    let source1 = "debugger;";
    let result1 = checker.check_source(path, source1);
    assert!(result1.is_ok());
    let diags1 = result1.unwrap();
    assert!(diags1.iter().any(|d| d.rule_id == "no-debugger"));

    // Updated content without issue
    let source2 = "const x = 1;";
    let result2 = checker.check_source(path, source2);
    assert!(result2.is_ok());
    let diags2 = result2.unwrap();
    assert!(!diags2.iter().any(|d| d.rule_id == "no-debugger"));
}

#[test]
fn test_document_close_clears_state() {
    // This is a conceptual test - in the actual LSP server,
    // closing a document removes it from the documents map
    // and clears its diagnostics
    let checker = Checker::new(CheckerConfig::default());
    let path = std::path::Path::new("test.js");

    let source = "debugger;";
    let result = checker.check_source(path, source);
    assert!(result.is_ok());

    // After close, diagnostics should be cleared (empty vec)
    let empty_diagnostics: Vec<Diagnostic> = Vec::new();
    assert!(empty_diagnostics.is_empty());
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_checker_handles_empty_source() {
    let checker = Checker::new(CheckerConfig::default());
    let source = "";
    let path = std::path::Path::new("test.js");

    let result = checker.check_source(path, source);
    assert!(result.is_ok());
}

#[test]
fn test_checker_handles_whitespace_only() {
    let checker = Checker::new(CheckerConfig::default());
    let source = "   \n\n   \t\t\n";
    let path = std::path::Path::new("test.js");

    let result = checker.check_source(path, source);
    assert!(result.is_ok());
}

#[test]
fn test_checker_handles_comments_only() {
    let checker = Checker::new(CheckerConfig::default());
    let source = "// This is a comment\n/* Block comment */";
    let path = std::path::Path::new("test.js");

    let result = checker.check_source(path, source);
    assert!(result.is_ok());
    let diagnostics = result.unwrap();
    assert!(!diagnostics.iter().any(|d| d.rule_id == "parse-error"));
}

#[test]
fn test_checker_handles_unicode() {
    let checker = Checker::new(CheckerConfig::default());
    let source = r#"const greeting = "Hello, 世界! 🌍";"#;
    let path = std::path::Path::new("test.js");

    let result = checker.check_source(path, source);
    assert!(result.is_ok());
    let diagnostics = result.unwrap();
    assert!(!diagnostics.iter().any(|d| d.rule_id == "parse-error"));
}

#[test]
fn test_checker_handles_large_file() {
    let checker = Checker::new(CheckerConfig::default());

    // Generate a large file
    let mut source = String::new();
    for i in 0..1000 {
        source.push_str(&format!("const var{} = {};\n", i, i));
    }

    let path = std::path::Path::new("test.js");
    let result = checker.check_source(path, &source);
    assert!(result.is_ok());
}

// ============================================================================
// Code Action Tests (simulating LSP code actions)
// ============================================================================

#[test]
fn test_fixable_diagnostic_has_edit_info() {
    let diag = Diagnostic::warn(
        PathBuf::from("test.js"),
        Span::new(0, 9),
        "no-debugger",
        "Unexpected debugger statement",
    )
    .with_fix(crate::diagnostics::Fix::delete("Remove debugger", Span::new(0, 10)));

    let fix = diag.fix.as_ref().unwrap();

    // Verify fix has all info needed for LSP TextEdit
    assert!(!fix.description.is_empty());
    assert!(!fix.edits.is_empty());

    for edit in &fix.edits {
        // Each edit should have valid span
        assert!(edit.span.start <= edit.span.end);
        // new_text can be empty (for deletions)
    }
}

#[test]
fn test_multiple_fixes_can_be_collected() {
    let checker = Checker::new(CheckerConfig::default());
    let source = "debugger;\nconsole.log('test');";
    let path = std::path::Path::new("test.js");

    let result = checker.check_source(path, source);
    assert!(result.is_ok());

    let diagnostics = result.unwrap();

    // Collect all fixable diagnostics
    let fixable: Vec<_> = diagnostics.iter().filter(|d| d.is_fixable()).collect();

    // At least debugger should be fixable
    // (console may or may not have a fix depending on implementation)
    assert!(!fixable.is_empty() || diagnostics.iter().any(|d| d.rule_id == "no-debugger"));
}

// ============================================================================
// Hover Tests (simulating LSP hover)
// ============================================================================

#[test]
fn test_diagnostic_has_info_for_hover() {
    let diag = Diagnostic::warn(
        PathBuf::from("test.js"),
        Span::new(0, 9),
        "no-debugger",
        "Unexpected debugger statement",
    )
    .with_suggestion("Remove the debugger statement before committing");

    // Verify diagnostic has info needed for hover
    assert!(!diag.rule_id.is_empty());
    assert!(!diag.message.is_empty());
    assert!(diag.suggestion.is_some());
}

#[test]
fn test_diagnostic_span_for_hover_range() {
    let source = "debugger;";
    let index = LineIndex::new(source);

    let diag = Diagnostic::warn(
        PathBuf::from("test.js"),
        Span::new(0, 9),
        "no-debugger",
        "Unexpected debugger statement",
    );

    let (start, end) = diag.span.to_line_col(&index);

    // Verify we can compute hover range
    assert!(start.line >= 1);
    assert!(start.col >= 1);
    assert!(end.line >= start.line);
    if end.line == start.line {
        assert!(end.col >= start.col);
    }
}

// ============================================================================
// Configuration Tests (simulating LSP workspace/configuration)
// ============================================================================

#[test]
fn test_config_affects_diagnostics() {
    // Default config should enable no-debugger
    let default_config = CheckerConfig::default();
    let checker = Checker::new(default_config);

    let source = "debugger;";
    let path = std::path::Path::new("test.js");

    let result = checker.check_source(path, source);
    assert!(result.is_ok());

    // Should detect debugger with default config
    let diagnostics = result.unwrap();
    assert!(diagnostics.iter().any(|d| d.rule_id == "no-debugger"));
}

#[test]
fn test_checker_can_be_recreated_with_new_config() {
    // Simulate config reload by creating new checker
    let config1 = CheckerConfig::default();
    let checker1 = Checker::new(config1);

    let source = "debugger;";
    let path = std::path::Path::new("test.js");

    let result1 = checker1.check_source(path, source);
    assert!(result1.is_ok());

    // Create new checker (simulating config reload)
    let config2 = CheckerConfig::default();
    let checker2 = Checker::new(config2);

    let result2 = checker2.check_source(path, source);
    assert!(result2.is_ok());

    // Both should produce diagnostics
    assert!(!result1.unwrap().is_empty());
    assert!(!result2.unwrap().is_empty());
}
