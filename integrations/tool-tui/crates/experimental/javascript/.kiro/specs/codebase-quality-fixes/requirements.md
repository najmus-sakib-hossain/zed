
# Requirements Document

## Introduction

This specification defines the requirements for fixing all code quality issues in the DX JavaScript Toolchain codebase. The goal is to eliminate all compiler warnings, clippy lints, remove development-phase allows, complete incomplete implementations, and ensure the codebase passes strict quality checks suitable for production release. This spec addresses the critical issues identified in the production readiness assessment, specifically focusing on code quality, linting, formatting, and build cleanliness.

## Glossary

- DX_Runtime: The JavaScript/TypeScript execution environment in the `runtime/` workspace
- DX_Package_Manager: The npm-compatible package manager in the `package-manager/` workspace
- DX_Bundler: The ES module bundler in the `bundler/` workspace
- DX_Test_Runner: The parallel test execution engine in the `test-runner/` workspace
- DX_Project_Manager: The workspace/project management tool in the `project-manager/` workspace
- DX_Compatibility: The Node.js/Bun/Web compatibility layer in the `compatibility/` workspace
- Clippy: The Rust linter that catches common mistakes and suggests improvements
- Dead_Code: Code that is never executed or referenced
- GC_Heap: The garbage collector managing JavaScript heap objects

## Requirements

### Requirement 1: Remove Development-Phase Allows

User Story: As a maintainer, I want all global `#![allow(...)]` directives removed from library crates, so that the compiler catches potential issues and the codebase is production-ready.

#### Acceptance Criteria

- THE runtime/src/lib.rs file SHALL NOT contain `#![allow(dead_code)]`
- THE runtime/src/lib.rs file SHALL NOT contain `#![allow(unused_variables)]`
- THE test-runner/crates/dx-test-cli/src/watch.rs file SHALL NOT contain `#![allow(dead_code)]`
- THE test-runner/crates/dx-test-cli/src/snapshot.rs file SHALL NOT contain `#![allow(dead_code)]`
- THE test-runner/crates/dx-test-cli/src/mock.rs file SHALL NOT contain `#![allow(dead_code)]`
- THE test-runner/crates/dx-test-cli/src/coverage.rs file SHALL NOT contain `#![allow(dead_code)]`
- WHEN dead code is identified, THE code SHALL either be removed, used, or annotated with a specific `#[allow(dead_code)]` with a justification comment

### Requirement 2: Fix Runtime Clippy Warnings

User Story: As a developer, I want the runtime crate to pass clippy without warnings, so that the code follows Rust best practices.

#### Acceptance Criteria

- WHEN `cargo clippy` is run on the runtime workspace, THE output SHALL contain zero warnings
- THE codegen.rs file SHALL NOT have `drop_non_drop` warnings for ThreadLocalHeapLock
- THE config.rs file SHALL use `is_some_and()` instead of `map_or(false,...)`
- THE error.rs file SHALL use `push('\n')` instead of `push_str("\n")`
- THE error.rs file SHALL collapse nested `else { if }` blocks into `else if`
- THE context.rs file SHALL derive Default instead of implementing it manually
- THE main.rs file SHALL NOT have `needless_borrows_for_generic_args` warnings

### Requirement 3: Fix Package Manager Clippy Warnings

User Story: As a developer, I want the package-manager crate to pass clippy without warnings, so that the code follows Rust best practices.

#### Acceptance Criteria

- WHEN `cargo clippy` is run on the package-manager workspace, THE output SHALL contain zero warnings
- ALL functions accepting `&PathBuf` parameters SHALL be changed to accept `&Path` instead
- THE dlx.rs file SHALL use `strip_prefix()` instead of manual prefix stripping
- THE global.rs file SHALL use `next_back()` instead of `last()` on DoubleEndedIterator
- THE global.rs file SHALL collapse nested `else { if }` blocks into `else if`
- THE outdated.rs file SHALL use `strip_prefix()` instead of manual prefix stripping
- THE update.rs file SHALL use `strip_prefix()` instead of manual prefix stripping

### Requirement 4: Fix Project Manager Clippy Warnings

User Story: As a developer, I want the project-manager crate to pass clippy without warnings, so that the code follows Rust best practices.

#### Acceptance Criteria

- WHEN `cargo clippy` is run on the project-manager workspace, THE output SHALL contain zero warnings
- THE dxc.rs file SHALL use iterator enumerate() instead of range-based indexing
- THE dxl.rs file SHALL use iterator enumerate() instead of range-based indexing
- THE watch.rs file SHALL factor complex types into type definitions
- THE workspace.rs file SHALL fix the `only_used_in_recursion` warning

### Requirement 5: Fix Compatibility Clippy Warnings

User Story: As a developer, I want the compatibility crate to pass clippy without warnings, so that the code follows Rust best practices.

#### Acceptance Criteria

- WHEN `cargo clippy` is run on the compatibility workspace, THE output SHALL contain zero warnings
- THE update.rs file SHALL factor complex types into type definitions
- THE compile lib.rs file SHALL implement FromStr trait instead of custom from_str method
- THE url mod.rs file SHALL implement Display trait instead of inherent to_string method
- THE path mod.rs file SHALL use `strip_suffix()` instead of manual suffix stripping
- THE lib.rs file SHALL use `vec![]` macro instead of push after creation

### Requirement 6: Complete GC Implementation

User Story: As a developer, I want the garbage collector to properly trace all object types, so that memory is correctly managed without leaks.

#### Acceptance Criteria

- THE GC_Heap SHALL implement array tracing for ObjectType::Array
- THE GC_Heap SHALL implement object tracing for ObjectType::Object
- THE GC_Heap SHALL implement closure tracing for ObjectType::Function and ObjectType::Closure
- WHEN tracing is performed, ALL reachable objects SHALL be marked regardless of type
- THE heap.rs file SHALL NOT contain TODO comments for unimplemented tracing

### Requirement 7: Code Formatting

User Story: As a developer, I want all code to be consistently formatted, so that the codebase is readable and maintainable.

#### Acceptance Criteria

- WHEN `cargo fmt
- check` is run on any workspace, THE command SHALL exit with code 0
- ALL Rust source files SHALL be formatted according to rustfmt defaults
- THE CI pipeline SHALL enforce formatting checks on all pull requests

### Requirement 8: Build Cleanliness

User Story: As a developer, I want the build to complete without any warnings, so that potential issues are not hidden.

#### Acceptance Criteria

- WHEN `cargo build
- workspace` is run on any workspace, THE output SHALL contain zero warnings
- WHEN `cargo clippy
- workspace
- D warnings` is run, THE command SHALL exit with code 0
- ALL workspaces (runtime, package-manager, bundler, test-runner, project-manager, compatibility) SHALL pass the above checks

### Requirement 9: Unsafe Code Documentation

User Story: As a maintainer, I want all unsafe code blocks to be documented with safety comments, so that the reasoning is clear and auditable.

#### Acceptance Criteria

- WHEN an `unsafe` block is used, THE block SHALL have a `// SAFETY:` comment explaining why it is safe
- WHEN `unsafe impl Send` or `unsafe impl Sync` is used, THE implementation SHALL have documentation explaining the safety guarantees
- ALL unsafe blocks SHALL be reviewed and documented during this cleanup
