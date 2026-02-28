# Changelog

All notable changes to the DX VS Code extension will be documented in this file.

The format is based on [Keep a Changelog], and this project adheres to [Semantic Versioning].

## [Unreleased]

## [0.3.0] - 2024-12-30

### Added

- **Generator Integration**
  - `DX: Generate from Template` command for template-based code generation
  - Context menu integration for quick generation
  - `DX: List Templates` command to browse available templates
  - `DX: Search Templates` command for finding templates
  - `DX: Refresh Templates` command to reload template list
  - `DX: Show Generator Stats` command for metrics

- **Trigger System**
  - Automatic generation on trigger patterns (e.g., `//gen:component`)
  - Configurable trigger patterns via settings
  - Content replacement on Enter key

- **Hover Preview**
  - Preview generated output on hover over triggers
  - Shows template parameters and description

- **Token Savings Status Bar**
  - Displays token savings after generation
  - Cumulative savings tracking

- **Configuration Options**
  - `dx.generator.templatePaths` - Custom template search paths
  - `dx.generator.enableTriggers` - Toggle trigger-based generation
  - `dx.generator.triggerPatterns` - Custom trigger patterns
  - `dx.generator.showTokenSavings` - Toggle status bar display
  - `dx.generator.enableHoverPreview` - Toggle hover preview
  - `dx.generator.autoRefresh` - Auto-refresh template list

### Changed

- Updated command palette with generator commands
- Enhanced status bar with generator metrics

## [0.2.0] - 2024-06-01

### Added

- DX Style CSS viewer
- Phantom Mode for hiding shadow files
- Binary cache generation
- Token details viewer
- Hologram View for Markdown preview

### Changed

- Improved Markdown table rendering
- Enhanced syntax highlighting

## [0.1.0] - 2024-01-01

### Added

- Initial release
- DX language support with syntax highlighting
- Forge daemon integration
- dx-check linting integration
- Markdown format support
- Color themes (Light and Dark)
