
# Tasks Document: Production Hardening for DX Forge

## Overview

This document contains the implementation tasks for transforming DX Forge from a prototype into a production-ready codebase. Tasks are organized by priority and dependency order.

## Critical Path Tasks (Must Complete First)

### Task 1: Replace Unsafe `static mut` with `OnceLock`

Priority: CRITICAL - Memory safety issue File: `src/api/lifecycle.rs` Estimated Effort: 2 hours Description: The current implementation uses `static mut` for global state, which is unsafe and acknowledged with `#![allow(static_mut_refs)]`. Replace with Rust 1.70+ `OnceLock` pattern. Current Code (lines 10-15):
```rust
static INIT: Once = Once::new();
pub(crate) static mut FORGE_INSTANCE: Option<Arc<RwLock<Forge>>> = None;
pub(crate) static mut TOOL_REGISTRY: Option<Arc<RwLock<HashMap<String, Arc<RwLock<Box<dyn DxTool>>>>>>> = None;
static mut CURRENT_CONTEXT: Option<Arc<RwLock<ExecutionContext>>> = None;
```
Target Code:
```rust
use std::sync::OnceLock;
static FORGE_INSTANCE: OnceLock<Arc<RwLock<Forge>>> = OnceLock::new();
static TOOL_REGISTRY: OnceLock<Arc<RwLock<HashMap<String, Arc<RwLock<Box<dyn DxTool>>>>>>> = OnceLock::new();
static CURRENT_CONTEXT: OnceLock<Arc<RwLock<ExecutionContext>>> = OnceLock::new();
```
Acceptance Criteria: -Remove all `static mut` declarations -Remove `#![allow(static_mut_refs)]` from lib.rs -All unsafe blocks in lifecycle.rs eliminated -`cargo clippy` passes without unsafe warnings -Existing tests still pass

### Task 2: Fix Test Imports

Priority: CRITICAL - Tests don't compile Files: `tests/*.rs` Estimated Effort: 1 hour Description: Test files use `use forge::*` but the crate is named `dx-forge` (or `dx_forge` in Rust). Fix Pattern:
```rust
// BEFORE use forge::Orchestrator;
// AFTER use dx_forge::Orchestrator;
```
Acceptance Criteria: -`cargo test --no-run` compiles all test targets -`cargo test` runs without import errors

## High Priority Tasks (Core Functionality)

### Task 3: Implement R2 Cloud Sync

Priority: HIGH - Advertised feature is fake Files: `src/dx_cache.rs`, `src/daemon/worker.rs` Estimated Effort: 8 hours Description: The R2 sync functions currently just return empty results. Implement actual R2 integration using the AWS S3-compatible API. Current Code (`src/dx_cache.rs` lines 270-280):
```rust
pub async fn sync_to_r2(&self, _tool: DxToolId) -> Result<SyncResult> { // TODO: Implement R2 sync using crate::storage::r2::R2Storage Ok(SyncResult { uploaded: 0, skipped: 0, failed: 0 })
}
```
Implementation Requirements: -Use `reqwest` or `aws-sdk-s3` for R2 API calls -Read R2 credentials from environment (`DX_R2_BUCKET`, `DX_R2_ACCESS_KEY`, `DX_R2_SECRET_KEY`) -Implement upload with content-hash deduplication -Implement download with integrity verification -Handle network errors with retry logic Acceptance Criteria: -`sync_to_r2()` uploads cache entries to R2 -`pull_from_r2()` downloads cache entries from R2 -Network errors are retried with exponential backoff -Missing credentials return clear error message -Integration test with mock R2 endpoint

### Task 4: Implement Background Worker Tasks

Priority: HIGH - All workers are fake File: `src/daemon/worker.rs` Estimated Effort: 6 hours Description: All `WorkerTask` variants just call `tokio::time::sleep()`. Implement actual functionality. Tasks to Implement: +----------------------------------------------+---------------------------------------------------------------+--------------------------------------------------------+
| Task                                         | Current                                                       | Required                                               |
+==============================================+===============================================================+========================================================+
| `WarmCache`                                  | `sleep\(100ms\)`                                              | Load cache index                                       |
+----------------------------------------------+---------------------------------------------------------------+--------------------------------------------------------+
| preload hot entries `SyncToR2`               | `sleep\(200ms\)`                                              | Call `DxToolCacheManager::sync_to_r2\(\)` `PullFromR2` |
+----------------------------------------------+---------------------------------------------------------------+--------------------------------------------------------+
| `sleep\(200ms\)`                             | Call `DxToolCacheManager::pull_from_r2\(\)` `AnalyzePatterns` | `sleep\(300ms\)`                                       |
+----------------------------------------------+---------------------------------------------------------------+--------------------------------------------------------+
| Scan files for DX patterns `PrefetchPackage` | `sleep\(500ms\)`                                              | Download package to local cache `CleanCache`           |
+----------------------------------------------+---------------------------------------------------------------+--------------------------------------------------------+
| `sleep\(100ms\)`                             | Remove entries older than max_age `BuildCache`                | `sleep\(200ms\)`                                       |
+----------------------------------------------+---------------------------------------------------------------+--------------------------------------------------------+
| Hash and store output files `IndexProject`   | `sleep\(300ms\)`                                              | Build file index for project                           |
+----------------------------------------------+---------------------------------------------------------------+--------------------------------------------------------+ Acceptance Criteria: -Each task performs its documented function -Tasks report accurate completion status -Failed tasks are logged with error details -Unit tests verify each task type



### Task 5: Implement Package Management Stubs

Priority: HIGH - Core feature is stubbed File: `src/api/packages.rs` Estimated Effort: 8 hours Description: Package management functions log messages but don't do anything. Functions to Implement: -`install_package_with_variant()` - Download and install package -`uninstall_package_safely()` - Remove package files -`update_package_intelligently()` - Compare versions, update -`list_all_installed_packages()` - Read from package index -`search_dx_package_registry()` - Query registry API Acceptance Criteria: -Packages can be installed from registry -Installed packages are tracked in index -Uninstall removes all package files -Search returns matching packages -Version conflicts are detected

## Medium Priority Tasks (Quality & Maintainability)

### Task 6: Eliminate `.unwrap()` Calls in Production Code

Priority: MEDIUM - Stability Files: All `src/**/*.rs` files Estimated Effort: 4 hours Description: Find and replace all `.unwrap()` calls with proper error handling. Audit Command:
```bash
grep -rn "\.unwrap()" src/ --include="*.rs" | grep -v "#\[cfg(test)\]" | grep -v "mod tests"
```
Replacement Patterns:
```rust
// Pattern 1: Propagate error let value = map.get("key").ok_or_else(|| anyhow!("Missing key"))?;
// Pattern 2: Default value let value = map.get("key").unwrap_or(&default);
// Pattern 3: Provably safe (document why)
let regex = Regex::new(r"^\d+$").expect("Static regex is always valid");
```
Acceptance Criteria: -Zero `.unwrap()` in non-test code (except documented safe cases) -All error paths return descriptive errors -`cargo clippy` passes

### Task 7: Remove Dead Code

Priority: MEDIUM - Maintainability Files: Various Estimated Effort: 3 hours Description: Run `cargo build` and address all `dead_code` warnings. Options for Each Warning: -Delete if truly unused -Add `#[allow(dead_code)]` with comment if reserved for future -Add `#[cfg(test)]` if test-only -Export publicly if meant to be used externally Acceptance Criteria: -`cargo build 2>&1 | grep "dead_code"` returns empty -All `#[allow(dead_code)]` have explanatory comments

### Task 8: Honest README

Priority: MEDIUM - Trust & Professionalism File: `README.md` Estimated Effort: 2 hours Description: The README makes false claims. Update to accurately reflect implementation status. Claims to Fix: +---------------------+-------------------------------+----------------------------------------------------------+
| Current Claim       | Reality                       | Fix                                                      |
+=====================+===============================+==========================================================+
| "production-ready"  | Prototype                     | "experimental" or "beta" "132/132 functions implemented" |
+---------------------+-------------------------------+----------------------------------------------------------+ New Status Section:
```markdown


## 📊 Implementation Status


+----------+-------------+---------+-------+
| Category | Implemented | Stubbed | Total |
+==========+=============+=========+=======+
| Core     | APIs        | 4       | 0     |
+----------+-------------+---------+-------+
⚠️ **Note**\: This is beta software. Some advertised features are not yet implemented.
```
Acceptance Criteria: -No false claims about implementation status -Clear distinction between implemented vs planned -Honest "beta" or "experimental" label -Status table shows actual implementation state

## Lower Priority Tasks (Polish)

### Task 9: Consolidate Duplicate Orchestrators

Priority: LOW - Architecture cleanup Files: `src/orchestrator.rs`, `src/sovereign/orchestrator.rs` Estimated Effort: 4 hours Description: There are two orchestrator implementations. Consolidate to one. Acceptance Criteria: -Single orchestrator implementation -Legacy code marked `#[deprecated]` with migration path -All tests use primary orchestrator

### Task 10: Deprecate Legacy Watcher

Priority: LOW - Architecture cleanup Files: `src/watcher.rs`, `src/watcher_legacy/` Estimated Effort: 2 hours Description: Mark legacy watcher as deprecated, document migration. Acceptance Criteria: -`watcher_legacy` module marked `#[deprecated]` -Migration guide in module docs -No new code uses legacy watcher

### Task 11: Add Property Tests for Core Logic

Priority: LOW - Quality Files: `src/dx_cache.rs`, `src/api/reactivity.rs`, `src/config/validator.rs` Estimated Effort: 6 hours Description: Add property-based tests using `proptest` for core functionality. Properties to Test: -Cache content round-trip (store → retrieve = original) -Config validation catches all invalid values -Debounced events coalesce correctly -Error messages always contain context Acceptance Criteria: -Property tests for cache operations -Property tests for config validation -Property tests for debouncing -All property tests pass with 100+ iterations

## Task Dependencies

@tree[]

## Completion Checklist

### Phase 1: Build Fixes (Week 1)

- Task 1: Replace `static mut` with `OnceLock`
- Task 2: Fix test imports

### Phase 2: Core Implementation (Week 2-3)

- Task 3: Implement R2 sync
- Task 4: Implement worker tasks
- Task 5: Implement package management

### Phase 3: Quality (Week 4)

- Task 6: Eliminate `.unwrap()` calls
- Task 7: Remove dead code
- Task 8: Honest README

### Phase 4: Polish (Week 5)

- Task 9: Consolidate orchestrators
- Task 10: Deprecate legacy watcher
- Task 11: Add property tests

## Success Metrics

After completing all tasks: -Build: `cargo build` succeeds on Linux, macOS, Windows -Tests: `cargo test` passes with 0 failures -Clippy: `cargo clippy` reports 0 errors, minimal warnings -Coverage: Core modules have 60%+ test coverage -Documentation: README accurately reflects implementation -Safety: Zero `static mut`, zero unhandled `.unwrap()` in prod code
