
# DX-Py Benchmarks

Benchmark framework for DX-Py components with output validation.

## Status: In Development

The benchmark framework validates output correctness before reporting performance comparisons.

## Overview

This crate provides a benchmark framework for comparing DX-Py against: -CPython — Runtime performance -UV — Package manager performance -pytest — Test runner performance

## Important Notes

- Benchmarks validate output correctness by comparing DX-Py output with CPython
- Only benchmarks with matching outputs are reported as valid
- Feature coverage is tracked and reported honestly
- Performance claims require validated output

## Quick Start

```bash


# Build benchmarks


cargo build --release


# Run all benchmark suites


./target/release/benchmark run --suite all


# Run specific suite


./target/release/benchmark run --suite runtime


# List available benchmarks


./target/release/benchmark list ```


## Commands


+---------+-------------+
| Command | Description |
+=========+=============+
| `run`   | Execute     |
+---------+-------------+


### Run Options


+----------+-------------+
| Option   | Description |
+==========+=============+
| `--suite | <name>`     |
+----------+-------------+


## Output Validation


Each benchmark: -Runs the same code on both DX-Py and CPython -Captures stdout/stderr from both -Compares outputs (normalized for whitespace) -Only reports timing if outputs match


### Validation Status


+---------+---------+
| Status  | Meaning |
+=========+=========+
| `Valid` | Outputs |
+---------+---------+


## Architecture


@tree:benchmarks[]


## Feature Coverage


The benchmark report includes: -Total benchmarks attempted -Valid benchmarks (outputs match) -Invalid benchmarks (outputs differ) -Feature coverage percentage


## Statistical Analysis


Valid benchmarks include: -Mean and median timing -Standard deviation -95% confidence intervals -Welch's t-test for significance (p < 0.05)


## Test Coverage


~90+ tests covering: -Output validation -Report generation -Statistical analysis -Feature coverage tracking


## Testing


```bash

# Run all tests

cargo test

# Run property-based tests

cargo test -- proptest ```

## Requirements

- Rust 1.70+
- CPython 3.12+ (for runtime benchmarks)
- UV (for package manager benchmarks)
- pytest (for test runner benchmarks)

## See Also

- Current Benchmark Results (CURRENT_BENCHMARK.md)
- Runtime README (../runtime/README.md)
- Main Documentation (../README.md)

## License

MIT OR Apache-2.0
