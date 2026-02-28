//! # dx-compat-bun
//!
//! Bun-specific APIs compatibility layer providing high-performance implementations.
//!
//! ## Modules
//!
//! - `serve` - Bun.serve() HTTP server
//! - `file` - Bun.file() and Bun.write() file operations
//! - `spawn` - Bun.spawn() process spawning
//! - `hash` - Bun hashing functions
//! - `password` - Bun.password hashing
//! - `compression` - Bun compression functions

#![warn(missing_docs)]

pub mod compression;
pub mod file;
pub mod hash;
pub mod password;
pub mod serve;
pub mod spawn;

/// Common error types for Bun compatibility.
pub mod error;

pub use error::BunError;
