//! # DX WWW Framework
//!
//! Binary-first, multi-language web framework with file-system routing.
//!
//! DX WWW compiles `.pg` (page) and `.cp` (component) files to `.dxob` binary format
//! for zero-parse performance. It supports multiple programming languages (Rust, Python,
//! JavaScript, Go) in component scripts and integrates with dx-style for atomic CSS
//! compilation to binary CSS.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     Developer Source Code                    │
//! │  (pages/, components/, api/, layouts/, public/, styles/)    │
//! └─────────────────────┬───────────────────────────────────────┘
//!                       │
//!                       ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    DX WWW Build Pipeline                     │
//! │  Parser → Compiler → Optimizer → Binary Generator           │
//! └─────────────────────┬───────────────────────────────────────┘
//!                       │
//!                       ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Production Output                         │
//! │  (.dxob files, binary CSS, route manifest, static assets)   │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Features
//!
//! - **File-System Routing**: Create routes by adding files to `pages/` directory
//! - **Multi-Language Support**: Write component logic in Rust, Python, JavaScript, or Go
//! - **Binary Compilation**: Zero-parse `.dxob` binary format for production
//! - **Hot Reload**: Instant updates during development without full page refresh
//! - **Layout System**: Nested layouts with automatic chain composition
//! - **API Routes**: Server-side endpoints in `api/` directory
//! - **Data Loaders**: Fetch data before page rendering
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use dx_www::{DxConfig, Project, BuildPipeline};
//!
//! // Load project configuration
//! let config = DxConfig::load("dx.config.toml")?;
//!
//! // Scan project structure
//! let project = Project::scan(&config)?;
//!
//! // Build for production
//! let pipeline = BuildPipeline::new(&config);
//! let output = pipeline.build(&project).await?;
//! ```

#![doc(html_logo_url = "https://dx-www.dev/logo.svg")]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(clippy::undocumented_unsafe_blocks)]
#![warn(missing_docs)]

// =============================================================================
// Core Modules
// =============================================================================

pub mod config;
pub mod project;

// =============================================================================
// Routing System
// =============================================================================

pub mod router;

// =============================================================================
// Parsing System
// =============================================================================

pub mod parser;

// =============================================================================
// Build Pipeline
// =============================================================================

pub mod build;

// =============================================================================
// API Routes
// =============================================================================

pub mod api;

// =============================================================================
// Data Loading
// =============================================================================

pub mod data;

// =============================================================================
// Development Server
// =============================================================================

#[cfg(feature = "dev-server")]
pub mod dev;

// =============================================================================
// Static Assets
// =============================================================================

pub mod assets;

// =============================================================================
// CLI Commands
// =============================================================================

#[cfg(feature = "cli")]
pub mod cli;

// =============================================================================
// Production Build
// =============================================================================

pub mod production;

// =============================================================================
// Error Handling
// =============================================================================

pub mod error;

// =============================================================================
// Error Pages
// =============================================================================

pub mod error_pages;

// =============================================================================
// Property Tests (test-only)
// =============================================================================

#[cfg(test)]
mod property_tests;

// =============================================================================
// Public Re-exports
// =============================================================================

pub use api::ApiRouter;
pub use build::{BinaryObject, BuildOutput, BuildPipeline};
pub use config::DxConfig;
pub use data::{DataLoader, DataLoaderResult};
pub use error::{DxError, DxResult};
pub use parser::{ComponentParser, ParsedComponent};
pub use project::Project;
pub use router::FileSystemRouter;

#[cfg(feature = "dev-server")]
pub use dev::DevServer;

#[cfg(feature = "cli")]
pub use cli::Cli;

// =============================================================================
// Prelude
// =============================================================================

/// Convenient re-exports for common usage patterns.
pub mod prelude {
    pub use crate::api::{ApiRoute, ApiRouter};
    pub use crate::build::{BinaryObject, BuildOutput, BuildPipeline};
    pub use crate::config::DxConfig;
    pub use crate::data::{DataLoader, DataLoaderResult};
    pub use crate::error::{DxError, DxResult};
    pub use crate::parser::{ComponentParser, ComponentType, ParsedComponent};
    pub use crate::project::Project;
    pub use crate::router::{FileSystemRouter, Route, RoutePattern};

    #[cfg(feature = "dev-server")]
    pub use crate::dev::DevServer;
}

// =============================================================================
// Constants
// =============================================================================

/// Framework version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default configuration file name
pub const CONFIG_FILE: &str = "dx.config.toml";

/// Page file extension
pub const PAGE_EXTENSION: &str = "pg";

/// Component file extension
pub const COMPONENT_EXTENSION: &str = "cp";

/// Binary object file extension
pub const BINARY_EXTENSION: &str = "dxob";

/// Binary CSS file extension
pub const CSS_BINARY_EXTENSION: &str = "bcss";

/// Default pages directory
pub const DEFAULT_PAGES_DIR: &str = "pages";

/// Default components directory
pub const DEFAULT_COMPONENTS_DIR: &str = "components";

/// Default API directory
pub const DEFAULT_API_DIR: &str = "api";

/// Default public directory
pub const DEFAULT_PUBLIC_DIR: &str = "public";

/// Default styles directory
pub const DEFAULT_STYLES_DIR: &str = "styles";

/// Default output directory
pub const DEFAULT_OUTPUT_DIR: &str = ".dx/build";

/// Default cache directory
pub const DEFAULT_CACHE_DIR: &str = ".dx/cache";

/// Default development server port
pub const DEFAULT_DEV_PORT: u16 = 3000;

/// Binary object magic bytes
pub const DXOB_MAGIC: [u8; 4] = *b"DXOB";

/// Binary object version
pub const DXOB_VERSION: u32 = 1;
