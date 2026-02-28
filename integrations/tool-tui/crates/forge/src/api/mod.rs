//! # DX Forge Public API - The 132 Eternal Functions
//!
//! This module contains the complete, final, immutable public API for Forge v0.1.0.
//! All 132 functions are implemented here and organized by category.

// Core API modules
pub mod branching;
pub mod cart;
pub mod cicd;
pub mod codegen;
pub mod config;
pub mod dx_directory;
pub mod dx_experience;
pub mod events;
pub mod lifecycle;
pub mod offline;
pub mod packages;
pub mod pipeline;
pub mod reactivity;
pub mod version;

// Re-export all public API functions
pub use branching::*;
pub use cart::*;
pub use cicd::*;
pub use codegen::*;
pub use config::*;
pub use dx_directory::*;
pub use dx_experience::*;
pub use events::*;
pub use lifecycle::*;
pub use offline::*;
pub use packages::*;
pub use pipeline::*;
pub use reactivity::*;
pub use version::*;
