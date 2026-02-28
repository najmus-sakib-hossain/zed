
# Contributing to dx-cli

Thank you for your interest in contributing to dx-cli!

## Getting Started

- Fork the repository
- Clone your fork: `git clone //github.com/YOUR_USERNAME/dx.git`
- Create a branch: `git checkout
- b feature/your-feature`
- Make your changes
- Run tests: `cargo test`
- Commit your changes: `git commit
- am 'Add some feature'`
- Push to the branch: `git push origin feature/your-feature`
- Submit a pull request

## Development Setup

```bash
curl --proto '=https' --tlsv1.2 -sSf sh.rustup.rs | sh
cargo build cargo test cargo clippy ```


## Code Style


- Follow Rust standard formatting (`cargo fmt`)
- Ensure all tests pass (`cargo test`)
- Ensure no clippy warnings (`cargo clippy`)
- Add documentation for public APIs
- Write tests for new functionality


## Reporting Issues


- Use the GitHub issue tracker
- Include steps to reproduce
- Include expected vs actual behavior
- Include Rust version and OS


## License


By contributing, you agree that your contributions will be licensed under the MIT OR Apache-2.0 license.
