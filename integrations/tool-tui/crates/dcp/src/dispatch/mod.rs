//! Dispatch layer for DCP tool routing.

pub mod handler;
pub mod router;

pub use handler::{SharedArgs, ToolHandler, ToolResult};
pub use router::{BinaryTrieRouter, ServerCapabilities};
