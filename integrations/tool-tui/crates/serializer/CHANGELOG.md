
# Changelog

All notable changes to dx-serializer will be documented in this file. The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## Unreleased

### Added

- Integration examples (planned)
- Thread safety documentation (planned)
- Fuzzing infrastructure (planned)
- CI pipeline configuration (planned)

## 0.1.0 - 2026-01-13

Initial production-ready release of dx-serializer.

### Added

#### Core Formats

- DX LLM Format: Token-efficient text format optimized for LLM context windows
- 25% fewer tokens than TOON format
- 63.5% fewer tokens than JSON
- Root-level `key|value` pairs for configuration
- Boolean markers: `+` (true), `-` (false), `~` (null)
- Array syntax: `*a,b,c`
- Reference syntax: `^ref` and `#:key|value` definitions
- Section syntax: `#<letter>(schema)` for tabular data
- Abbreviated keys support (`nm` → name, `v` → version)
- DX Machine Format: Zero-copy binary format for runtime performance
- Sub-nanosecond field access (0.70ns)
- Zero-copy deserialization
- Inline strings (≤14 bytes) without pointer chase
- Little-endian encoding
- Magic header: `0x5A 0x44` ("ZD")

#### Simplified Public API

- `serialize()` and `deserialize()` convenience functions for common use cases
- `SerializerBuilder` with fluent API for advanced configuration-`indent_size()`, `expand_keys()`, `validate_output()` options
- `for_humans()` and `for_llms()` presets

#### Format Conversion

- Bidirectional conversion between all formats:-`document_to_llm()` / `llm_to_document()`
- `document_to_machine()` / `machine_to_document()`
- `llm_to_human()` / `human_to_llm()`
- `llm_to_machine()` / `machine_to_llm()`

#### dx Format (Token-Efficient)

- `parse_dx_format()` and `serialize_dx_format()` for ultra-compact serialization
- SPACE-separated items (no commas)
- Tight key=value binding (no spaces around `=`)
- Quotes only for strings with spaces

#### Holographic Architecture

- `inflate()` / `deflate()` for editor integration
- Seamless conversion between Human, LLM, and Machine formats

#### Converters

- JSON to DX: `json_to_dx()`
- YAML to DX: `yaml_to_dx()`
- TOML to DX: `toml_to_dx()`
- TOON to DX: `toon_to_dx()` / `dx_to_toon()`

#### Security Features

- Input size limit: 100 MB (`MAX_INPUT_SIZE`)
- Recursion depth limit: 1000 levels (`MAX_RECURSION_DEPTH`)
- Table row limit: 10 million rows (`MAX_TABLE_ROWS`)
- Billion-laughs attack protection
- UTF-8 validation with byte offset reporting

#### Error Handling

- `DxError::Parse` with line, column, byte offset, and snippet
- `DxError::TypeMismatch` with expected and actual type names
- `DxError::SecurityLimit` for resource exhaustion prevention
- `DxError::InvalidUtf8` with byte offset
- Proper `std::error::Error` implementation with `source()` chaining

#### Utilities

- Base62 encoding/decoding for compact integers
- Binary output with caching and validation
- Schema type hints
- Token counting for LLM models (optional feature)
- File watching (optional feature)
- WASM bindings (optional feature)

#### Platform Support

- Cross-platform: Linux, macOS, Windows
- Async I/O backends: io_uring (Linux), IOCP (Windows), kqueue (macOS)
- WASM support for browser environments

#### Testing

- 490+ unit tests
- Property-based tests using proptest
- Round-trip preservation tests
- Security limit enforcement tests
- UTF-8 validation tests

### Changed

- Removed `#![allow(dead_code)]` blanket allowance from crate root
- Inlined dx-safety utilities for standalone publishability
- Updated documentation to remove marketing language
- Added benchmark methodology and reproducibility instructions

### Fixed

- `test_llm_human_round_trip`: Format detection now recognizes root-level `key|value` format
- `test_special_values_conversion`: Boolean markers (`|+`, `|-`) preserved through format conversion
- `test_serializer_to_dense`: Test assertions updated for new format behavior
- `prop_llm_round_trip_context`: Context key-value pairs preserved through round-trip
- `prop_llm_round_trip_booleans`: Boolean values survive Human format round-trip
- All compiler warnings resolved
- All clippy lints addressed

### Security

- Enforced input size limits to prevent memory exhaustion
- Enforced recursion depth limits to prevent stack overflow
- Enforced table row limits to prevent DoS attacks
- Added billion-laughs protection against exponential expansion
- Improved UTF-8 validation with detailed error reporting

## Feature Flags

+-----------+-------------+
| Feature   | Description |
+===========+=============+
| `default` | Includes    |
+-----------+-------------+



## Known Limitations

- Async I/O backends fall back to sequential operations
- Machine format uses little-endian; big-endian requires byte swapping
- Inline strings limited to 14 bytes in Machine format
