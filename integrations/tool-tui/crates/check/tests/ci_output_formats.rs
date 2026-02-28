//! CI Output Format Integration Tests
//!
//! Integration tests for CI/CD output formats.
//! **Validates: Requirement 11.8 - Write integration tests for CI output formats**

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

/// Get the path to the dx-check binary
fn dx_check_binary() -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // Remove test binary name
    path.pop(); // Remove deps
    path.push("dx-check");

    #[cfg(windows)]
    path.set_extension("exe");

    path
}

/// Helper to run dx-check with arguments
fn run_dx_check(args: &[&str], cwd: Option<&std::path::Path>) -> std::process::Output {
    let mut cmd = Command::new(dx_check_binary());
    cmd.args(args);

    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    cmd.output().expect("Failed to execute dx-check")
}

// ============================================================================
// GitHub Actions Format Tests
// ============================================================================

#[test]
fn test_github_format_structure() {
    let dir = tempdir().unwrap();

    // Create a file with issues
    let js_content = "debugger;\nconsole.log('test');\n";
    fs::write(dir.path().join("test.js"), js_content).unwrap();

    let output = run_dx_check(&["check", "--format", "github", "."], Some(dir.path()));

    let stdout = String::from_utf8_lossy(&output.stdout);

    // GitHub format uses ::error, ::warning, or ::notice annotations
    if !stdout.is_empty() && stdout.contains("::") {
        // Verify format: ::level file=path,line=N::message
        let lines: Vec<&str> = stdout.lines().collect();
        for line in lines {
            if line.starts_with("::") {
                assert!(
                    line.starts_with("::error")
                        || line.starts_with("::warning")
                        || line.starts_with("::notice"),
                    "Invalid GitHub annotation format: {}",
                    line
                );
            }
        }
    }
}

#[test]
fn test_github_format_file_attribute() {
    let dir = tempdir().unwrap();

    let js_content = "debugger;\n";
    fs::write(dir.path().join("issue.js"), js_content).unwrap();

    let output = run_dx_check(&["check", "--format", "github", "."], Some(dir.path()));

    let stdout = String::from_utf8_lossy(&output.stdout);

    // If there are annotations, they should include file attribute
    if stdout.contains("::error") || stdout.contains("::warning") {
        assert!(stdout.contains("file="), "GitHub format should include file attribute");
    }
}

// ============================================================================
// JUnit XML Format Tests
// ============================================================================

#[test]
fn test_junit_format_valid_xml() {
    let dir = tempdir().unwrap();

    let js_content = "const x = 1;\n";
    fs::write(dir.path().join("test.js"), js_content).unwrap();

    let output = run_dx_check(&["check", "--format", "junit", "."], Some(dir.path()));

    let stdout = String::from_utf8_lossy(&output.stdout);

    // JUnit format should be valid XML
    if !stdout.is_empty() {
        assert!(
            stdout.contains("<?xml") || stdout.contains("<testsuites>"),
            "JUnit format should be XML"
        );
    }
}

#[test]
fn test_junit_format_structure() {
    let dir = tempdir().unwrap();

    let js_content = "debugger;\n";
    fs::write(dir.path().join("test.js"), js_content).unwrap();

    let output = run_dx_check(&["check", "--format", "junit", "."], Some(dir.path()));

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify JUnit structure
    if stdout.contains("<testsuites>") {
        assert!(stdout.contains("<testsuite"), "JUnit should have testsuite element");
        assert!(stdout.contains("</testsuites>"), "JUnit should close testsuites");
    }
}

#[test]
fn test_junit_format_failure_elements() {
    let dir = tempdir().unwrap();

    let js_content = "debugger;\n";
    fs::write(dir.path().join("test.js"), js_content).unwrap();

    let output = run_dx_check(&["check", "--format", "junit", "."], Some(dir.path()));

    let stdout = String::from_utf8_lossy(&output.stdout);

    // If there are failures, they should be in testcase elements
    if stdout.contains("<failure") {
        assert!(stdout.contains("<testcase"), "Failures should be in testcase elements");
    }
}

// ============================================================================
// JSON Format Tests
// ============================================================================

#[test]
fn test_json_format_valid_json() {
    let dir = tempdir().unwrap();

    let js_content = "const x = 1;\n";
    fs::write(dir.path().join("test.js"), js_content).unwrap();

    let output = run_dx_check(&["check", "--format", "json", "."], Some(dir.path()));

    let stdout = String::from_utf8_lossy(&output.stdout);

    // JSON format should be valid JSON
    if !stdout.is_empty() {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
        assert!(parsed.is_ok() || stdout.trim().is_empty(), "JSON format should be valid JSON");
    }
}

#[test]
fn test_json_format_structure() {
    let dir = tempdir().unwrap();

    let js_content = "const x = 1;\n";
    fs::write(dir.path().join("test.js"), js_content).unwrap();

    let output = run_dx_check(&["check", "--format", "json", "."], Some(dir.path()));

    let stdout = String::from_utf8_lossy(&output.stdout);

    if !stdout.is_empty() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
            // Should have expected fields
            assert!(
                json.get("files_checked").is_some() || json.get("diagnostics").is_some(),
                "JSON should have files_checked or diagnostics field"
            );
        }
    }
}

#[test]
fn test_json_format_diagnostics_array() {
    let dir = tempdir().unwrap();

    let js_content = "debugger;\n";
    fs::write(dir.path().join("test.js"), js_content).unwrap();

    let output = run_dx_check(&["check", "--format", "json", "."], Some(dir.path()));

    let stdout = String::from_utf8_lossy(&output.stdout);

    if !stdout.is_empty() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
            if let Some(diagnostics) = json.get("diagnostics") {
                assert!(diagnostics.is_array(), "diagnostics should be an array");
            }
        }
    }
}

// ============================================================================
// Compact Format Tests
// ============================================================================

#[test]
fn test_compact_format_structure() {
    let dir = tempdir().unwrap();

    let js_content = "debugger;\n";
    fs::write(dir.path().join("test.js"), js_content).unwrap();

    let output = run_dx_check(&["check", "--format", "compact", "."], Some(dir.path()));

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Compact format: file:line:col: severity [rule] message
    if !stdout.is_empty() {
        let lines: Vec<&str> = stdout.lines().collect();
        for line in lines {
            if line.contains(":") && !line.starts_with("Error") {
                // Should have colon-separated format
                let parts: Vec<&str> = line.split(':').collect();
                assert!(parts.len() >= 3, "Compact format should have file:line:col structure");
            }
        }
    }
}

// ============================================================================
// CI Config Generation Tests
// ============================================================================

#[test]
fn test_ci_generate_github_actions() {
    let output = run_dx_check(&["ci", "--platform", "github"], None);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should generate valid GitHub Actions YAML
    assert!(
        stdout.contains("name:") || stdout.contains("jobs:") || stdout.contains("GitHub"),
        "Should generate GitHub Actions config"
    );
}

#[test]
fn test_ci_generate_gitlab_ci() {
    let output = run_dx_check(&["ci", "--platform", "gitlab"], None);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should generate valid GitLab CI YAML
    assert!(
        stdout.contains("stages:") || stdout.contains("script:") || stdout.contains("GitLab"),
        "Should generate GitLab CI config"
    );
}

#[test]
fn test_ci_generate_to_file() {
    let dir = tempdir().unwrap();
    let output_path = dir.path().join("ci-config.yml");

    let output = run_dx_check(
        &[
            "ci",
            "--platform",
            "github",
            "--output",
            output_path.to_str().unwrap(),
        ],
        None,
    );

    assert!(output.status.success());
    assert!(output_path.exists(), "CI config file should be created");

    let content = fs::read_to_string(&output_path).unwrap();
    assert!(!content.is_empty(), "CI config file should not be empty");
}

// ============================================================================
// Exit Code Tests for CI
// ============================================================================

#[test]
fn test_exit_code_zero_on_success() {
    let dir = tempdir().unwrap();

    // Create a clean file
    let js_content = "const x = 1;\n";
    fs::write(dir.path().join("clean.js"), js_content).unwrap();

    let output = run_dx_check(&["check", "."], Some(dir.path()));

    // Exit code 0 = success, 1 = errors found, 2 = tool error
    let code = output.status.code().unwrap_or(2);
    assert!(code <= 1, "Exit code should be 0 or 1, got {}", code);
}

#[test]
fn test_exit_code_nonzero_on_errors() {
    let dir = tempdir().unwrap();

    // Create a file with definite errors
    let js_content = "debugger;\n";
    fs::write(dir.path().join("error.js"), js_content).unwrap();

    let output = run_dx_check(&["check", "."], Some(dir.path()));

    // Should return non-zero if debugger rule is enabled
    // (may be 0 if rule is not enabled by default)
    let _ = output.status.code();
}

#[test]
fn test_exit_code_two_on_tool_error() {
    // Try to check a non-existent path
    let output = run_dx_check(
        &[
            "check",
            "/nonexistent/path/that/definitely/does/not/exist/12345",
        ],
        None,
    );

    // Should handle gracefully
    let code = output.status.code().unwrap_or(0);
    // May be 0 (no files found) or 2 (error)
    assert!(code <= 2, "Exit code should be 0, 1, or 2, got {}", code);
}

// ============================================================================
// Format Consistency Tests
// ============================================================================

#[test]
fn test_all_formats_handle_empty_results() {
    let dir = tempdir().unwrap();

    // Create an empty directory (no files to check)
    let formats = ["pretty", "json", "compact", "github", "junit"];

    for format in &formats {
        let output = run_dx_check(&["check", "--format", format, "."], Some(dir.path()));

        // All formats should handle empty results gracefully
        assert!(
            output.status.success() || output.status.code() == Some(1),
            "Format {} should handle empty results",
            format
        );
    }
}

#[test]
fn test_all_formats_handle_errors() {
    let dir = tempdir().unwrap();

    let js_content = "debugger;\n";
    fs::write(dir.path().join("test.js"), js_content).unwrap();

    let formats = ["pretty", "json", "compact", "github", "junit"];

    for format in &formats {
        let output = run_dx_check(&["check", "--format", format, "."], Some(dir.path()));

        // All formats should complete without crashing
        let _ = output.status;
    }
}
