
# Changelog

All notable changes to dx-check will be documented in this file. The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [Unreleased]

### Added

- LSP Server: Full Language Server Protocol implementation
- Real-time diagnostics on document open/change
- Code actions for fixable rules
- Hover documentation for rules
- Configuration hot-reload
- Graceful shutdown handling
- VS Code Extension Integration: Complete IDE integration
- Automatic LSP server start
- Status bar with diagnostic counts
- Commands: lint, fix, show rules, restart server
- Settings integration
- CLI Enhancements
- Multiple output formats: pretty, compact, JSON, GitHub, JUnit
- `dx check analyze` command for project analysis
- `dx check rule list` with category and enabled filters
- Verbose mode with timing information
- Exit codes for CI integration
- Configuration System
- `dx.toml` configuration file support
- ESLint-style configuration compatibility
- Environment variable substitution
- Glob pattern overrides
- Configuration validation with specific errors
- Plugin System
- Native Rust plugins
- WASM plugin support
- JavaScript plugin support
- Plugin marketplace client
- CI/CD Integration
- GitHub Actions workflow generator
- GitLab CI support
- Azure DevOps support
- CircleCI support
- Cloud Team Sync
- Team configuration sharing
- Offline-first with conflict resolution
- Cloud client for sync operations
- Testing Infrastructure
- Unit tests for public APIs
- CLI integration tests
- LSP protocol tests
- Benchmark suite
- CI workflow for automated testing
- Documentation
- Configuration guide
- Rule reference
- Extension usage guide
- Contribution guide

### Changed

- Integrated dx-check into the dx workspace
- Updated to Rust 2021 edition
- Improved error messages with source locations

### Fixed

- Configuration loading from parent directories
- Parallel processing for large codebases
- AST cache invalidation on file changes

## [0.1.0] - 2025-12-27

### Added

- Initial release
- Binary Rule Fusion Engine for single-pass AST traversal
- SIMD pattern scanner for fast file scanning
- Thread-per-core reactor for parallel processing
- Binary AST cache for incremental linting
- Project intelligence for auto-detection
- 8 built-in rules:-`no-console`
- `no-debugger`
- `no-unused-vars`
- `eqeqeq`
- `prefer-const`
- `no-var`
- `no-eval`
- `no-with`
- Multi-language support via binary rule format
- CLI with check, format, analyze commands
