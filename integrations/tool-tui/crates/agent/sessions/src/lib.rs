//! Session management and persistence for DX Agent.
//!
//! Provides per-user session isolation, context window management,
//! SQLite-backed persistence, and session compaction.

pub mod context;
pub mod manager;
pub mod storage;

pub use context::ContextWindow;
pub use manager::{AgentRouter, RoutingStrategy, SessionManager};
pub use storage::SessionStorage;
