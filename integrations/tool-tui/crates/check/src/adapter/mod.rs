//! Tool Adapter Pattern for Third-Party Integration
//!
//! This module provides a trait-based abstraction for integrating external
//! formatters, linters, and analyzers without modifying core code.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                        ToolRegistry                               │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐              │
//! │  │  Rustfmt    │  │   Ruff      │  │  Prettier   │  ...         │
//! │  │  Adapter    │  │  Adapter    │  │  Adapter    │              │
//! │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘              │
//! │         │                │                │                      │
//! │         └────────────────┼────────────────┘                      │
//! │                          │                                       │
//! │                    ToolAdapter Trait                             │
//! └──────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_check::adapter::{ToolAdapter, ToolRegistry, ToolCapabilities};
//!
//! let registry = ToolRegistry::new(Default::default());
//! if let Some(adapter) = registry.get_adapter_for_extension("rs") {
//!     let result = adapter.lint(path, content)?;
//!     for diag in result.diagnostics {
//!         println!("{}", diag);
//!     }
//! }
//! ```

pub mod discovery;
pub mod registry;
pub mod tools;
mod traits;

pub use discovery::{DiscoveryResult, ToolDiscovery};
pub use registry::{ToolConfig, ToolRegistry};
pub use tools::*;
pub use traits::{
    AdapterOutput, OutputParser, ToolAdapter, ToolCapabilities, ToolError, ToolErrorKind,
    ToolResult,
};

use std::path::Path;

/// Convenience function to get a formatted output from any supported file
pub fn format_file(
    registry: &ToolRegistry,
    path: &Path,
    content: &[u8],
) -> Result<ToolResult, ToolError> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    registry
        .get_adapter_for_extension(ext)
        .ok_or_else(|| {
            ToolError::new(
                ToolErrorKind::UnsupportedLanguage,
                format!("No adapter for extension: {ext}"),
            )
        })?
        .format(path, content)
}

/// Convenience function to lint any supported file
pub fn lint_file(
    registry: &ToolRegistry,
    path: &Path,
    content: &[u8],
) -> Result<ToolResult, ToolError> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    registry
        .get_adapter_for_extension(ext)
        .ok_or_else(|| {
            ToolError::new(
                ToolErrorKind::UnsupportedLanguage,
                format!("No adapter for extension: {ext}"),
            )
        })?
        .lint(path, content)
}
