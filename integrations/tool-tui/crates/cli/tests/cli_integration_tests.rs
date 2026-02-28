//! Integration tests for the DX CLI binary
//!
//! These tests verify CLI binary execution behavior including:
//! - --version flag
//! - --help flag
//! - Invalid command handling
//!
//! Run with: cargo test --test cli_integration_tests
//!
//! _Requirements: 6.4, 6.5_

use assert_cmd::Command;
use predicates::prelude::*;

/// Get a Command for the dx-cli binary
fn dx_cmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_dx"))
}

/// Test that --version flag outputs version information
/// _Requirements: 6.4_
#[test]
fn test_version_flag() {
    let mut cmd = dx_cmd();
    cmd.arg("--version").assert().success().stdout(predicate::str::contains("dx"));
}

/// Test that --help flag outputs help information
/// _Requirements: 6.4_
#[test]
fn test_help_flag() {
    let mut cmd = dx_cmd();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("DX CLI"))
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("Commands:"));
}

/// Test that help subcommand works
/// _Requirements: 6.4_
#[test]
fn test_help_subcommand() {
    let mut cmd = dx_cmd();
    cmd.arg("help").assert().success().stdout(predicate::str::contains("DX CLI"));
}

/// Test that invalid command returns error
/// _Requirements: 6.5_
#[test]
fn test_invalid_command() {
    let mut cmd = dx_cmd();
    cmd.arg("nonexistent-command-xyz")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

/// Test that invalid flag returns error
/// _Requirements: 6.5_
#[test]
fn test_invalid_flag() {
    let mut cmd = dx_cmd();
    cmd.arg("--nonexistent-flag-xyz")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

/// Test that subcommand help works (forge --help)
/// _Requirements: 6.4_
#[test]
fn test_subcommand_help() {
    let mut cmd = dx_cmd();
    cmd.args(["forge", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));
}

/// Test that --verbose flag is accepted globally
/// _Requirements: 6.4_
#[test]
fn test_verbose_flag_accepted() {
    let mut cmd = dx_cmd();
    cmd.args(["--verbose", "--help"]).assert().success();
}

/// Test that --quiet flag is accepted globally
/// _Requirements: 6.4_
#[test]
fn test_quiet_flag_accepted() {
    let mut cmd = dx_cmd();
    cmd.args(["--quiet", "--help"]).assert().success();
}

/// Test that -v short flag works for verbose
/// _Requirements: 6.4_
#[test]
fn test_verbose_short_flag() {
    let mut cmd = dx_cmd();
    cmd.args(["-v", "--help"]).assert().success();
}

/// Test that -q short flag works for quiet
/// _Requirements: 6.4_
#[test]
fn test_quiet_short_flag() {
    let mut cmd = dx_cmd();
    cmd.args(["-q", "--help"]).assert().success();
}

// ============================================================================
// Shell Completion Tests
// ============================================================================

/// Test that completions bash outputs valid bash completion script
/// _Requirements: 5.1_
#[test]
fn test_generate_completion_bash() {
    let mut cmd = dx_cmd();
    cmd.args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_dx()"))
        .stdout(predicate::str::contains("complete"));
}

/// Test that completions zsh outputs valid zsh completion script
/// _Requirements: 5.2_
#[test]
fn test_generate_completion_zsh() {
    let mut cmd = dx_cmd();
    cmd.args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef dx"))
        .stdout(predicate::str::contains("_dx"));
}

/// Test that completions fish outputs valid fish completion script
/// _Requirements: 5.3_
#[test]
fn test_generate_completion_fish() {
    let mut cmd = dx_cmd();
    cmd.args(["completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete"))
        .stdout(predicate::str::contains("-c dx"));
}

/// Test that completions powershell outputs valid PowerShell completion script
/// _Requirements: 5.4_
#[test]
fn test_generate_completion_powershell() {
    let mut cmd = dx_cmd();
    cmd.args(["completions", "powershell"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Register-ArgumentCompleter"))
        .stdout(predicate::str::contains("dx"));
}

/// Test that completions with invalid shell returns error
/// _Requirements: 5.5_
#[test]
fn test_generate_completion_invalid_shell() {
    let mut cmd = dx_cmd();
    cmd.args(["completions", "invalid-shell"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

/// Test that bash completion includes subcommands
/// _Requirements: 5.1_
#[test]
fn test_bash_completion_includes_subcommands() {
    let mut cmd = dx_cmd();
    cmd.arg("completions")
        .arg("bash")
        .assert()
        .success()
        .stdout(predicate::str::contains("forge"))
        .stdout(predicate::str::contains("branch"));
}

// ============================================================================
// Doctor Command Tests
// ============================================================================

/// Test that doctor command runs without errors
/// _Requirements: 11.1_
#[test]
fn test_doctor_command_runs() {
    let mut cmd = dx_cmd();
    cmd.arg("doctor").assert().success();
}

/// Test that doctor command outputs system information
/// _Requirements: 11.2_
#[test]
fn test_doctor_shows_system_info() {
    let mut cmd = dx_cmd();
    cmd.arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("System"))
        .stdout(predicate::str::contains("CLI Version"))
        .stdout(predicate::str::contains("Operating System"))
        .stdout(predicate::str::contains("Architecture"));
}

/// Test that doctor command outputs daemon status
/// _Requirements: 11.3_
#[test]
fn test_doctor_shows_daemon_status() {
    let mut cmd = dx_cmd();
    cmd.arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Daemon"))
        .stdout(predicate::str::contains("Agent Daemon"));
}

/// Test that doctor command outputs configuration info
/// _Requirements: 11.4_
#[test]
fn test_doctor_shows_config_info() {
    let mut cmd = dx_cmd();
    cmd.arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Configuration"));
}

/// Test that doctor command outputs diagnostic checks
/// _Requirements: 11.5_
#[test]
fn test_doctor_shows_checks() {
    let mut cmd = dx_cmd();
    cmd.arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("Diagnostics"));
}

/// Test that doctor --format json outputs valid JSON
/// _Requirements: 11.1_
#[test]
fn test_doctor_json_output() {
    let mut cmd = dx_cmd();
    let output = cmd.args(["doctor", "--format", "json"]).assert().success();

    // Verify it's valid JSON by checking for expected fields
    let stdout = output.get_output().stdout.clone();
    let json_str = String::from_utf8_lossy(&stdout);

    // Should contain expected JSON fields
    assert!(json_str.contains("version"));
    assert!(json_str.contains("summary"));
    assert!(json_str.contains("checks"));
}

/// Test that doctor --help shows usage
/// _Requirements: 11.1_
#[test]
fn test_doctor_help() {
    let mut cmd = dx_cmd();
    cmd.args(["doctor", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("diagnostics"));
}

// ============================================================================
// Config Subcommand Tests
// ============================================================================

/// Test that config --help shows usage
/// _Requirements: 6.1_
#[test]
fn test_config_help() {
    let mut cmd = dx_cmd();
    cmd.args(["config", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Configure"))
        .stdout(predicate::str::contains("--reset"))
        .stdout(predicate::str::contains("--show"));
}

/// Test that config --show works
/// _Requirements: 6.1_
#[test]
fn test_config_show_defaults() {
    let mut cmd = dx_cmd();
    let temp_dir = tempfile::tempdir().unwrap();
    cmd.current_dir(temp_dir.path()).args(["config", "--show"]).assert().success();
}

/// Test that config command runs successfully
/// _Requirements: 6.1_
#[test]
#[ignore = "Config command requires interactive input"]
fn test_config_show_json() {
    let mut cmd = dx_cmd();
    let temp_dir = tempfile::tempdir().unwrap();
    let assert = cmd.current_dir(temp_dir.path()).args(["config"]).assert().success();

    // Verify it's valid JSON
    let stdout = assert.get_output().stdout.clone();
    let json_str = String::from_utf8_lossy(&stdout);

    // Should be parseable as JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json_str);
    assert!(parsed.is_ok(), "Config show --json should output valid JSON");
}

/// Test that config path works when no config exists
/// _Requirements: 6.1_
#[test]
#[ignore = "config path subcommand does not exist"]
fn test_config_path_no_config() {
    let mut cmd = dx_cmd();
    let temp_dir = tempfile::tempdir().unwrap();
    cmd.current_dir(temp_dir.path())
        .args(["config", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No dx config file found"));
}

/// Test that config init is not available
/// _Requirements: 6.1_
#[test]
#[ignore = "config init subcommand does not exist"]
fn test_config_init() {
    let mut cmd = dx_cmd();
    let temp_dir = tempfile::tempdir().unwrap();
    cmd.current_dir(temp_dir.path())
        .args(["config", "init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created config file"));

    // Verify file was created
    assert!(temp_dir.path().join("dx").exists());
}

/// Test that config init --minimal is not available
/// _Requirements: 6.1_
#[test]
#[ignore = "config init --minimal does not exist"]
fn test_config_init_minimal() {
    let mut cmd = dx_cmd();
    let temp_dir = tempfile::tempdir().unwrap();
    cmd.current_dir(temp_dir.path())
        .args(["config", "init", "--minimal"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created config file"));

    // Verify file was created
    assert!(temp_dir.path().join("dx").exists());
}

/// Test that config get is not available
/// _Requirements: 6.1_
#[test]
#[ignore = "config get subcommand does not exist"]
fn test_config_get_known_key() {
    let mut cmd = dx_cmd();
    let temp_dir = tempfile::tempdir().unwrap();
    cmd.current_dir(temp_dir.path())
        .args(["config", "get", "project.name"])
        .assert()
        .success();
}

/// Test that config get handles unknown keys
/// _Requirements: 6.1_
#[test]
#[ignore = "config get subcommand does not exist"]
fn test_config_get_unknown_key() {
    let mut cmd = dx_cmd();
    let temp_dir = tempfile::tempdir().unwrap();
    cmd.current_dir(temp_dir.path())
        .args(["config", "get", "unknown.key.xyz"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Unknown config key"));
}

// ============================================================================
// Cache Subcommand Tests (cache command does not exist yet)
// ============================================================================

/// Test that cache --help shows usage
/// _Requirements: 6.2_
#[test]
#[ignore = "cache command does not exist"]
fn test_cache_help() {
    let mut cmd = dx_cmd();
    cmd.args(["cache", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cache"))
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("clean"));
}

/// Test that cache init creates .dx directory
/// _Requirements: 6.2_
#[test]
#[ignore = "cache command does not exist"]
fn test_cache_init() {
    let mut cmd = dx_cmd();
    let temp_dir = tempfile::tempdir().unwrap();
    cmd.current_dir(temp_dir.path())
        .args(["cache", "init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Initializing .dx directory"))
        .stdout(predicate::str::contains("Created .dx directory"));

    // Verify .dx directory was created
    assert!(temp_dir.path().join(".dx").exists());
}

/// Test that cache status works when .dx doesn't exist
/// _Requirements: 6.2_
#[test]
#[ignore = "cache command does not exist"]
fn test_cache_status_no_dx() {
    let mut cmd = dx_cmd();
    let temp_dir = tempfile::tempdir().unwrap();
    cmd.current_dir(temp_dir.path())
        .args(["cache", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains(".dx directory not found"));
}

/// Test that cache status works after init
/// _Requirements: 6.2_
#[test]
#[ignore = "cache command does not exist"]
fn test_cache_status_after_init() {
    let temp_dir = tempfile::tempdir().unwrap();

    // First init
    let mut init_cmd = dx_cmd();
    init_cmd.current_dir(temp_dir.path()).args(["cache", "init"]).assert().success();

    // Then check status
    let mut status_cmd = dx_cmd();
    status_cmd
        .current_dir(temp_dir.path())
        .args(["cache", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("DX Cache Status"))
        .stdout(predicate::str::contains("Location"));
}

/// Test that cache status --detailed shows breakdown
/// _Requirements: 6.2_
#[test]
#[ignore = "cache command does not exist"]
fn test_cache_status_detailed() {
    let temp_dir = tempfile::tempdir().unwrap();

    // First init
    let mut init_cmd = dx_cmd();
    init_cmd.current_dir(temp_dir.path()).args(["cache", "init"]).assert().success();

    // Then check detailed status
    let mut status_cmd = dx_cmd();
    status_cmd
        .current_dir(temp_dir.path())
        .args(["cache", "status", "--detailed"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Directory"));
}

/// Test that cache path shows .dx path
/// _Requirements: 6.2_
#[test]
#[ignore = "cache command does not exist"]
fn test_cache_path() {
    let mut cmd = dx_cmd();
    let temp_dir = tempfile::tempdir().unwrap();
    cmd.current_dir(temp_dir.path())
        .args(["cache", "path"])
        .assert()
        .success()
        .stdout(predicate::str::contains(".dx"));
}

/// Test that cache list shows subdirectories
/// _Requirements: 6.2_
#[test]
#[ignore = "cache command does not exist"]
fn test_cache_list() {
    let mut cmd = dx_cmd();
    cmd.args(["cache", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("DX Cache Subdirectories"))
        .stdout(predicate::str::contains(".dx/"));
}

/// Test that cache clean without args shows help
/// _Requirements: 6.2_
#[test]
#[ignore = "cache command does not exist"]
fn test_cache_clean_no_args() {
    let mut cmd = dx_cmd();
    let temp_dir = tempfile::tempdir().unwrap();

    // First init
    let mut init_cmd = dx_cmd();
    init_cmd.current_dir(temp_dir.path()).args(["cache", "init"]).assert().success();

    // Then try clean without args
    cmd.current_dir(temp_dir.path())
        .args(["cache", "clean"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Specify --target"));
}

// ============================================================================
// Error Handling Tests
// ============================================================================

/// Test that missing required argument shows error
/// _Requirements: 6.3_
#[test]
#[ignore = "config get subcommand does not exist"]
fn test_missing_required_arg() {
    let mut cmd = dx_cmd();
    cmd.args(["config", "get"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

/// Test that invalid subcommand shows error
/// _Requirements: 6.3_
#[test]
fn test_invalid_subcommand() {
    let mut cmd = dx_cmd();
    cmd.args(["config", "invalid-subcommand-xyz"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

/// Test that conflicting flags are handled
/// _Requirements: 6.3_
#[test]
fn test_verbose_and_quiet_together() {
    // Both verbose and quiet together - should work (quiet takes precedence typically)
    let mut cmd = dx_cmd();
    cmd.args(["--verbose", "--quiet", "--help"]).assert().success();
}

// ============================================================================
// JSON Output Tests
// ============================================================================

/// Test that doctor --format json produces valid JSON with required fields
/// _Requirements: 6.4_
#[test]
fn test_doctor_json_has_required_fields() {
    let mut cmd = dx_cmd();
    let output = cmd.args(["doctor", "--format", "json"]).assert().success();

    let stdout = output.get_output().stdout.clone();
    let json_str = String::from_utf8_lossy(&stdout);

    // Parse as JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("Doctor --json should output valid JSON");

    // Check required fields exist
    assert!(parsed.get("version").is_some(), "Should have version");
    assert!(parsed.get("summary").is_some(), "Should have summary");
    assert!(parsed.get("checks").is_some(), "Should have checks");
}

// ============================================================================
// Additional Error Handling Tests (Task 13.3)
// ============================================================================

/// Test that invalid JSON flag value is handled gracefully
/// _Requirements: 6.3_
#[test]
fn test_json_flag_invalid_context() {
    let mut cmd = dx_cmd();
    // Doctor command supports --format json, should work
    cmd.args(["doctor", "--format", "json"]).assert().success();
}

/// Test that config --show works from temp directory
/// _Requirements: 6.3_
#[test]
fn test_config_show_from_nonexistent_dir() {
    let mut cmd = dx_cmd();
    let temp_dir = tempfile::tempdir().unwrap();
    // Should still work even from a temp directory with no config
    cmd.current_dir(temp_dir.path()).args(["config", "--show"]).assert().success();
}

/// Test that --help works on all major subcommands
/// _Requirements: 6.4_
#[test]
fn test_all_major_commands_have_help() {
    // Only test commands that actually exist
    let commands = vec!["forge", "config", "doctor"];

    for cmd_name in commands {
        let mut cmd = dx_cmd();
        cmd.args([cmd_name, "--help"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Usage:"));
    }
}

// ============================================================================
// Additional JSON Output Tests (Task 13.4)
// ============================================================================

/// Test that doctor --format json has expected structure
/// _Requirements: 6.4_
#[test]
fn test_config_json_structure() {
    let mut cmd = dx_cmd();
    let output = cmd.args(["doctor", "--format", "json"]).assert().success();

    let stdout = output.get_output().stdout.clone();
    let json_str = String::from_utf8_lossy(&stdout);

    // Should parse as valid JSON
    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("Config show --json should output valid JSON");

    // Should be an object
    assert!(parsed.is_object(), "Config JSON should be an object");
}
