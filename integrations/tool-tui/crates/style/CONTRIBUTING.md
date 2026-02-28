# Contributing to dx-style

## Code Organization

- Target file size: 500-800 lines
- Hard limit: 2000 lines
- Use mod.rs pattern for module organization
- One responsibility per file

## Development

```bash
# Build
cargo build -p dx-style

# Test
cargo test -p dx-style

# Format
cargo fmt --manifest-path crates/style/Cargo.toml

# Lint
cargo clippy -p dx-style

# Benchmark
cargo bench -p dx-style
```

## Code Quality

- No unwrap/expect in production code
- Proper error handling with Result types
- Document all public APIs
- Add tests for new features
- Fix all clippy warnings
