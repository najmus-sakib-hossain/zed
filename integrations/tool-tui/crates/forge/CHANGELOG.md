
# Changelog

All notable changes to this project will be documented in this file. The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## Unreleased

## 0.1.0 - 2025-01-27

### Added

- Global State Elimination: Replaced `OnceLock`/`static` patterns with dependency injection-`BranchingEngine` extracted from global state with full revert support
- `EventBus` extracted with proper subscriber management
- `Forge` struct now owns all state, enabling multiple isolated instances
- Backward-compatible deprecated wrappers for legacy API
- Stub Implementations Completed:-`trigger_debounced_event()` now performs actual debouncing with configurable timing
- `trigger_idle_event()` now detects idle state and triggers events after threshold
- `schedule_task_for_idle_time()` now schedules tasks for idle execution
- `revert_most_recent_application()` now restores file backups
- Error Handling Improvements:-Eliminated panic-prone `.unwrap()` and `.expect()` calls in non-test code
- Added `.context()` to all error returns for better diagnostics
- Consistent use of `anyhow::Result` across public API
- Property-Based Tests:-Instance isolation property test
- Debounce timing correctness property test
- Idle detection correctness property test
- Revert round-trip property test
- Graceful error handling property test
- Backend fallback property test
- Platform I/O:-Platform-native I/O backends (io_uring, kqueue, IOCP)
- Graceful backend fallback when native backends fail
- Resource management with handle limiting
- Metrics collection and observability
- Configuration validation
- Feature Flags: Heavy dependencies now feature-gated (`crdt`, `daemon`, `semantic-analysis`)
- Parallel tool execution with dependency-based wave computation
- Git-like snapshot system with branching and merging
- Comprehensive API documentation
- Integration test suite
- Generated code tracking system
- Enhanced error handling with retry policies
- Traffic branch system for merge safety analysis
- Dual-watcher architecture (LSP + File System)
- Tool lifecycle management with events
- Component injection from R2 storage
- Pattern detection for DX tools
- Semantic versioning with compatibility checking
- Content-addressable storage with SHA-256
- CRDT-based document operations
- WebSocket server for real-time updates
- VSCode extension integration support

### Changed

- API Organization: Cleaned up `lib.rs` with clear section comments
- Dependency Rationalization:-Removed unused dependencies: `yrs`, `git2`, `jwalk`, `walkdir`, `bytes`, `md5`
- Feature-gated: `automerge`, `axum`, `tower`, `tower-http`, `tree-sitter`
- Reduced dependency count by ~8.5%
- Documentation: Updated README with accurate implementation status table
- Repository Hygiene: Updated `.gitignore`, removed committed artifacts
- Restructured core modules for better API ergonomics
- Improved documentation with examples
- Enhanced orchestrator with parallel execution support
- Updated tool trait with comprehensive lifecycle hooks

### Fixed

- Duplicate type exports (`ToolStatus` vs `SovereignToolStatus`)
- Missing deprecation attributes on legacy exports
- Doc examples now compile and run successfully
- Compilation errors in core modules
- Missing exports in lib.rs
- Circular dependency detection

### Deprecated

- `initialize_forge()`
- Use `Forge::new()` instead
- `shutdown_forge()`
- Use `drop(forge)` (RAII) instead
- `ForgeWatcher` alias
- Use `DualWatcher` instead

## 0.0.2 - 2025-01-21

### Added

- Initial crate structure
- Basic tool orchestration
- File watching capabilities
- Version control foundation
- Storage layer with SQLite

### Changed

- Project restructuring for library use

## 0.0.1 - 2025-01-20

### Added

- Initial project setup
- Basic CLI implementation
- LSP server foundation
