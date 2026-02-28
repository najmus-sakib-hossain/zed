//! DX configuration file parsing
//!
//! Provides configuration loading from dx.toml with support for:
//! - Custom config paths via --config flag (Requirement 12.2)
//! - Error reporting with line numbers (Requirement 12.3)
//! - Binary caching for faster subsequent loads (Requirement 12.4)
//! - Field validation (Requirement 4.1)
//! - Unknown field detection (Requirement 4.3)
//! - Config merging (Requirement 4.5)
//! - Atomic save with backup (Requirement 4.6, 4.7)

pub mod cache;
pub mod loader;
pub mod merge;
pub mod save;
pub mod types;
pub mod validation;

#[cfg(test)]
mod tests;

pub use types::*;
