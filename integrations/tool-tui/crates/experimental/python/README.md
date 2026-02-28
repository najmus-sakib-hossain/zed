
# DX-Py

A Python toolchain written in Rust, focused on correctness and developer experience.

## Status: In Development

DX-Py is actively being developed. While not yet production-ready for all use cases, significant progress has been made on core features.

## Overview

DX-Py provides: -Runtime — Python interpreter with bytecode compilation -Package Manager — Dependency resolution and installation -Test Runner — Test discovery and execution -Benchmarks — Comparison framework with output validation

## Installation

```bash


# Build from source


git clone https://github.com/example/dx-py cd dx-py


# Build all components


cargo build --release


# Binaries available at:



# - runtime/target/release/dx-py



# - package-manager/target/release/dx-py



# - test-runner/target/release/dx-py


```

## Quick Start

### Runtime

```bash


# Execute expressions


dx-py -c "1 + 2 * 3"


# Output: 7



# Run REPL


dx-py -i


# Show runtime info


dx-py info ```


### Package Manager


```bash

# Add dependencies

dx-py add requests numpy

# Add dev dependencies

dx-py add --dev pytest ```

### Test Runner

```bash


# Discover tests


dx-py discover -r ./tests


# Run tests


dx-py test -r ./tests ```


## Implemented Features



### Runtime


- Bytecode compiler and interpreter
- String methods (upper, lower, split, join, replace, find, etc.)
- List methods (append, extend, insert, remove, pop, sort, etc.)
- Dict methods (keys, values, items, get, pop, update, etc.)
- List comprehensions (basic, filtered, nested)
- Exception handling (try/except/finally)
- Class system with inheritance and super()
- JSON module (dumps/loads)
- Module import caching


### Package Manager


- Add packages to pyproject.toml
- Version constraint support (==, >=, <, etc.)
- Dev dependencies (--dev flag)
- Format preservation using toml_edit


### Test Runner


- Test discovery (test_*.py files)
- Test execution via worker processes
- JSON-over-stdio worker communication
- Pass/fail/error result reporting
- Graceful crash handling


## Architecture


@tree:dx-py[]


## Known Limitations



### Runtime


- Dict/set comprehensions not yet supported
- Generator expressions not yet supported
- Async/await not yet implemented
- Some stdlib modules are stubs only
- Native extension loading is experimental


### Package Manager


- Package installation from PyPI is partial
- Lock file generation is basic


### Test Runner


- Fixtures have limited support
- Some pytest plugins not supported


## Test Coverage


+-----------+-------+--------+
| Component | Tests | Status |
+===========+=======+========+
| Runtime   | ~300+ | ✅      |
+-----------+-------+--------+


## Development


```bash

# Run all tests

cargo test --workspace

# Run clippy

cargo clippy --workspace

# Format code

cargo fmt --all

# Build release

cargo build --release --workspace ```

## Documentation

- Runtime (runtime/README.md)
- Package Manager (package-manager/README.md)
- Test Runner (test-runner/README.md)
- Benchmarks (benchmarks/README.md)
- Production Readiness (PROBLEMS.md)

## License

Licensed under either of: -MIT License (LICENSE-MIT (LICENSE-MIT)) -Apache License, Version 2.0 (LICENSE-APACHE (LICENSE-APACHE))

## Contributing

Contributions welcome. Please ensure: -All tests pass: `cargo test --workspace` -No clippy warnings: `cargo clippy --workspace` -Code is formatted: `cargo fmt --all` I want to make a commertial use video so that I want to create a professional music or sound to be used by my dx - Enhanced Development Experience software - so what online of offline tool to use to make the music in my windows pc?
