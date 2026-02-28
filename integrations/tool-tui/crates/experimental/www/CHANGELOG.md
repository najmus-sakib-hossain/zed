
# Changelog

All notable changes to the dx-www framework will be documented in this file. The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [1.0.2] - 2026-01-13

### Changed

- Replaced bincode with DX Binary Codec-Removed unmaintained `bincode` dependency (RUSTSEC-2025-0141)
- Implemented custom binary codec in `dx-www-binary/src/codec.rs`
- Zero-copy length-prefixed encoding for strings and arrays
- All 79 binary crate tests pass with new codec
- Eliminates cargo audit warning for unmaintained dependency

### Security

- Removed `bincode 2.0.1` which was flagged as unmaintained
- Custom codec provides full control over binary format

## [1.0.1] - 2026-01-13

### Fixed

- Stream Reader Header Validation
- Implemented proper magic bytes verification (DXB1 and DX formats)
- Added version validation (accepts v1 and v2)
- Improved error codes for invalid headers
- Stream Reader Patch Handling
- Implemented delta patch chunk processing
- Added PatchHeader parsing (base_hash, target_hash, algorithm)
- Support for Block XOR (algorithm 1) and VCDIFF (algorithm 2) formats
- Proper error handling for invalid patch headers
- Token Revocation Check
- Implemented token revocation verification in auth_middleware
- Added credential store integration for refresh token validation
- Proper AUTH_1004 error response for revoked tokens
- Improved Error Messages
- Enhanced panic messages in sched crate with context about browser requirements
- Improved thread spawn error messages in reactor crate
- Better documentation for WASM-specific code paths

### Changed

- Replaced generic `expect()` messages with descriptive error context
- Added `# Panics` documentation sections for functions that can panic
- Improved code comments explaining WASM-specific requirements

## [1.0.0] - 2026-01-13

### Added

- Production Readiness
- JSX Splitter: Proper template generation from JSX AST (no more dummy templates)
- JSX Splitter: Conditional rendering detection (&&, ternary expressions)
- JSX Splitter: List rendering detection (.map() calls with key tracking)
- Morph: ClassToggle binding type implementation
- Morph: Style binding type implementation
- Binary: CRC32 checksum validation in HTIP format
- Delta: Size comparison to return full target when patch is larger
- Delta: Patch chunk handling in WASM client
- Storage: IndexedDB and Cache API size calculation
- Storage: Entry counting across storage backends
- Property-Based Tests
- Parser round-trip consistency (Property 1)
- Parser error location accuracy (Property 2)
- Parser banned keyword detection (Property 3)
- Delta patch round-trip (Property 4)
- Delta patch size bound (Property 5)
- Morph dirty bit processing (Property 6)
- Morph binding type coverage (Property 7)
- Binary validation: magic bytes, version, signature, checksum (Properties 8-11)
- Splitter no dummy templates (Property 12)
- Splitter conditional binding generation (Property 13)
- Splitter iteration binding generation (Property 14)
- Server Integration Tests
- Health check endpoint verification
- Binary streaming verification
- SSR bot detection verification
- Delta patch serving verification
- Error handling (404) verification
- Documentation
- Comprehensive rustdoc for core, binary, and server crates
- Architecture diagrams in docs/architecture.md
- Complete example applications (todo-app, dashboard, blog)
- Improved error messages with file paths, line/column numbers

### Changed

- Replaced all `.expect()` calls in production code with proper error handling
- Replaced all `.unwrap()` calls in production code with fallible patterns
- Pre-compiled regex patterns using `once_cell::sync::Lazy` for better performance
- Response builders now use `unwrap_or_else` with fallback responses

### Security

- All unsafe code blocks now have documented safety invariants
- cargo audit passes with no known vulnerabilities
- 19 crates with `#![forbid(unsafe_code)]`
- Eliminated panic-prone code paths in production handlers

## [0.1.0] - 2026-01-08

### Added

- Core Framework-Audited all unsafe code with documented safety invariants
- Added #![forbid(unsafe_code)] to safe crates
- Updated docs/SECURITY.md with current threat model

### Changed

- Improved parser error messages with suggestions for common mistakes
- Enhanced compilation error messages with context and fix suggestions
- Runtime errors now include component names and stack traces

### Deprecated

### Removed

### Fixed

### Security

- All unsafe code blocks now have documented safety invariants
- cargo audit passes with no known vulnerabilities

### CI/Lint

- All clippy warnings resolved with `-D warnings` flag
- Added lint allowances for intentional patterns:-`collapsible_if` in form, rtl, cache, core, error crates
- `should_implement_trait` in core crate
- `regex_creation_in_loops` in core crate
- `len_without_is_empty` in sched crate
- `vec_init_then_push` in server crate
- `doc_nested_refdefs` in server crate
- Comprehensive lint allowances in reactor crate for low-level I/O operations
- Fixed `manual_div_ceil` warnings in binary/delta.rs using `div_ceil()` method
- All workspace tests pass (excluding WASM client crates due to panic_impl conflicts)

## [0.1.0] - 2026-01-08

### Added

- Core Framework
- TSX/JSX parser using OXC (Oxidation Compiler) with full AST support
- Tree shaking for dead code elimination with usage graph analysis
- HTIP (Hyper Text Interchange Protocol) binary format for template streaming
- Delta patching for efficient incremental updates using BLAKE3 hashing
- Runtime
- Cross-platform I/O reactor with io_uring (Linux 5.1+), epoll (Linux), kqueue (macOS/BSD), and IOCP (Windows) backends
- Sub-20KB WASM client runtime with pure FFI (no wasm-bindgen)
- JavaScript host function implementations for DOM manipulation and event handling
- Dirty bit tracking for incremental DOM updates
- Server
- SSR (Server-Side Rendering) with binary streaming
- Delta patching for efficient client updates
- WebSocket support for real-time communication
- Authentication middleware with Ed25519 tokens
- Developer Experience
- Compile-time accessibility auditing (a11y) with AST analysis
- Binary validation engine for forms with zero runtime overhead
- RTL detection and CSS flipping for internationalization
- Print stylesheet generator
- Infrastructure
- Cargo workspace with 29 member crates
- CI/CD pipeline with GitHub Actions (fmt, clippy, test, build)
- Cross-platform builds (Linux, macOS, Windows)
- Code coverage reporting with cargo-llvm-cov
- Benchmark suite with Criterion
- Testing
- Property-based testing with proptest
- Parser round-trip consistency tests
- Delta patch round-trip and corruption detection tests
- Reactor I/O callback invocation tests
- A11y rule detection tests

### Changed

- N/A (initial release)

### Deprecated

- N/A (initial release)

### Removed

- N/A (initial release)

### Fixed

- N/A (initial release)

### Security

- Eliminated unsafe `.unwrap()` calls in non-test code paths
- Added `#![forbid(unsafe_code)]` lint configuration for safe crates
- Documented all uses of unsafe code with safety invariants
- Integrated cargo-audit in CI for vulnerability detection

## Version History

+---------+------------+----------+------------+
| Version | Release    | Date     | Highlights |
+=========+============+==========+============+
| 1.0.2   | 2026-01-13 | Replaced | bincode    |
+---------+------------+----------+------------+



## Upgrade Guide

### Upgrading from 0.1.0 to 1.0.0

- No breaking API changes
- All `.expect()` and `.unwrap()` calls in production code replaced with proper error handling
- Regex patterns now pre-compiled for better performance
- Version number updated to signal production readiness

### Upgrading to 0.1.0

This is the initial release. No upgrade steps required.

## Links

- Repository
- Documentation
- Issue Tracker
