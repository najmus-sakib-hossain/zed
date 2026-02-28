//! # dx-workspace: Universal Development Environment Configuration
//!
//! `dx-workspace` serves as the single source of truth for development environment
//! configuration across all code editors and cloud IDEs. Rather than maintaining
//! dozens of scattered configuration files in different formats, dx-workspace uses
//! a unified binary configuration that generates optimized, platform-specific
//! configurations on demand.

// Allow dead_code for API completeness
#![allow(dead_code)]

//! ## Philosophy
//!
//! The philosophy aligns with dx's core principle: **"Binary Everywhere."**
//! Your workspace configuration lives in dx's compact binary format, and the tool
//! generates whatever format each platform requires—VS Code's JSON, Gitpod's YAML,
//! Codespaces' devcontainer specs, and more.
//!
//! ## What dx-workspace Handles
//!
//! - **Editor Experience Configuration** - Keybindings, snippets, themes, fonts
//! - **Debugging & Run Configurations** - Launch configs, debug adapters
//! - **Task Automation** - Build tasks, dev server, test runners
//! - **Extension Recommendations** - Curated extensions per platform
//! - **Project Structure Intelligence** - File associations, search exclusions
//!
//! ## Supported Platforms
//!
//! ### Desktop Editors
//! - VS Code / VS Codium
//! - Zed
//! - Neovim / Vim
//! - IntelliJ / Fleet
//! - Helix
//! - Sublime Text
//!
//! ### Cloud IDEs
//! - GitHub Codespaces
//! - Gitpod
//! - CodeSandbox
//! - Firebase Studio (IDX)
//! - StackBlitz
//! - Replit
//! - Glitch
//! - CodeAnywhere
//! - AWS Cloud9
//!
//! ### Container Environments
//! - Dev Containers
//! - Docker Compose
//! - Podman
//! - Nix Flakes
//!
//! ## Example
//!
//! ```rust,no_run
//! use dx_workspace::{WorkspaceConfig, Platform, Generator};
//!
//! // Load or create workspace configuration
//! let config = WorkspaceConfig::detect("./my-dx-project")?;
//!
//! // Generate configurations for specific platforms
//! let generator = Generator::new(&config);
//! generator.generate(Platform::VsCode)?;
//! generator.generate(Platform::Gitpod)?;
//! generator.generate(Platform::Codespaces)?;
//! # Ok::<(), dx_workspace::Error>(())
//! ```

#![warn(missing_docs)]
#![deny(unsafe_code)]

pub mod binary;
pub mod commands;
pub mod config;
pub mod error;
pub mod generator;
pub mod platforms;
pub mod project;

// Re-exports for convenience
pub use binary::{BINARY_EXTENSION, load_binary, save_binary, validate_binary};
pub use config::WorkspaceConfig;
pub use error::{Error, Result};
pub use generator::Generator;
pub use platforms::Platform;
pub use project::ProjectDetector;
