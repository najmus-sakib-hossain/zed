
# Contributing to DX Serializer

Thank you for your interest in contributing! This document provides guidelines and instructions.

## Table of Contents

- Code of Conduct
- Getting Started
- Development Workflow
- Architecture
- Testing
- Submitting Changes

## Code of Conduct

We are committed to providing a welcoming and inclusive environment. Please: -Be respectful and constructive -Focus on what is best for the community -Show empathy towards other contributors -Accept constructive criticism gracefully

## Getting Started

### Prerequisites

- Rust: 1.75+ (stable)
- Cargo: Latest version
- Git: For version control

### Clone Repository

```bash
git clone https://github.com/dx-www/dx-serializer cd dx-serializer/crates/dx-serializer ```


### Build Project


```bash
cargo build cargo test cargo bench ```

## Development Workflow

### 1. Create Feature Branch

```bash
git checkout -b feature/your-feature-name ```


### 2. Make Changes


Follow our architecture principles.


### 3. Run Tests


```bash
cargo test cargo fmt -- --check cargo clippy -- -D warnings ```

### 4. Update Documentation

Update relevant docs in `docs/` if needed.

### 5. Commit Changes

```bash
git add .
git commit -m "feat: add inline prefixing operator"
```
Use conventional commits: -`feat:` — New feature -`fix:` — Bug fix -`docs:` — Documentation -`perf:` — Performance improvement -`refactor:` — Code refactoring -`test:` — Tests -`chore:` — Maintenance

### 6. Push Branch

```bash
git push origin feature/your-feature-name ```


### 7. Create Pull Request


Go to GitHub and create a PR with: -Clear title and description -Reference related issues -Include benchmark results if relevant


## Architecture



### Modules


@tree:src[]


### Architecture Principles


- Zero-Copy First
- Operate on `&[u8]` slices
- Avoid allocations unless necessary
- Use `Cow<'static, str>` for strings
- SIMD When Possible
- Use `memchr::memchr()` for byte scanning
- Leverage CPU vector instructions
- Type-Guided
- Schema hints eliminate ambiguity
- Enable zero-copy vacuum parsing
- Memory Safety
- Minimize `unsafe` blocks
- Document safety invariants
- Prefer safe abstractions
- Minimal Dependencies
- Core: `memchr`, `rustc-hash`, `bytemuck`
- Test: `criterion` (benches only)
- Avoid heavyweight dependencies


### Data Flow


```
Input (&[u8])
↓ Tokenizer (SIMD)
↓ Token<'a> Stream ↓ Parser (Schema-Guided)
↓ DxValue (Zero-Copy)
```


## Testing



### Unit Tests


Located in `src/*.rs` files:
```rust

#[cfg(test)]

mod tests { use super::*;

#[test]

fn test_feature() { // Test implementation assert_eq!(parse(b"test:value").unwrap(), expected);
}
}
```
Run: `cargo test`


### Integration Tests


Located in `tests/`:
```rust
// tests/integration.rs

#[test]

fn test_round_trip() { let data = parse(input).unwrap();
let encoded = encode(&data).unwrap();
let parsed = parse(&encoded).unwrap();
assert_eq!(data, parsed);
}
```


### Benchmarks


Located in `benches/`:
```rust
// benches/parser.rs use criterion::{black_box, criterion_group, criterion_main, Criterion};
fn bench_parse(c: &mut Criterion) { c.bench_function("parse_simple", |b| { b.iter(|| parse(black_box(INPUT)))
});
}
criterion_group!(benches, bench_parse);
criterion_main!(benches);
```
Run: `cargo bench`


### Test Coverage


Aim for: -Lines: 80%+ -Branches: 70%+ -Functions: 90%+ Check coverage:
```bash
cargo tarpaulin --out Html ```

## Code Style

### Formatting

Use `rustfmt`:
```bash
cargo fmt ```


### Linting


Use `clippy`:
```bash
cargo clippy -- -D warnings ```

### Naming Conventions

- Types: `PascalCase` (e.g., `DxValue`)
- Functions: `snake_case` (e.g., `parse_table`)
- Constants: `SCREAMING_SNAKE_CASE` (e.g., `MAX_DEPTH`)
- Modules: `snake_case` (e.g., `tokenizer`)

### Documentation

All public APIs must have:
```rust
/// Parses DX format into typed structures.
/// /// # Arguments /// * `input` - DX format bytes (UTF-8)
/// /// # Returns /// * `Ok(DxValue)` - Parsed data /// * `Err(DxError)` - Parse error with position /// /// # Examples /// ```
/// let data = parse(b"name:Alice")?;
/// ```
pub fn parse(input: &[u8]) -> Result<DxValue, DxError> { // Implementation }
```

## Submitting Changes

### Pull Request Checklist

- Tests pass (`cargo test`)
- Benchmarks run (`cargo bench`)
- Code formatted (`cargo fmt`)
- No lint warnings (`cargo clippy`)
- Documentation updated
- Changelog updated (if applicable)
- Commit messages follow conventions

### Review Process

- Automated Checks: CI runs tests, lints, benchmarks
- Code Review: Maintainer reviews code
- Feedback: Address review comments
- Approval: Maintainer approves PR
- Merge: Squash and merge to main

### After Merge

- Your contribution will be credited
- Changelog updated with your changes
- Next release will include your work

## Feature Requests

Have an idea? Open an issue: -Search: Check if idea already exists -Create Issue: Use feature request template -Describe: What problem does it solve? -Discuss: Maintainers will discuss feasibility

## Bug Reports

Found a bug? Help us fix it: -Search: Check if bug already reported -Create Issue: Use bug report template -Reproduce: Minimal reproduction steps -Context: Rust version, OS, error message

### Bug Report Template

```markdown
**Describe the bug** Clear description of the issue.
**To Reproduce** 1. Code snippet 2. Expected behavior 3. Actual behavior **Environment**
- Rust version: 1.75
- OS: Ubuntu 22.04
- dx-serializer version: 0.1.0
**Additional context** Error messages, stack traces, etc.
```

## Performance Contributions

When optimizing: -Benchmark First: Measure current performance -Optimize: Make changes -Benchmark Again: Verify improvement -Document: Explain optimization

### Benchmark Results

Include in PR:
```
Before:
parse_simple 1.9 µs ± 0.05 µs After:
parse_simple 1.1 µs ± 0.03 µs Improvement: 42% faster ```


## Documentation Contributions


Improve docs? Great! -Typos: Just fix and PR -Clarifications: Improve existing docs -Examples: Add code examples -Guides: Write tutorials All docs in `docs/` directory.


## Questions?


- GitHub Issues: Ask questions
- Discussions: Community forum
- Email: dev@dx-www.com


## License


By contributing, you agree that your contributions will be licensed under the MIT License. Thank you for contributing to DX Serializer! 🦀⚡
