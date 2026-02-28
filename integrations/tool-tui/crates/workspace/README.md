
# dx-workspace

Universal Development Environment Configuration for dx Ecosystem (LICENSE)

## Overview

`dx-workspace` serves as the single source of truth for development environment configuration across all code editors and cloud IDEs. Rather than maintaining dozens of scattered configuration files in different formats, dx-workspace uses a unified binary configuration that generates optimized, platform-specific configurations on demand. The philosophy aligns with dx's core principle: "Binary Everywhere."

## Features

### 🖥️ Desktop Editor Support

- VS Code / VS Codium
- Full configuration suite (settings, tasks, launch, extensions)
- Zed
- Native Rust editor with deep integration
- Neovim / Vim
- LSP and Lua configuration
- IntelliJ / Fleet
- JetBrains ecosystem support
- Helix
- Modern terminal editor configuration
- Sublime Text
- Project and build system files

### ☁️ Cloud IDE Support

- GitHub Codespaces
- Devcontainer with dx toolchain
- Gitpod
- YAML configuration with prebuilds
- CodeSandbox
- Sandbox configuration for instant demos
- Firebase Studio (IDX)
- Nix-based environment definition
- StackBlitz
- WebContainers configuration
- Replit
- Nix environment and run commands

### 📦 Container Environments

- Dev Containers
- Universal container spec
- Docker Compose
- Development service orchestration
- Nix Flakes
- Reproducible development environments

## Installation

Add to your `Cargo.toml`:
```toml
[dependencies]
dx-workspace = "0.1"
```

## Quick Start

```rust
use dx_workspace::{WorkspaceConfig, Platform, Generator};
// Detect project and create configuration let config = WorkspaceConfig::detect("./my-dx-project")?;
// Generate configurations for specific platforms let generator = Generator::new(&config);
generator.generate(Platform::VsCode)?;
generator.generate(Platform::Gitpod)?;
generator.generate(Platform::Codespaces)?;
```

## CLI Usage

When integrated with `dx-cli`:
```bash


# Initialize workspace configuration


dx workspace init


# Generate for specific platforms


dx workspace generate --platform vscode dx workspace generate --platform gitpod dx workspace generate --all


# Synchronize configurations


dx workspace sync


# Validate configuration


dx workspace validate


# Clean generated files


dx workspace clean --platform vscode dx workspace clean --all


# Export configuration


dx workspace export --format yaml ```


## Configuration Structure


dx-workspace manages several configuration categories:


### Editor Experience


- Tab size and indentation preferences
- Font and theme recommendations
- Keybinding suggestions
- Code snippets for dx patterns


### Debug & Launch Configurations


- dx-cli command execution
- WASM debugging setup
- Server attach configurations
- Compound debug configurations


### Task Automation


- Build tasks (`dx build`, `dx forge`)
- Development server (`dx dev`)
- Test runners (`dx check`)
- Custom task pipelines


### Extension Recommendations


- Core dx extensions (rust-analyzer, CodeLLDB)
- Recommended productivity tools
- Platform-specific extensions


## Generated Files



### VS Code (`.vscode/`)


@tree:.vscode[]


### Gitpod (`.gitpod.yml`)


```yaml
image: gitpod/workspace-rust tasks:
- name: dx dev
command: dx dev vscode:
extensions:
- rust-lang.rust-analyzer
```


### Codespaces (`.devcontainer/`)


@tree:.devcontainer[]


## Project Detection


dx-workspace automatically detects project features: -Cargo.toml → Rust project with dx dependencies -dx-www → Frontend component framework -dx-style → Styling system -dx-server → Backend server -dx-client → WASM runtime -dx-forge → Build pipeline


## API Reference



### WorkspaceConfig


```rust
// Create from detection let config = WorkspaceConfig::detect("./project")?;
// Create manually let mut config = WorkspaceConfig::new("my-project");
config.editor.tab_size = 2;
config.tasks = TaskConfig::dx_defaults();
// Save/Load config.save("dx-workspace.json")?;
let loaded = WorkspaceConfig::load("dx-workspace.json")?;
// Validate config.validate()?;
```


### Generator


```rust
let generator = Generator::new(&config);
// Single platform let result = generator.generate(Platform::VsCode)?;
// Multiple platforms let results = generator.generate_all();
let desktop_results = generator.generate_desktop();
let cloud_results = generator.generate_cloud();
// Check existence if generator.exists(Platform::VsCode) { generator.clean(Platform::VsCode)?;
}
```


### Platforms


```rust
// Get all platforms let all = Platform::all();
// Get by category let desktop = Platform::desktop_editors();
let cloud = Platform::cloud_ides();
let containers = Platform::container_environments();
// Check category assert!(Platform::VsCode.is_desktop());
assert!(Platform::Gitpod.is_cloud());
assert!(Platform::NixFlakes.is_container());
```


## Key Differentiators



### Single Canonical Source


One binary file generates all platform configurations. Changes propagate everywhere instantly.


### Cloud-First Philosophy


Cloud IDE support is not an afterthought. CodeSandbox, Firebase Studio, Gitpod, and Codespaces configurations are first-class citizens.


### Binary Performance


Configuration loading and generation happens at binary speed—microseconds instead of milliseconds.


### dx Ecosystem Integration


Deep integration with dx-cli, dx-forge, dx-debug means workspace configuration understands the full dx development lifecycle.


### Intelligent Defaults


Analyzes project structure and generates optimized configurations automatically based on detected dx features.


### Bidirectional Sync


Edit configurations directly in your IDE when convenient, then sync changes back to the canonical format.


## Contributing


Contributions are welcome! Please read the contributing guidelines (../../CONTRIBUTING.md) first.


## License


MIT OR Apache-2.0
