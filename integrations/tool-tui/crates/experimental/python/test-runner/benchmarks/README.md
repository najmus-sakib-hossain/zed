
# DX-Py Test Runner Benchmarks

Benchmark suite for comparing DX-Py test runner against pytest.

## Status: In Development

Benchmarks require output validation before performance claims can be made.

## Running Benchmarks

```bash


# Build DX-Py


cargo build --release


# Run discovery benchmark


target/release/dx-py discover -r benchmarks/test_project


# Run full test benchmark


target/release/dx-py test -r benchmarks/test_project -v ```


## Directory Structure


@tree:benchmarks[]


## Notes


- Benchmarks should validate output correctness before reporting timing
- Results may vary based on system load and hardware configuration
- See main benchmark framework for validated comparisons


## See Also


- Test Runner README (../README.md)
- Main Benchmark Framework (../../benchmarks/README.md)
