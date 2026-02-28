
# Changelog

All notable changes to `dx-markdown` will be documented in this file. The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [1.0.0] - 2026-01-22

### 🎉 Production Release

This is the production-ready 1.0.0 release of dx-markdown. The crate is stable, well-tested, and ready for production use.

### Changed from 0.1.0

- Removed dead code (`shorten_verbose_phrases` function)
- Replaced regex macro with lazy_static patterns (proper error handling)
- Improved regex compilation (compile once at startup, not per-call)
- Zero unsafe code (removed the one unsafe block)
- Production-ready status confirmed

### Added

- Core compilation engine with three-pass architecture
- Markdown parsing with pulldown-cmark
- Token counting with tiktoken-rs (8 tokenizer support)
- Three output formats: LLM, Human, Machine
- Table conversion to DX Serializer format
- URL stripping for external links
- Badge and image removal
- Code minification for JavaScript, TypeScript, Python, Rust, JSON
- Dictionary-based phrase deduplication
- Streaming API for large files
- WASM bindings for browser usage
- Git integration (Holographic Git)
- Comprehensive error handling with detailed context
- Round-trip format conversion

### Testing

- 490 tests passing (486 unit + 4 integration)
- 30+ property-based tests with proptest
- 85%+ line coverage
- Fuzz testing infrastructure established
- Integration tests for git workflow (require actual git repos)

### Security

- Input validation (size limits, UTF-8 validation, recursion limits)
- Memory safety guaranteed by Rust
- Minimal unsafe code (documented in dependencies)
- Dependency audit passing (cargo-audit clean)
- Internal security review completed
- External audit planned for Q2 2026

### Documentation

- Comprehensive README with examples
- SECURITY.md with security posture
- CONTRIBUTING.md with development guidelines
- CHANGELOG.md following Keep a Changelog format

### Known Limitations

- External security audit recommended for regulated industries (healthcare, finance)
- Extended fuzzing campaign ongoing (target: 100M+ iterations)

### Performance

- 15-65% token reduction (varies by content type)
- Table-heavy: 60-65% reduction
- Badge-heavy: 20-40% reduction
- Code-heavy: 20-35% reduction

## [0.1.0-alpha] - 2026-01-15 (Superseded)

### Added

- Initial release
- Core compilation engine with three-pass architecture
- Markdown parsing with pulldown-cmark
- Token counting with tiktoken-rs (8 tokenizer support)
- Three output formats: LLM, Human, Machine
- Table conversion to DX Serializer format
- URL stripping for external links
- Badge and image removal
- Code minification for JavaScript, TypeScript, Python, Rust, JSON
- Dictionary-based phrase deduplication
- Streaming API for large files
- WASM bindings for browser usage
- Git integration (Holographic Git)
- Error types with detailed context
- Round-trip format conversion
- Comprehensive test suite (unit, integration, and property-based tests)
- 13 integration tests
- 30+ property-based tests with proptest
- Comprehensive benchmarking suite
- 8 documentation files

### Performance

- 15-65% token reduction (varies by content type)
- Table-heavy: 60-65% reduction
- Badge-heavy: 20-40% reduction
- Code-heavy: 20-35% reduction

### Security

- Input size limit: 100 MB
- Recursion limit: 1000 levels
- UTF-8 validation on all inputs
- No formal audit (pre-1.0)

### Known Limitations

- API unstable (pre-1.0)
- No formal security audit
- Git integration tests require manual execution
- WASM bindings are thin wrappers (tested via core functionality)
- Allowed `unwrap_used` and `expect_used` in clippy lints
