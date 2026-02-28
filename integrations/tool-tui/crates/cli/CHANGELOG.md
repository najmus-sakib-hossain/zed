
# Changelog

All notable changes to dx-cli will be documented in this file. The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [Unreleased]

## [0.1.0] - 2026-01-08

### Added

- Initial production-ready release
- Unified CLI for all DX ecosystem tools
- Forge daemon control (`forge start`, `forge stop`, `forge status`)
- Tool management (`tools list`, `tools run`, `tools enable`, `tools disable`)
- Branch control (`branch status`, `branch approve`, `branch reject`)
- Configuration management (`config show`, `config set`, `config reset`)
- Cache management (`cache clean`, `cache info`)
- Code checking and linting (`check lint`, `check format`)
- DX Markdown commands (`dxm convert`, `dxm render`, `dxm validate`)
- DX Serializer commands (`sr convert`, `sr tokens`, `sr benchmark`)
- Icon utilities (`icon search`, `icon list`, `icon get`, `icon component`)
- Font utilities (`font search`, `font download`, `font list`, `font info`)
- Media processing (`media search`, `media download`, `media process`)
- Code generation (`gen run`, `gen list`, `gen compile`, `gen scaffold`)
- Driven sync (`driven init`, `driven sync`, `driven convert`)
- DCP server (`dcp serve`, `dcp convert`, `dcp validate`)
- WWW framework (`www new`, `www component`, `www build`, `www dev`)
- Security scanning (`security scan`, `security audit`)
- File conversion (`convert json`, `convert yaml`, `convert toml`)
- Token analysis (`tokens analyze`, `tokens stats`)
- Structured logging with `--verbose` and `--quiet` flags
- Graceful shutdown handling (Ctrl+C)
- Centralized constants module for configuration defaults
- Unified output formatting (JSON, Table, Simple)
- Version compatibility checking for daemon communication
- Atomic file write utilities for safe file operations

### Changed

- Consolidated output formatting into single `output.rs` module
- Centralized error handling with proper exit codes
- Standardized all commands to return `anyhow::Result`

### Security

- Added security scanning commands for vulnerability detection
- Implemented atomic file writes to prevent data corruption
