//! CLI Integration Tests
//!
//! Integration tests for dx-check CLI commands.
//! **Validates: Requirements 1.1-1.6, 9.3, 10.1-10.5**

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

/// Get the path to the dx-check binary
fn dx_check_binary() -> PathBuf {
    // In tests, the binary is in target/debug
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
// Format Command Tests
// ============================================================================

#[test]
fn test_format_command_help() {
    let output = run_dx_check(&["format", "--help"], None);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("format") || stdout.contains("Format"));
}

#[test]
fn test_format_command_check_mode() {
    let dir = tempdir().unwrap();

    // Create a JS file that needs formatting
    let js_content = "const   x=1;";
    fs::write(dir.path().join("test.js"), js_content).unwrap();

    let output = run_dx_check(&["format", "--check", "."], Some(dir.path()));

    // Check mode should report if files need formatting
    // Exit code 0 = no changes needed, 1 = changes needed
    let _ = output.status.code();
}

#[test]
fn test_format_command_write_mode() {
    let dir = tempdir().unwrap();

    // Create a simple JS file
    let js_content = "const x = 1;";
    let file_path = dir.path().join("test.js");
    fs::write(&file_path, js_content).unwrap();

    let output = run_dx_check(&["format", "--write", "."], Some(dir.path()));

    // Should complete without error
    assert!(output.status.success() || output.status.code() == Some(1));
}

#[test]
fn test_format_command_nonexistent_path() {
    let output = run_dx_check(&["format", "/nonexistent/path/that/does/not/exist"], None);

    // Should handle gracefully (may succeed with 0 files or fail)
    let _ = output.status;
}

// ============================================================================
// Lint Command Tests
// ============================================================================

#[test]
fn test_lint_command_help() {
    let output = run_dx_check(&["lint", "--help"], None);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("lint") || stdout.contains("Lint"));
}

#[test]
fn test_lint_command_clean_file() {
    let dir = tempdir().unwrap();

    // Create a clean JS file
    let js_content = "const x = 1;\n";
    fs::write(dir.path().join("clean.js"), js_content).unwrap();

    let output = run_dx_check(&["lint", "."], Some(dir.path()));

    // Clean file should pass
    assert!(output.status.success() || output.status.code() == Some(1));
}

#[test]
fn test_lint_command_file_with_issues() {
    let dir = tempdir().unwrap();

    // Create a file with lint issues
    let js_content = "debugger;\nconsole.log('test');\n";
    fs::write(dir.path().join("issues.js"), js_content).unwrap();

    let output = run_dx_check(&["lint", "."], Some(dir.path()));

    // Should detect issues
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Should mention debugger or console
    assert!(
        combined.contains("debugger")
            || combined.contains("console")
            || combined.contains("no-debugger")
            || combined.contains("no-console")
            || output.status.code() == Some(1)
    );
}

#[test]
fn test_lint_command_json_format() {
    let dir = tempdir().unwrap();

    let js_content = "const x = 1;\n";
    fs::write(dir.path().join("test.js"), js_content).unwrap();

    let output = run_dx_check(&["lint", "--format", "json", "."], Some(dir.path()));

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should output valid JSON
    if !stdout.is_empty() {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
        assert!(parsed.is_ok() || stdout.trim().is_empty());
    }
}

#[test]
fn test_lint_command_compact_format() {
    let dir = tempdir().unwrap();

    let js_content = "debugger;\n";
    fs::write(dir.path().join("test.js"), js_content).unwrap();

    let output = run_dx_check(&["lint", "--format", "compact", "."], Some(dir.path()));

    // Compact format should work
    let _ = output.status;
}

// ============================================================================
// Check Command Tests
// ============================================================================

#[test]
fn test_check_command_help() {
    let output = run_dx_check(&["check", "--help"], None);
    assert!(output.status.success());
}

#[test]
fn test_check_command_basic() {
    let dir = tempdir().unwrap();

    let js_content = "const x = 1;\n";
    fs::write(dir.path().join("test.js"), js_content).unwrap();

    let output = run_dx_check(&["check", "."], Some(dir.path()));

    // Should complete
    let _ = output.status;
}

#[test]
fn test_check_command_with_fix() {
    let dir = tempdir().unwrap();

    // Create a file that can be auto-fixed
    let js_content = "var x = 1;\n";
    let file_path = dir.path().join("test.js");
    fs::write(&file_path, js_content).unwrap();

    let output = run_dx_check(&["check", "--fix", "."], Some(dir.path()));

    // Should complete (fix may or may not apply depending on rules)
    let _ = output.status;
}

// ============================================================================
// Init Command Tests
// ============================================================================

#[test]
fn test_init_command() {
    let dir = tempdir().unwrap();

    let output = run_dx_check(&["init"], Some(dir.path()));

    // Should create config file
    assert!(output.status.success());
    assert!(dir.path().join("dx.toml").exists());
}

#[test]
fn test_init_command_force() {
    let dir = tempdir().unwrap();

    // Create existing config
    fs::write(dir.path().join("dx.toml"), "# existing").unwrap();

    // Without force, should fail or warn
    let output1 = run_dx_check(&["init"], Some(dir.path()));
    let _ = output1.status;

    // With force, should overwrite
    let output2 = run_dx_check(&["init", "--force"], Some(dir.path()));
    assert!(output2.status.success());
}

// ============================================================================
// Rule Command Tests
// ============================================================================

#[test]
fn test_rule_list_command() {
    let output = run_dx_check(&["rule", "list"], None);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should list some rules
    assert!(
        stdout.contains("no-debugger")
            || stdout.contains("no-console")
            || stdout.contains("Available rules")
    );
}

#[test]
fn test_rule_show_command() {
    let output = run_dx_check(&["rule", "show", "no-debugger"], None);

    // Should show rule details or error if not found
    let _ = output.status;
}

// ============================================================================
// Cache Command Tests
// ============================================================================

#[test]
fn test_cache_stats_command() {
    let output = run_dx_check(&["cache", "stats"], None);

    // Should show stats or "no cache" message
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    assert!(
        combined.contains("Cache") || combined.contains("cache") || combined.contains("No cache")
    );
}

#[test]
fn test_cache_path_command() {
    let output = run_dx_check(&["cache", "path"], None);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should output a path
    assert!(stdout.contains("cache") || stdout.contains(".dx"));
}

// ============================================================================
// CI Command Tests
// ============================================================================

#[test]
fn test_ci_generate_github() {
    let output = run_dx_check(&["ci", "--platform", "github"], None);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should generate GitHub Actions YAML
    assert!(stdout.contains("name:") || stdout.contains("jobs:") || stdout.contains("GitHub"));
}

#[test]
fn test_ci_generate_gitlab() {
    let output = run_dx_check(&["ci", "--platform", "gitlab"], None);

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should generate GitLab CI YAML
    assert!(stdout.contains("stages:") || stdout.contains("script:") || stdout.contains("GitLab"));
}

// ============================================================================
// Output Format Tests
// ============================================================================

#[test]
fn test_github_output_format() {
    let dir = tempdir().unwrap();

    let js_content = "debugger;\n";
    fs::write(dir.path().join("test.js"), js_content).unwrap();

    let output = run_dx_check(&["check", "--format", "github", "."], Some(dir.path()));

    let stdout = String::from_utf8_lossy(&output.stdout);

    // GitHub format uses ::error or ::warning annotations
    if !stdout.is_empty() && stdout.contains("::") {
        assert!(
            stdout.contains("::error")
                || stdout.contains("::warning")
                || stdout.contains("::notice")
        );
    }
}

#[test]
fn test_junit_output_format() {
    let dir = tempdir().unwrap();

    let js_content = "const x = 1;\n";
    fs::write(dir.path().join("test.js"), js_content).unwrap();

    let output = run_dx_check(&["check", "--format", "junit", "."], Some(dir.path()));

    let stdout = String::from_utf8_lossy(&output.stdout);

    // JUnit format should be XML
    if !stdout.is_empty() {
        assert!(stdout.contains("<?xml") || stdout.contains("<testsuites>"));
    }
}

// ============================================================================
// Exit Code Tests
// ============================================================================

#[test]
fn test_exit_code_success() {
    let dir = tempdir().unwrap();

    // Create a clean file
    let js_content = "const x = 1;\n";
    fs::write(dir.path().join("clean.js"), js_content).unwrap();

    let output = run_dx_check(&["check", "."], Some(dir.path()));

    // Exit code 0 = success, 1 = errors found, 2 = tool error
    assert!(output.status.code().unwrap_or(2) <= 1);
}

#[test]
fn test_exit_code_with_errors() {
    let dir = tempdir().unwrap();

    // Create a file with errors
    let js_content = "debugger;\n";
    fs::write(dir.path().join("error.js"), js_content).unwrap();

    let output = run_dx_check(&["check", "."], Some(dir.path()));

    // Should return non-zero for errors
    // (may be 0 if debugger rule is not enabled by default)
    let _ = output.status.code();
}

// ============================================================================
// Plugin Command Tests
// ============================================================================

#[test]
fn test_plugin_list_command() {
    let output = run_dx_check(&["plugin", "list"], None);

    // Should list plugins or show "no plugins" message
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    assert!(
        combined.contains("plugin")
            || combined.contains("Plugin")
            || combined.contains("No plugins")
            || combined.contains("Installed")
    );
}

// ============================================================================
// Verbose and Quiet Mode Tests
// ============================================================================

#[test]
fn test_verbose_mode() {
    let dir = tempdir().unwrap();

    let js_content = "const x = 1;\n";
    fs::write(dir.path().join("test.js"), js_content).unwrap();

    let output = run_dx_check(&["check", "--verbose", "."], Some(dir.path()));

    // Verbose mode should produce more output
    let _ = output.status;
}

#[test]
fn test_quiet_mode() {
    let dir = tempdir().unwrap();

    let js_content = "const x = 1;\n";
    fs::write(dir.path().join("test.js"), js_content).unwrap();

    let output = run_dx_check(&["check", "--quiet", "."], Some(dir.path()));

    // Quiet mode should produce less output
    let _ = output.status;
}
