
# Changelog

All notable changes to dx-generator will be documented in this file. The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [Unreleased]

## [0.2.0] - 2024-12-30

### Added

- CLI Integration: Full `dx gen` command suite
- `dx gen run`
- Generate from templates with parameters
- `dx gen list`
- List available templates with filtering
- `dx gen init`
- Initialize template directory with examples
- `dx gen compile`
- Compile Handlebars templates to binary.dxt format
- `dx gen scaffold`
- Multi-file project scaffolding
- `dx gen watch`
- Watch mode for automatic regeneration
- `dx gen stats`
- View generation metrics and token savings
- `dx gen registry`
- Search, install, and publish templates
- Smart Placeholder System
- Type-aware placeholders (PascalCase, snake_case, camelCase, etc.)
- Transform pipeline (lowercase, uppercase, pluralize, singularize, etc.)
- Default values for optional parameters
- Dependency resolution with topological sort
- Template Registry
- Local template discovery from `.dx/templates/`
- Template metadata with parameter schemas
- Ed25519 signature verification for security
- Search by name, tags, and description
- XOR Differential Patching
- Incremental file updates with binary diff
- 95% reduction in disk writes for regeneration
- Automatic fallback to full write on patch failure
- Protected Region Preservation
- `// @dx:preserve` markers for manual code sections
- Automatic merging of protected content during regeneration
- Support for multiple named regions
- AI Agent Protocol
- JSON-RPC interface for programmatic access
- `generate`, `list`, and `schema` methods
- Intelligent defaults from context (directory name, git config)
- Structured error responses with suggestions
- Metrics Tracking
- Generation time and bytes tracking
- Token savings estimation
- Per-template statistics
- Persistent storage in `.dx/stats.json`
- Fusion Mode (Multi-File Scaffolding)
- Bundle definitions with multiple templates
- Conditional file inclusion
- Path placeholder resolution
- Post-generation hooks
- VS Code Extension Integration
- Trigger-based generation (`//gen:component`)
- Context menu and command palette integration
- Hover preview for triggers
- Token savings status bar
- Example Templates
- `component.dxt.hbs`
- React/Vue component template
- `model.dxt.hbs`
- Data model template
- `api-client.dxt.hbs`
- API client template
- `test.dxt.hbs`
- Test file template
- Scaffold Bundles
- `rust-crate`
- Full Rust crate with lib, tests, and docs
- `react-component`
- Component with tests and Storybook story
- `api-endpoint`
- API route with handler and tests

### Changed

- Improved binary template format with metadata block
- Enhanced error messages with suggestions
- Better alignment handling in binary deserialization

### Fixed

- Bytemuck alignment issues in template deserialization
- Property test stability for compilation round-trip

## [0.1.0] - 2024-01-01

### Added

- Initial implementation of binary template format (.dxt)
- SIMD placeholder detection
- Dual-mode engine (Micro/Macro)
- DX ∞ parameter encoding
- Dirty-bit template caching
- Stack-only generation
- Integer token system
- Capability-based security with Ed25519 signing
- Compile-time validation
