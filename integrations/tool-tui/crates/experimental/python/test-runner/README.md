
# DX-Py Test Runner

A Python test runner written in Rust.

## Status: In Development

The test runner is actively being developed with a focus on correctness.

## Features

- Test discovery (finds test_*.py files)
- Test execution via worker processes
- JSON-over-stdio worker communication
- Pass/fail/error result reporting
- Graceful crash handling

## Installation

```bash
cd test-runner cargo build --release


# Binary at target/release/dx-py


```

## Quick Start

```bash


# Discover tests


dx-py discover -r ./tests


# Run all tests


dx-py test -r ./tests


# Run with pattern filter


dx-py test "test_auth*" -r ./tests


# CI mode with JUnit output


dx-py test --ci --junit-output results.xml -r ./tests ```


## Commands


+------------+-------------+
| Command    | Description |
+============+=============+
| `discover` | Find        |
+------------+-------------+


## Architecture


@tree:test-runner[]


## Implemented Features



### Test Discovery


- `test_*.py` files
- `*_test.py` files
- `test_*` functions
- `Test*` classes
- `test_*` methods


### Markers


- `@pytest.mark.skip`
- `@pytest.mark.skipif`
- `@pytest.mark.xfail`
- `@pytest.mark.parametrize`


### Fixtures


- Function scope
- Class scope
- Module scope
- Session scope
- Autouse fixtures (limited)


### Output Formats


- Console (default)
- JUnit XML
- JSON


## Known Limitations


- Fixtures have limited support
- Parametrized tests have basic support
- Some pytest plugins not supported
- Watch mode is experimental


## Test Coverage


~150+ tests covering: -Test discovery -Test execution -Worker communication -Result reporting -Crash handling


## Testing


```bash

# Run all tests

cargo test --workspace

# Run specific crate

cargo test -p dx-py-discovery cargo test -p dx-py-executor

# Run with release optimizations

cargo test --release ```

## Requirements

- Rust 1.70+ (for building)
- Python 3.8+ (for test execution)

## License

MIT
