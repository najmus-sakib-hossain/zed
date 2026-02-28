
# Requirements Document

## Introduction

This document specifies the requirements for making DX JavaScript Tooling production-ready and battle-tested. The goal is to fix all existing compilation errors, failing tests, and ensure the toolchain works correctly across all platforms (Windows, Linux, macOS) with real-world npm packages.

## Glossary

- DX_Toolchain: The complete DX JavaScript tooling suite including runtime, package manager, bundler, test runner, and compatibility layer
- Runtime: The dx-js-runtime component that executes JavaScript/TypeScript code
- Package_Manager: The dx-pkg-* crates that handle npm package installation
- Bundler: The dx-bundle-* crates that bundle JavaScript modules
- Compatibility_Layer: The dx-js-compatibility crate providing Node.js API compatibility
- Project_Manager: The project-manager crate for task execution
- Property_Test: A test that verifies a property holds for all generated inputs
- E2E_Test: End-to-end test that validates complete workflows

## Requirements

### Requirement 1: Fix All Compilation Errors

User Story: As a developer, I want all DX crates to compile without errors, so that I can build and use the toolchain.

#### Acceptance Criteria

- WHEN compiling the compatibility crate tests, THE Compiler SHALL produce zero errors
- WHEN compiling the bundler dx-bundle-dxm tests, THE Compiler SHALL produce zero errors
- WHEN compiling any DX crate with `cargo build
- release`, THE Compiler SHALL succeed without errors
- WHEN running `cargo test
- workspace` from any component directory, THE Test_Runner SHALL compile all test code successfully

### Requirement 2: Fix All Failing Tests

User Story: As a developer, I want all tests to pass, so that I can trust the codebase is working correctly.

#### Acceptance Criteria

- WHEN running project-manager tests, THE Test_Runner SHALL pass all tests including `test_execute_with_dependencies`
- WHEN running compatibility property tests, THE Test_Runner SHALL pass all property tests with 100+ iterations
- WHEN running bundler tests, THE Test_Runner SHALL pass all tests without packed struct alignment errors
- WHEN running runtime tests, THE Test_Runner SHALL pass all 64+ tests
- FOR ALL test suites in the workspace, running `cargo test` SHALL result in zero failures

### Requirement 3: Fix Packed Struct Alignment Issues

User Story: As a developer, I want binary format structs to be safe and correct, so that serialization works reliably.

#### Acceptance Criteria

- WHEN accessing fields in packed structs, THE Code SHALL use safe copy operations instead of references
- WHEN using `assert_eq!` on packed struct fields, THE Code SHALL copy field values to local variables first
- THE DxmHeader, ExportEntry, and ImportPatchSlot structs SHALL provide safe accessor methods
- IF a packed struct field is accessed, THEN THE Code SHALL NOT create unaligned references

### Requirement 4: Fix Property Test Return Types

User Story: As a developer, I want property tests to compile correctly, so that I can validate correctness properties.

#### Acceptance Criteria

- WHEN a proptest block contains async code, THE Test SHALL return `Result<(), TestCaseError>` from the async block
- WHEN using `prop_assert!` or `prop_assert_eq!` macros, THE Code SHALL ensure proper return type propagation
- FOR ALL property tests in web_props.rs, THE Tests SHALL compile and run successfully
- WHEN a for loop follows a `prop_assert!`, THE Code SHALL include `Ok(())` at the end of the block

### Requirement 5: Fix Task Executor Dependencies

User Story: As a developer, I want task execution with dependencies to work correctly, so that build pipelines execute properly.

#### Acceptance Criteria

- WHEN executing a task with dependencies, THE Executor SHALL wait for all dependencies to complete first
- WHEN a dependency command fails, THE Executor SHALL report the failure correctly
- WHEN testing with mock commands, THE Test SHALL use commands that exist on the test platform
- IF a task depends on another task, THEN THE Executor SHALL mark the dependency as completed before proceeding

### Requirement 6: Implement Real End-to-End Testing

User Story: As a developer, I want E2E tests that validate real-world usage, so that I can trust the toolchain works in practice.

#### Acceptance Criteria

- WHEN running `dx install lodash`, THE Package_Manager SHALL successfully install lodash and its dependencies
- WHEN running `dx install react`, THE Package_Manager SHALL successfully install react and its dependencies
- WHEN bundling a project with imports, THE Bundler SHALL produce valid JavaScript output
- WHEN running a bundled file with dx-js, THE Runtime SHALL execute it correctly
- THE E2E_Test suite SHALL include at least 10 real npm packages

### Requirement 7: Cross-Platform Compatibility

User Story: As a developer, I want DX to work on Windows, Linux, and macOS, so that all developers can use it.

#### Acceptance Criteria

- WHEN running on Windows, THE Package_Manager SHALL use junctions for node_modules linking
- WHEN running on Linux/macOS, THE Package_Manager SHALL use symlinks for node_modules linking
- WHEN executing shell commands, THE Executor SHALL use the correct shell for the platform (cmd on Windows, sh on Unix)
- FOR ALL file operations, THE Code SHALL handle path separators correctly for the platform
- WHEN running tests on CI, THE Tests SHALL pass on Windows, Linux, and macOS

### Requirement 8: Remove All Remaining TODOs in Critical Paths

User Story: As a developer, I want all critical functionality to be implemented, so that the toolchain is complete.

#### Acceptance Criteria

- WHEN performing incremental installs, THE Package_Manager SHALL compute actual diffs instead of doing full installs
- WHEN bundling modules, THE Bundler SHALL generate valid source maps
- WHEN rewriting imports, THE Bundler SHALL correctly transform import paths based on resolution
- FOR ALL TODO comments in production code paths, THE Code SHALL either implement the functionality or remove the TODO

### Requirement 9: Validate Performance Claims

User Story: As a developer, I want performance claims to be accurate and reproducible, so that marketing is honest.

#### Acceptance Criteria

- WHEN running benchmark scripts, THE Benchmarks SHALL produce reproducible results
- WHEN comparing against Bun/npm/pnpm, THE Benchmarks SHALL use fair comparison methodology
- IF a performance claim cannot be reproduced, THEN THE Documentation SHALL be updated to remove or qualify the claim
- THE Benchmark documentation SHALL include exact reproduction steps

### Requirement 10: Improve Error Messages

User Story: As a developer, I want clear error messages, so that I can debug issues quickly.

#### Acceptance Criteria

- WHEN a package installation fails, THE Error_Message SHALL include the package name and reason
- WHEN a file cannot be found, THE Error_Message SHALL include the full path attempted
- WHEN a parse error occurs, THE Error_Message SHALL include line and column numbers
- WHEN a network error occurs, THE Error_Message SHALL include the URL and HTTP status
- FOR ALL error types, THE Error_Message SHALL suggest possible solutions

### Requirement 11: Clean Up Compiler Warnings

User Story: As a developer, I want a clean build with no warnings, so that real issues are not hidden.

#### Acceptance Criteria

- WHEN compiling with `cargo build`, THE Compiler SHALL produce zero warnings in production code
- WHEN there are unused imports, THE Code SHALL remove them
- WHEN there are unused variables, THE Code SHALL prefix them with underscore or remove them
- WHEN there are dead code warnings, THE Code SHALL either use the code or remove it
- THE CI pipeline SHALL fail if any warnings are introduced

### Requirement 12: Documentation Accuracy

User Story: As a developer, I want documentation to match reality, so that I can trust what I read.

#### Acceptance Criteria

- WHEN README claims a feature is "production ready", THE Feature SHALL have passing tests
- WHEN README shows benchmark numbers, THE Numbers SHALL be reproducible with provided scripts
- WHEN README shows usage examples, THE Examples SHALL work when copy-pasted
- IF a feature is incomplete, THEN THE Documentation SHALL state its status
- THE Documentation SHALL include a "Known Limitations" section for each component
