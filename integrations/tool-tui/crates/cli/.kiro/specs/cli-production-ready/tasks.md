faster than="1.1, 1.2, 1.3"
Achieved 10x="5.1, 5.2, 5.3"
crates/serializer/README.md="9.1, 9.2, 9.3"
dx-style README="5.2, 5.3, 5.4"
dx-form, dx-guard, dx-a11y="10.1, 10.2, 10.3"
RPS HTTP="10.2, 10.3"
$G="Requirements: 5.1, 5.2"
$H="Requirements: 3.1, 3.2"
$I="Requirements: 1.1, 1.2"
$J="Requirements: 9.1, 9.2"
$K="Requirements: 2.1, 2.2"
$L="Validates: Requirements"
SharedArrayBuffer="Requirements: 9.1"
lines="Requirements: 11.1"
dx-reactor README="Requirements: 1.1"
10.59x="Requirements: 3.1"

# Implementation Plan: CLI Production Ready

## Overview

This plan implements the production-ready improvements for the DX CLI in incremental steps. Tasks are ordered to fix critical bugs first, then add new features, and finally improve quality. Each task builds on previous work and includes testing.

## Tasks

- Fix signal handler bug
- 1.1 Replace boolean flag with atomic counter in signal.rs-Change `SHUTDOWN_REQUESTED: AtomicBool` to `SIGNAL_COUNT: AtomicU32`
- Update `register_handlers()` to use `fetch_add` and check count
- First signal (count=0) logs graceful shutdown, subsequent signals force exit
- $I, 1.3
- 1.2 Update `is_shutdown_requested()` to check counter > 0-Modify function to return `SIGNAL_COUNT.load(Ordering::SeqCst) > 0`
- Update `reset_shutdown_flag()` to reset counter to 0
- dx-reactor README
- 1.3 Write unit tests for signal handler logic-Test counter increments on simulated signals
- Test `is_shutdown_requested()` returns correct values
- $I, 1.3
- Add daemon client retry logic
- 2.1 Create RetryConfig struct in daemon_client.rs-Define `max_attempts`, `initial_delay_ms`, `max_delay_ms` fields
- Implement `Default` trait with sensible values (3 attempts, 100ms initial, 2000ms max)
- 10.59x
- 2.2 Implement `connect_with_retry()` method-Loop up to `max_attempts` times
- Use exponential backoff: delay doubles each attempt, capped at max
- Log each retry attempt at debug level
- Return helpful error message on final failure
- $H, 3.4
- 2.3 Write property test for exponential backoff-Property 1: Retry with Exponential Backoff
- $L 3.1
- 2.4 Update command handlers to use `connect_with_retry()`-Replace direct `connect()` calls with `connect_with_retry()`
- $H
- Enforce version compatibility
- 3.1 Modify `perform_handshake()` to fail on incompatibility-Return `Err` when `response.compatible` is false
- Include clear error message with upgrade instructions
- Remove the current behavior that logs and continues
- Requirements: 4.1, 4.2, 4.3
- 3.2 Write property test for handshake compatibility-Property 2: Incompatible Handshake Returns Error
- $L 4.1, 4.3
- Checkpoint
- Ensure all tests pass
- Ensure all tests pass, ask the user if questions arise.
- Add shell completion generation
- 5.1 Add clap_complete dependency to Cargo.toml-Add `clap_complete = "4.5"` to dependencies
- Requirements: 5.5
- 5.2 Add `--generate-completion` flag to CLI struct-Add optional `Shell` argument to Cli struct
- Handle completion generation before normal command processing
- Output completion script to stdout
- $G, 5.3, 5.4
- 5.3 Write integration tests for completion generation-Test bash completion output contains expected patterns
- Test zsh, fish, powershell completion outputs
- $G, 5.3, 5.4
- Add update checker
- 6.1 Create src/update.rs module-Define `UpdateCache` struct with `checked_at` and `latest_version`
- Define `UpdateResult` enum (UpToDate, UpdateAvailable)
- Define `UpdateChecker` struct with cache path
- SharedArrayBuffer, 9.5
- 6.2 Implement cache read/write methods-Read cache from JSON file in cache directory
- Write cache after successful version fetch
- Handle missing/corrupt cache gracefully
- Requirements: 9.5
- 6.3 Implement version check logic-Check cache first, use if within 24 hours
- Fetch latest version from GitHub API if cache expired
- Compare versions and return appropriate result
- Handle network errors gracefully (warn, don't fail)
- $J, 9.3, 9.4
- 6.4 Add `--check-update` flag to CLI-Add flag to Cli struct
- Call update checker and display result
- $J, 9.3
- 6.5 Write property test for update caching-Property 4: Update Check Caching
- $L 9.5
- Add doctor command
- 7.1 Create src/commands/doctor.rs module-Define `DoctorCommand` struct
- Implement `execute()` method
- lines
- 7.2 Implement system info display-Show CLI version, OS, architecture
- Requirements: 11.2
- 7.3 Implement daemon status check-Check if daemon is running
- Show uptime if running
- Requirements: 11.3
- 7.4 Implement configuration check-Find and display config file location
- Show project name and version if found
- Requirements: 11.4
- 7.5 Implement diagnostic checks-Check daemon socket accessibility
- Check cache directory writability
- Display pass/fail for each check
- Requirements: 11.5
- 7.6 Register doctor command in main.rs-Add Doctor variant to Commands enum
- Wire up command execution
- lines
- 7.7 Write integration tests for doctor command-Test command runs without errors
- Test output contains expected sections
- lines, 11.2, 11.3, 11.4
- Checkpoint
- Ensure all tests pass
- Ensure all tests pass, ask the user if questions arise.
- Improve error context
- 9.1 Add context to file operations in config.rs-Add `.with_context()` to `read_to_string` calls
- Include file path in error messages
- Requirements: 8.1, 8.4
- 9.2 Add context to daemon connection in daemon_client.rs-Add `.with_context()` to connection calls
- Include socket path or port in error messages
- Requirements: 8.2, 8.4
- 9.3 Add context to config parsing-Add `.with_context()` to TOML parsing
- Include file path in parse error messages
- Requirements: 8.3, 8.4
- 9.4 Write property test for error context-Property 3: Error Messages Include Relevant Context
- $L 8.1, 8.2, 8.3
- Standardize JSON output
- 10.1 Audit commands for JSON output consistency-Review all commands that support `--format json`
- Identify commands not using `SuccessResponse`/`ErrorResponse`
- Requirements: dx-form, dx-guard, dx-a11y
- 10.2 Update commands to use standard response structs-Modify identified commands to use `SuccessResponse::with_results()`
- Ensure all JSON output includes `success` and `version` fields
- Requirements: RPS HTTP
- 10.3 Implement JSON error output-Create helper function for JSON error output
- Use `ErrorResponse` for all error cases with JSON format
- Include `error`, `code`, and `hint` fields
- Requirements: 10.4, 10.5
- 10.4 Write property tests for JSON output-Property 5: JSON Output is Valid JSON
- Property 6: JSON Output Has Required Structure
- $L dx-form, dx-guard, dx-a11y, 10.4, 10.5
- Clean up dead code
- 11.1 Audit output.rs for dead code-Review all `#[allow(dead_code)]` annotations
- Remove annotations from code that is used
- Add documentation comments explaining public API intent for unused code
- $K
- 11.2 Audit config.rs for dead code-Review all `#[allow(dead_code)]` annotations
- Remove or document as appropriate
- $K
- 11.3 Verify compilation without dead code warnings-Run `cargo build` and check for warnings
- Fix any remaining dead code issues
- Requirements: 2.3
- Fix cross-platform test paths
- 12.1 Audit tests for hardcoded Unix paths-Search for `/` path separators in test files
- Identify tests using paths like `/nonexistent/path`
- Requirements: 7.1
- 12.2 Replace hardcoded paths with platform-agnostic alternatives-Use `std::path::PathBuf` for path construction
- Use `tempfile` crate for temporary test directories
- Use platform-appropriate invalid paths for error tests
- Requirements: 7.2, 7.3
- 12.3 Verify tests pass on all platforms-Run tests locally
- Ensure no platform-specific failures
- Requirements: 7.4
- Add command integration tests
- 13.1 Add integration tests for config subcommand-Test loading config from temp directory
- Test error handling for missing config
- Requirements: 6.1
- 13.2 Add integration tests for cache subcommand-Test cache operations in temp directory
- Requirements: 6.2
- 13.3 Add integration tests for error handling-Test error output for missing files
- Test error output for invalid arguments
- Requirements: 6.3
- 13.4 Add integration tests for JSON output-Test JSON format flag produces valid JSON
- Test JSON output has required fields
- Requirements: 6.4
- Final checkpoint
- Ensure all tests pass
- Ensure all tests pass, ask the user if questions arise.

## Notes

- All tasks are required for comprehensive implementation
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties
- Unit tests validate specific examples and edge cases
