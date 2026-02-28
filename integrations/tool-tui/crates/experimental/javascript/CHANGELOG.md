
# Changelog

All notable changes to the DX JavaScript Toolchain will be documented in this file. The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [Unreleased]

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.1] - 2025-12-29

Initial early development release. This release establishes the foundation with error handling, memory management, thread safety, and testing infrastructure. Many features are experimental or incomplete.

### Added

#### Runtime (`dx-js`)

- Structured Error Handling: Complete error infrastructure with stack traces, source locations, and code snippets
- Source Map Support: JIT compiler generates source maps for accurate error reporting
- Memory Management: Configurable heap limits (16MB
- 16GB) with OOM handling
- Memory Statistics: Node.js-compatible `process.memoryUsage()` API
- JavaScript Contexts: Isolated execution environments for thread-safe parallel execution
- Feature Detection: `dx.features` object for runtime capability detection
- Configuration System: Support for `dx.config.js` and `dx.config.json`

#### Package Manager (`dx`)

- Global Installation: `dx install
- g <package>` for global package installation
- Global Listing: `dx list
- g` to view globally installed packages
- Enhanced CLI: Comprehensive help output with examples

#### Bundler (`dx-bundle`)

- Delta Bundling: Incremental bundling for faster rebuilds
- DXM Format: Optimized module format for fast loading

#### Test Runner (`dx-test`)

- Parallel Execution: Multi-threaded test execution
- VM Isolation: Isolated test environments

#### Compatibility Layer

- Node.js APIs: Expanded coverage of Node.js core modules
- Bun APIs: Basic Bun compatibility layer
- Web APIs: Browser-compatible APIs

#### CI/CD

- GitHub Actions: CI workflow for all platforms (Linux, macOS, Windows)
- Release Workflow: Automated binary builds and GitHub releases
- Benchmark Regression: Automated performance regression detection

#### Documentation

- Getting Started Guide: Installation and basic usage
- API Reference: Comprehensive Rust API documentation
- Compatibility Matrix: Node.js API implementation status
- Migration Guide: Moving from Node.js/npm/Jest to DX

### Changed

- All crates now use workspace version inheritance
- Improved public API documentation with examples
- Enhanced CLI help output across all tools
- Conservative benchmark claims with methodology notes

### Fixed

- Error messages now include complete source location information
- Built-in function errors map back to JavaScript source
- Memory statistics accurately track heap usage
- Thread safety for concurrent JavaScript execution

### Security

- No known vulnerabilities (verified with `cargo audit`)
- Thread-safe global state with proper synchronization
- Memory-safe garbage collection

## [0.0.0] - 2024-01-01

### Added

- Initial prototype of DX JavaScript Toolchain
- `dx-js`: JavaScript/TypeScript runtime with Cranelift JIT
- `dx`: npm-compatible package manager
- `dx-bundle`: ES module bundler
- `dx-test`: Parallel test runner
- Basic Node.js compatibility layer
