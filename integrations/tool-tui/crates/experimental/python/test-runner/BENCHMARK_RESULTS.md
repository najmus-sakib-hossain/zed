
# DX-Py Test Runner Benchmark Results

Benchmark data for DX-Py test runner.

## Status: In Development

These benchmarks require output validation before performance claims can be made.

## Test Suite

- Location: `benchmarks/test_project/`
- Test Files: 7 files
- Test Types: Simple functions, unittest-style classes, async tests, parametrized tests, fixtures

## Running Benchmarks

```bash


# Build release binary


cargo build --release


# Run discovery benchmark


target/release/dx-py discover -r benchmarks/test_project


# Run full test benchmark


target/release/dx-py test -r benchmarks/test_project -v


# Run all Rust tests


cargo test --release ```


## Notes


- Benchmarks should validate output correctness before reporting timing
- Performance claims require validated output comparison
- See main benchmark framework for validated comparisons


## See Also


- Test Runner README (README.md)
- Benchmark Framework (../benchmarks/README.md)
