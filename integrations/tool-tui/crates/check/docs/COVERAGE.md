
# Code Coverage Guide

This document explains how to generate and view code coverage reports for dx-check.

## Prerequisites

Install one of the following coverage tools:

### Option 1: cargo-llvm-cov (Recommended)

```bash
cargo install cargo-llvm-cov ```


### Option 2: cargo-tarpaulin


```bash
cargo install cargo-tarpaulin ```

## Generating Coverage Reports

### Using cargo-llvm-cov

```bash
cargo llvm-cov --html -p dx-check cargo llvm-cov --json -p dx-check --output-path coverage.json cargo llvm-cov --lcov -p dx-check --output-path lcov.info cargo llvm-cov -p dx-check cargo llvm-cov --all-features -p dx-check ```


### Using cargo-tarpaulin


```bash
cargo tarpaulin -p dx-check --out Html cargo tarpaulin -p dx-check --out Lcov cargo tarpaulin -p dx-check --out Html --out Lcov --out Json ```

## Viewing Reports

### HTML Reports

After generating an HTML report, open it in your browser:
```bash
open target/llvm-cov/html/index.html open tarpaulin-report.html ```


### Coverage Thresholds


t:0(Module,Target,Coverage)[]


## CI Integration



### GitHub Actions


Add to your workflow:
```yaml
- name: Install cargo-llvm-cov
uses: taiki-e/install-action@cargo-llvm-cov
- name: Generate coverage report
run: cargo llvm-cov --lcov --output-path lcov.info -p dx-check
- name: Upload coverage to Codecov
uses: codecov/codecov-action@v3 with:
files: lcov.info fail_ci_if_error: true ```

### GitLab CI

```yaml
coverage:
stage: test script:
- cargo install cargo-llvm-cov
- cargo llvm-cov
- lcov
- output-path lcov.info
- p dx-check
coverage: /^\TOTAL.*\s+(\d+(?:\.\d+)?)%/ artifacts:
reports:
coverage_report:
coverage_format: cobertura path: coverage.xml ```


## Excluding Code from Coverage


Use `#[cfg(not(tarpaulin_include))]` or `#[coverage(off)]` (nightly) to exclude code:
```rust

#[cfg(not(tarpaulin_include))]fn debug_only_function(){}

```


## Troubleshooting



### Low Coverage Numbers


- Ensure all test files are being run
- Check that feature flags are enabled for optional code
- Verify that integration tests are included


### Missing Source Files


If source files are missing from the report:
```bash
RUSTFLAGS="-C instrument-coverage" cargo build -p dx-check ```

### Slow Coverage Generation

For faster iteration:
```bash
cargo llvm-cov --lib -p dx-check -- test_name cargo llvm-cov --lib --bins -p dx-check ```
