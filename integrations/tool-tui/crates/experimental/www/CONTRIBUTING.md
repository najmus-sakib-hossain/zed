
# Contributing to dx-www

Thank you for your interest in contributing to dx-www! This document provides guidelines and instructions for contributing.

## Code of Conduct

Please be respectful and constructive in all interactions. We're building something together.

## Getting Started

### Prerequisites

- Rust 1.85 or later
- Git
- A GitHub account

### Development Setup

- Fork the repository on GitHub
- Clone your fork:
```bash
git clone https://github.com/YOUR_USERNAME/dx-www.git cd dx-www ```
- Add the upstream remote:
```bash
git remote add upstream https://github.com/dx-www/dx-www.git ```
- Build the project:
```bash
cargo build ```
- Run tests:
```bash
cargo test ```

### Project Structure

@tree:dx-www[]

## Making Changes

### Branch Naming

- `feature/description`
- New features
- `fix/description`
- Bug fixes
- `docs/description`
- Documentation changes
- `refactor/description`
- Code refactoring

### Commit Messages

Follow conventional commits:
```
type(scope): description
[optional body]
[optional footer]
```
Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore` Examples: -`feat(parser): add support for JSX fragments` -`fix(reactor): handle EAGAIN on epoll` -`docs(readme): update installation instructions`

### Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Follow Rust naming conventions
- Add doc comments to public items

### Testing

- Write tests for new functionality
- Ensure all tests pass: `cargo test`
- Add property-based tests for core logic
- Run benchmarks if performance-critical: `cargo bench`

## Pull Request Process

- Create a feature branch:
```bash
git checkout -b feature/my-feature ```
- Make your changes and commit:
```bash
git add .
git commit -m "feat(scope): description"
```
- Push to your fork:
```bash
git push origin feature/my-feature ```
- Open a Pull Request on GitHub
- Fill out the PR template with:
- Description of changes
- Related issues
- Testing performed
- Screenshots (if UI changes)
- Wait for review and address feedback

### PR Requirements

- All tests pass
- Code is formatted (`cargo fmt`)
- No clippy warnings (`cargo clippy`)
- Documentation updated (if applicable)
- CHANGELOG.md updated (for user-facing changes)

## Reporting Issues

### Bug Reports

Include: -dx-www version -Rust version -Operating system -Steps to reproduce -Expected vs actual behavior -Error messages or logs

### Feature Requests

Include: -Use case description -Proposed solution -Alternatives considered

## Development Tips

### Running Specific Tests

```bash


# Run tests for a specific crate


cargo test -p dx-www-compiler


# Run a specific test


cargo test test_name


# Run tests with output


cargo test -- --nocapture ```


### Debugging


```bash

# Build with debug symbols

cargo build

# Run with RUST_BACKTRACE

RUST_BACKTRACE=1 cargo test ```

### Benchmarking

```bash


# Run all benchmarks


cargo bench


# Run specific benchmark


cargo bench --bench parser_benchmarks ```


### Documentation


```bash

# Build docs

cargo doc --workspace --no-deps

# Open in browser

cargo doc --workspace --no-deps --open ```

## Architecture Decisions

Major architectural changes should be discussed in an issue first. Include: -Problem statement -Proposed solution -Alternatives considered -Migration path (if breaking)

## Release Process

Releases are managed by maintainers. The process: -Update version in `Cargo.toml` -Update `CHANGELOG.md` -Create git tag: `git tag v0.x.0` -Push tag: `git push origin v0.x.0` -CI publishes to crates.io

## Getting Help

- GitHub Discussions
- Discord
- Stack Overflow

## License

By contributing, you agree that your contributions will be licensed under the MIT OR Apache-2.0 license.
