//! # dx-compat-hmr
//!
//! Hot Module Replacement compatibility layer.
//!
//! Provides:
//! - File watching with notify crate
//! - Dependency graph tracking with petgraph
//! - WebSocket-based client communication
//! - import.meta.hot API

#![warn(missing_docs)]

mod error;
mod graph;
mod server;
mod update;

pub use error::{HmrError, HmrResult};
pub use graph::{DependencyGraph, SharedDependencyGraph};
pub use server::HmrServer;
pub use update::{HmrUpdate, PropagationResult, UpdatePropagator, UpdateType};

/// import.meta.hot API.
pub mod hot;
pub use hot::{HmrRuntime, HotModule};
