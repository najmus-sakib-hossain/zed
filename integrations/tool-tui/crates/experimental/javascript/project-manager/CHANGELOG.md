
# Changelog

All notable changes to dx-js-project-manager will be documented in this file. The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [Unreleased]

### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security

## [0.0.1] - 2025-12-29

Initial early development release. Many features are experimental or incomplete.

### Added

- Build Task Graph (BTG): Efficient task dependency management
- Build Workspace Manager (BWM): Multi-project workspace support
- Affected Detection: Smart detection of affected projects based on changes
- Remote Cache: Distributed build cache support
- Ghost Dependencies: Detection and reporting of undeclared dependencies
- Fusion Builds: Optimized parallel build execution
- DXC Format: Efficient binary cache format
- DXL Lockfile: Deterministic dependency locking
- Property-Based Tests: Comprehensive property tests for core functionality

### Changed

- Improved CLI with comprehensive help output
- Enhanced error messages with actionable information

### Fixed

- Workspace detection edge cases
- Cache invalidation accuracy

### Security

- No known vulnerabilities (verified with `cargo audit`)

## [0.0.0] - 2024-01-01

### Added

- Initial prototype of dx-js-project-manager
- Basic workspace management
- Task execution framework
- File watching support
