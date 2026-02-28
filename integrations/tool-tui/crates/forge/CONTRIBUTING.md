
# Contributing to DX Forge

Thank you for your interest in contributing to DX Forge! This document provides guidelines and instructions for contributing.

## Code of Conduct

Be respectful, inclusive, and constructive in all interactions.

## Getting Started

- Fork the repository on GitHub
- Clone your fork locally:```bash git clone //github.com/YOUR_USERNAME/forge.git cd forge
```
- Add upstream remote:```bash
git remote add upstream https://github.com/najmus-sakib-hossain/forge.git ```


## Development Setup



### Prerequisites


- Rust 1.70 or later
- Cargo
- Git


### Building


```bash
cargo build ```

### Running Tests

```bash


# All tests


cargo test


# Integration tests only


cargo test --test integration_test


# With logging


RUST_LOG=debug cargo test


# Specific test


cargo test test_orchestrator_priority_ordering ```


### Running Examples


```bash
cargo run --example simple cargo run --example full_workflow ```

## Making Changes

### Branch Naming

- `feature/description`
- New features
- `fix/description`
- Bug fixes
- `docs/description`
- Documentation updates
- `refactor/description`
- Code refactoring

### Commit Messages

Follow conventional commits format:
```
type(scope): subject body (optional)
footer (optional)
```
Types: -`feat`: New feature -`fix`: Bug fix -`docs`: Documentation -`style`: Formatting -`refactor`: Code restructuring -`test`: Adding tests -`chore`: Maintenance Example:
```
feat(orchestrator): add parallel execution support Implements wave-based parallel execution for tools that have no dependencies on each other. Adds OrchestratorConfig.parallel flag.
Closes #123 ```


## Pull Request Process


- Update your fork:
```bash
git fetch upstream git rebase upstream/main ```
- Make your changes in a feature branch
- Add tests for new functionality
- Run tests and linting:
```bash
cargo test cargo clippy -- -D warnings cargo fmt --check ```
- Update documentation:
- Add rustdoc comments for public APIs
- Update README.md if needed
- Update CHANGELOG.md
- Submit pull request:
- Provide clear description
- Reference related issues
- Ensure CI passes


## Code Style



### Rust Guidelines


- Follow Rust API Guidelines
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Write idiomatic Rust code


### Documentation


- Document all public APIs with rustdoc
- Include examples in documentation
- Keep docs up to date with code changes


### Testing


- Write unit tests for all new functionality
- Add integration tests for complex features
- Maintain test coverage above 70%


## Project Structure


@tree:forge[]


## Adding New Features



### New DX Tool Integration


- Implement the `DxTool` trait
- Add example in `examples/`
- Document usage in README
- Add integration test


### Core Features


- Discuss design in GitHub issue first
- Maintain backward compatibility
- Update API documentation
- Add comprehensive tests


## Release Process


- Update version in `Cargo.toml`
- Update `CHANGELOG.md`
- Create git tag: `git tag v0.x.x`
- Push tag: `git push
- tags`
- CI will publish to crates.io


## Questions?


- Open an issue for bugs or feature requests
- Use discussions for questions
- Check existing issues and docs first


## License


By contributing, you agree that your contributions will be licensed under either: -Apache License, Version 2.0 -MIT License at your option. Thank you for contributing to DX Forge! 🎉
