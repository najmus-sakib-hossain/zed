
# Changelog

All notable changes to dx-style will be documented in this file. The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [Unreleased]

### Added

- Production observability with tracing instrumentation
- Enhanced error handling with thiserror derive macros

### Changed

- Improved code hygiene with zero compiler warnings

### Fixed

- Dead code warnings resolved with appropriate annotations

## [0.1.0] - 2026-01-10

### Added

- Binary Dawn format (`.dxbd`) for zero-copy CSS loading
- SIMD-optimized HTML class extraction with `extract_classes_fast()`
- Theme generation from color palettes
- CSS property matching and generation engine
- Arbitrary value parser for Tailwind-style `[value]` syntax
- Property-based testing for binary format correctness
- `compile()` function for HTML-to-Binary Dawn conversion
- `BinaryDawnWriter` and `BinaryDawnReader` for binary format I/O
- Style caching with `StyleCache`
- Animation utilities and easing functions
- CSS grouping and similarity detection
- Remote style fetching capabilities
- File watcher for style hot-reloading
- Platform-specific optimizations (Windows console support)

### Changed

- Initial public release

### Security

- Input validation for all file paths
- Safe binary format parsing with bounds checking
