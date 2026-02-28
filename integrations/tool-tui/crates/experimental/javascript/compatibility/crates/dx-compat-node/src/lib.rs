//! # dx-compat-node
//!
//! Node.js API compatibility layer providing implementations for core Node.js modules.
//!
//! ## Modules
//!
//! - `fs` - File system operations with memory-mapped I/O optimization
//! - `path` - Cross-platform path manipulation
//! - `buffer` - Binary data handling with zero-copy optimization
//! - `stream` - Streaming data with backpressure support
//! - `events` - Event emitter pattern
//! - `http` - HTTP server and client
//! - `crypto` - Cryptographic operations
//! - `child_process` - Process spawning
//! - `process` - Process object (env, argv, cwd, exit, platform, arch)

#![warn(missing_docs)]

pub mod buffer;
pub mod child_process;
pub mod crypto;
pub mod events;
pub mod fs;
pub mod http;
pub mod path;
pub mod process;
pub mod stream;

/// Common error types for Node.js compatibility.
pub mod error;

pub use error::{ErrorCode, NodeError};
