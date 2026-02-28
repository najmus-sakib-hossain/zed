
# Requirements Document: DX Forge Production-Ready Transformation

## Introduction

This document specifies the comprehensive requirements for transforming DX Forge from its current prototype state into a truly production-ready, professional codebase suitable for publication on crates.io and real-world usage. This spec addresses critical architectural issues, code quality problems, and professional standards that were identified in a thorough codebase audit. The existing `production-hardening` spec addressed some issues but left significant gaps. This spec provides a complete, honest path to production readiness.

## Glossary

- DX_Forge: The main orchestration engine and VCS for developer experience tools
- Global_State: Static mutable variables that hold application state across function calls
- Dependency_Injection: A design pattern where dependencies are passed to components rather than accessed globally
- Orchestrator: The component that coordinates tool execution with lifecycle hooks
- API_Function: One of the public API functions exposed by DX Forge
- Stub_Implementation: A function that has a signature but returns placeholder/fake data
- Dead_Code: Code that is never executed or referenced
- Test_Coverage: The percentage of code lines executed during test runs
- Property_Test: A test that verifies properties hold for many randomly generated inputs
- RAII: Resource Acquisition Is Initialization
- a pattern for automatic resource cleanup

## Requirements

### Requirement 1: Eliminate Global Mutable State Architecture

User Story: As a library consumer, I want to use DX Forge in multi-tenant applications and tests without global state interference, so that I can run isolated instances and parallel tests safely.

#### Acceptance Criteria

- THE Codebase SHALL NOT use `OnceLock` or `static` variables for storing application state that varies between instances
- THE Forge struct SHALL be the single entry point that owns all state
- WHEN a user creates multiple Forge instances, THE instances SHALL be completely isolated from each other
- THE API functions SHALL accept a Forge reference or context parameter instead of accessing global state
- WHEN running tests in parallel, THE tests SHALL NOT interfere with each other's state
- THE Codebase SHALL use dependency injection for all cross-component communication

### Requirement 2: Complete All Stub Implementations

User Story: As a developer, I want all advertised API functions to perform their documented behavior, so that I can rely on the library for production use.

#### Acceptance Criteria

- WHEN any public API function is called, THE function SHALL perform its documented operation (not just log and return Ok)
- THE `sync_to_r2()` function SHALL upload data to R2 storage when credentials are configured
- THE `pull_from_r2()` function SHALL download data from R2 storage
- THE `install_package_with_variant()` function SHALL install packages
- THE `search_dx_package_registry()` function SHALL query a registry
- THE `trigger_debounced_event()` function SHALL debounce events with configurable timing
- THE `trigger_idle_event()` function SHALL detect idle state and trigger events
- THE `schedule_task_for_idle_time()` function SHALL schedule tasks
- THE `revert_most_recent_application()` function SHALL revert file changes (not return TODO)
- IF a function cannot be fully implemented, THEN it SHALL return `Err` with "not yet implemented" message

### Requirement 3: Eliminate Panic-Prone Code Patterns

User Story: As a production operator, I want the library to handle all error conditions gracefully without panicking, so that my application remains stable.

#### Acceptance Criteria

- THE Codebase SHALL contain zero `.unwrap()` calls in non-test code
- THE Codebase SHALL contain zero `.expect()` calls in non-test code unless the condition is provably always true with a comment explaining why
- WHEN deserializing data, THE Parser SHALL return `Result` with context about what failed
- WHEN accessing HashMap/Vec by index, THE Code SHALL use `.get()` with proper error handling
- THE Codebase SHALL use `?` operator for all fallible operations
- WHEN a panic would occur, THE Code SHALL instead return a descriptive error

### Requirement 4: Remove All Dead and Unused Code

User Story: As a maintainer, I want a clean codebase without unused code, so that I can understand and maintain it efficiently.

#### Acceptance Criteria

- WHEN `cargo build` runs, THE Compiler SHALL emit zero `dead_code` warnings
- THE Codebase SHALL NOT contain unused struct fields (fields prefixed with `_` that are never read)
- THE Codebase SHALL NOT contain empty directories (like `src/watcher/`)
- THE Codebase SHALL NOT contain duplicate implementations (single Orchestrator, single Watcher)
- THE `watcher_legacy` module SHALL either be removed or properly integrated with deprecation notices
- THE Codebase SHALL NOT contain commented-out code blocks
- IF code is intentionally reserved for future use, THEN it SHALL have `#[allow(dead_code)]` with a comment explaining the timeline

### Requirement 5: Achieve Meaningful Test Coverage

User Story: As a developer, I want comprehensive tests that verify actual behavior, so that I can refactor with confidence and catch regressions.

#### Acceptance Criteria

- THE Test_Suite SHALL achieve at least 70% line coverage on core modules (orchestrator, storage, api)
- THE Test_Suite SHALL include integration tests that perform real file I/O operations
- THE Test_Suite SHALL include tests for all error conditions and edge cases
- THE Property_Tests SHALL verify meaningful invariants (not just "code doesn't crash")
- WHEN `cargo test` runs, THE Test_Suite SHALL execute with zero failures
- THE Test_Suite SHALL NOT contain tests that always pass regardless of implementation
- WHEN a stub is implemented, THE corresponding test SHALL verify the actual behavior

### Requirement 6: Honest and Accurate Documentation

User Story: As a potential user, I want documentation that accurately reflects the implementation status, so that I can make informed decisions about using the library.

#### Acceptance Criteria

- THE README SHALL NOT claim "production-ready" status until all requirements in this spec are met
- THE README SHALL include an accurate implementation status table showing implemented vs stubbed functions
- THE README SHALL use honest labels like "alpha", "beta", or "experimental" for current status
- THE API documentation SHALL mark any functions that are not fully implemented
- THE doc examples SHALL compile and run successfully (`cargo test
- doc`)
- THE CHANGELOG SHALL accurately reflect the actual version and changes
- THE README SHALL NOT claim "132/132 functions implemented" when functions are stubs

### Requirement 7: Clean Repository Hygiene

User Story: As a contributor, I want a clean repository without committed artifacts, so that I can clone and build without issues.

#### Acceptance Criteria

- THE Repository SHALL NOT contain committed log files (like `logs/forge.log.*`)
- THE Repository SHALL NOT contain committed `node_modules` directories
- THE Repository SHALL NOT contain committed build artifacts (`.vsix` files, `out/` directories)
- THE `.gitignore` SHALL properly exclude all generated files
- THE Repository SHALL NOT contain `proptest-regressions/` files (indicates failing property tests)
- WHEN cloning fresh, THE Repository SHALL build without downloading additional artifacts

### Requirement 8: Dependency Rationalization

User Story: As a library consumer, I want minimal dependencies, so that my build times are fast and my attack surface is small.

#### Acceptance Criteria

- THE Crate SHALL NOT include both `automerge` and `yrs` unless both are actively used
- THE Crate SHALL NOT include multiple hashing libraries (`sha2`, `blake3`, `md5`) unless each serves a distinct purpose
- THE Crate SHALL use feature flags to make heavy dependencies optional
- THE Crate SHALL document why each major dependency is needed
- WHEN a dependency is only used in one module, THE dependency SHALL be feature-gated
- THE total dependency count SHALL be reduced by at least 20% from current state

### Requirement 9: Consistent Error Handling

User Story: As a user, I want consistent, informative error messages across all operations, so that I can diagnose and fix problems quickly.

#### Acceptance Criteria

- THE Codebase SHALL use `anyhow::Result` consistently for all fallible public functions
- WHEN an error occurs, THE Error SHALL include context about what operation failed using `.context()`
- THE Error messages SHALL be actionable (tell user what to do, not just what went wrong)
- THE `ForgeError` type SHALL be used consistently for domain-specific errors
- WHEN logging errors, THE Logger SHALL include structured fields (operation, file, context)

### Requirement 10: API Design Cleanup

User Story: As a library consumer, I want a clean, intuitive API without redundant or confusing exports, so that I can use the library effectively.

#### Acceptance Criteria

- THE Public API SHALL NOT export multiple types with the same name (e.g., `ToolStatus` vs `SovereignToolStatus`)
- THE Public API SHALL NOT export deprecated items without `#[deprecated]` attribute
- THE Public API SHALL follow Rust API guidelines for naming and organization
- THE `lib.rs` SHALL have clear sections with comments explaining each export group
- WHEN types are re-exported, THE re-export SHALL use consistent naming
- THE API SHALL NOT expose internal implementation details

### Requirement 11: Platform I/O Backend Verification

User Story: As a user on Linux/macOS/Windows, I want the platform-native I/O backends to work, so that I get the advertised performance benefits.

#### Acceptance Criteria

- THE io_uring backend SHALL be tested on Linux with kernel 5.1+
- THE kqueue backend SHALL be tested on macOS
- THE IOCP backend SHALL be tested on Windows
- WHEN a native backend fails to initialize, THE System SHALL fall back gracefully with a warning
- THE Platform_IO tests SHALL verify actual I/O operations, not just backend selection
- THE batch operations SHALL demonstrate measurable performance improvement over sequential

### Requirement 12: Version and Release Consistency

User Story: As a user, I want consistent version numbers across all project files, so that I know exactly what version I'm using.

#### Acceptance Criteria

- THE version in Cargo.toml SHALL match the version in README
- THE CHANGELOG SHALL have an entry for the current version
- THE git tags SHALL match the Cargo.toml version
- WHEN publishing to crates.io, THE version SHALL follow semver correctly
- THE version SHALL be 0.x.y until all requirements in this spec are met (no 1.0 until production-ready)
