
# Contributing to DX Markdown

Thank you for your interest in contributing to DX Markdown! Note: This crate is part of the Dx monorepo. Please refer to the workspace-level contribution guidelines in the root `CONTRIBUTING.md` for general policies, code of conduct, and CI/CD workflows.

## Development Setup

### Prerequisites

- Rust 1.85+ (2024 Edition)
- Cargo workspace environment (clone the full Dx monorepo)

### Building

```bash


# From workspace root


cargo build -p dx-markdown


# Run tests


cargo test -p dx-markdown


# Run benchmarks


cargo bench -p dx-markdown


# Check code quality


cargo clippy -p dx-markdown cargo fmt -p dx-markdown --check ```


## Code Quality Standards



### Testing


- All new features must include unit tests
- Property-based tests for core algorithms (use proptest)
- Integration tests for format conversions
- Benchmark tests for performance-critical code
- Aim for 85%+ line coverage (enforced at workspace level)


### Documentation


- Public APIs must have rustdoc comments with examples
- Include examples in documentation
- Update README.md for user-facing changes
- Update CHANGELOG.md following Keep a Changelog format
- Document all unsafe code with SAFETY comments


### Code Style


- Follow Rust 2024 Edition idioms
- Max line width: 100 characters
- Use `cargo fmt` for formatting
- Address all `cargo clippy` warnings
- Workspace lints are enforced (deny unwrap_used, expect_used in production code)
- Document all `unsafe` blocks with SAFETY comments


### Error Handling


- Use `Result` types, avoid `.unwrap()` and `.expect()` in library code
- Tests can use `#[allow(clippy::unwrap_used)]` as needed
- Provide detailed error messages with context
- Include suggestions for fixing errors when possible


## Monorepo Context


This crate is part of the Dx workspace: -CI/CD: Managed at workspace level (see root `.github/workflows/`) -Dependencies: Shared workspace dependencies in root `Cargo.toml` -Linting: Workspace-level clippy configuration applies -Testing: Coverage enforcement at workspace level -Security: Dependency audits run at workspace level When contributing: -Run workspace-level tests: `cargo test --workspace` -Check workspace-level lints: `cargo clippy --workspace` -Ensure changes don't break other crates


## Testing Guidelines



### Running Tests


```bash

# Run all tests

cargo test -p dx-markdown

# Run specific test module

cargo test -p dx-markdown --lib compiler::tests

# Run with output

cargo test -p dx-markdown -- --nocapture

# Run benchmarks

cargo bench -p dx-markdown ```

### Test Coverage

We maintain high test coverage through comprehensive unit, integration, and property-based tests.
```bash


# Generate coverage report


cargo install cargo-llvm-cov cargo llvm-cov -p dx-markdown --html


# Check coverage threshold (enforced in CI at 85%)


cargo llvm-cov -p dx-markdown --fail-under-lines 85 ```


### Unit Tests


Place unit tests in the same file as the code:
```rust

#[cfg(test)]

mod tests { use super::*;

#[test]

fn test_feature() { // Test implementation }
}
```


### Property-Based Tests


Use proptest for algorithmic correctness:
```rust

#[cfg(test)]

mod prop_tests { use super::*;
use proptest::prelude::*;
proptest! {

#[test]

fn prop_invariant(input in any::<String>()) { // Property test }
}
}
```


### Integration Tests


Place integration tests in `tests/`: @tree:tests[]


### Fuzzing


Fuzz targets are in `fuzz/fuzz_targets/`:
```bash

# Install cargo-fuzz

cargo install cargo-fuzz

# Run fuzzing

cargo fuzz run compiler_fuzz cargo fuzz run tokenizer_fuzz cargo fuzz run parser_fuzz ```

## Benchmarking

Add benchmarks to `benches/`:
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
fn benchmark_feature(c: &mut Criterion) { c.bench_function("feature_name", |b| { b.iter(|| { // Benchmark code });
});
}
criterion_group!(benches, benchmark_feature);
criterion_main!(benches);
```

## Performance Considerations

- Minimize allocations in hot paths
- Use `String::with_capacity` when size is known
- Prefer iterators over collecting to vectors
- Profile before optimizing

## Submitting Changes

### Pull Request Process

- Create a feature branch from `main`
- Make your changes with clear commit messages
- Add tests for new functionality
- Update documentation
- Run the full test suite
- Submit a pull request with a clear description

### Commit Messages

Follow conventional commits:
```
feat: add table inline detection fix: preserve code block formatting docs: update API examples perf: optimize whitespace collapsing test: add property tests for idempotence ```


### Code Review


- All changes require review
- Address reviewer feedback promptly
- Keep PRs focused and reasonably sized
- Squash commits before merging


## Questions?


- Open an issue for bugs or feature requests
- Check existing issues before creating new ones
- Be respectful and constructive in discussions


## License


By contributing, you agree that your contributions will be licensed under the same terms as the project (MIT/Apache-2.0). See the workspace root for full license texts.
