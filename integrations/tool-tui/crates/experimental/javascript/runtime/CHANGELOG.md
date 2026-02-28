
# Changelog

All notable changes to dx-js-runtime will be documented in this file. The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [Unreleased]

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.1] - 2025-12-29

Initial early development release. Many features are experimental or incomplete.

### Added

- Structured Error Handling: Complete error handling infrastructure with `JsException`, `StackFrame`, `SourceLocation`, and `CodeSnippet` types
- Source Map Support: JIT compiler now generates source maps for accurate error reporting
- Stack Trace Capture: `capture_stack_trace()` function for walking JIT frames and mapping to JavaScript source
- Error Formatting: `ErrorFormatter` trait with colored output support and code snippets
- Memory Management: Configurable heap limits with `GcConfig.max_heap_size`
- OOM Handling: `alloc_checked()` method with proper `OomError` reporting
- Memory Statistics: `MemoryUsage` struct compatible with Node.js `process.memoryUsage()` format
- CLI Flags: `--max-heap-size` flag for configuring heap limits (16MB
- 16GB)
- CommonJS Resolution: Full Node.js-compatible CommonJS module resolution
- ESM Resolution: Full Node.js-compatible ES module resolution with exports field support
- Not Implemented Errors: Consistent "Not implemented: [api_name]" error format for missing APIs
- JavaScript Contexts: `JsContext` struct for isolated execution environments
- Thread Safety: Thread-local GC heap and synchronized global compiler state
- Feature Detection: `dx.features` object for runtime feature detection
- Unsupported Feature Errors: Clear error messages for unsupported JavaScript features
- Configuration System: Support for `dx.config.js` and `dx.config.json` configuration files
- Property-Based Tests: Comprehensive property tests for all major features

### Changed

- Workspace version inheritance: all crates now inherit version from root Cargo.toml
- Improved public API documentation with `#[doc]` attributes and examples
- Enhanced CLI help output with comprehensive descriptions and examples

### Fixed

- Error messages now include file, line, and column information
- Built-in function errors now map back to JavaScript source locations
- Memory statistics now accurately track heap usage and GC events

### Security

- No known vulnerabilities (verified with `cargo audit`)

## [0.0.0] - 2024-01-01

### Added

- Initial prototype of dx-js-runtime
- OXC parser integration for JavaScript/TypeScript parsing
- Cranelift JIT compiler for native code generation
- Basic garbage collector with generational collection
- Persistent code cache for fast cold starts
- Basic built-in functions (console.log, Math, etc.)
