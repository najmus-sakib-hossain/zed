//! DX Forge Daemon - Binary Dawn Edition
//!
//! A persistent daemon that orchestrates all DX tools with:
//! - Dual-nature watchers (LSP + FileSystem)
//! - Background task processing
//! - Tool lifecycle management
//! - R2 cloud sync
//! - VS Code extension integration
//!
//! The LSP server functionality requires the "daemon" feature for axum support.
//!
//! Architecture:
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │                     FORGE DAEMON (Binary Dawn)                    │
//! ├──────────────────────────────────────────────────────────────────┤
//! │  ┌─────────────────────┐     ┌─────────────────────────────┐    │
//! │  │   LSP Watcher       │     │   FileSystem Watcher        │    │
//! │  │   (Primary)         │     │   (Fallback)                │    │
//! │  │   - VS Code events  │     │   - notify-debouncer        │    │
//! │  │   - Semantic info   │     │   - Recursive watch         │    │
//! │  └─────────┬───────────┘     └──────────────┬──────────────┘    │
//! │            │                                │                    │
//! │            └──────────────┬─────────────────┘                    │
//! │                           ▼                                      │
//! │  ┌────────────────────────────────────────────────────────────┐ │
//! │  │              UNIFIED CHANGE STREAM                          │ │
//! │  │  (Deduplication + Pattern Detection + Priority Queue)       │ │
//! │  └───────────────────────────┬────────────────────────────────┘ │
//! │                              ▼                                   │
//! │  ┌────────────────────────────────────────────────────────────┐ │
//! │  │              TOOL ORCHESTRATOR                              │ │
//! │  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐          │ │
//! │  │  │ bundler │ │ style   │ │ test    │ │ www     │ ...      │ │
//! │  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘          │ │
//! │  └───────────────────────────┬────────────────────────────────┘ │
//! │                              ▼                                   │
//! │  ┌────────────────────────────────────────────────────────────┐ │
//! │  │              BACKGROUND WORKER POOL                         │ │
//! │  │  - Cache warming    - R2 sync     - Pattern analysis       │ │
//! │  │  - Package prefetch - Cleanup     - Metrics                │ │
//! │  └────────────────────────────────────────────────────────────┘ │
//! └──────────────────────────────────────────────────────────────────┘
//! ```

pub mod core;
pub mod lsp;
#[cfg(feature = "daemon")]
pub mod lsp_server;
pub mod server;
pub mod state;
pub mod worker;

pub use core::{DaemonConfig, DaemonEvent, DaemonState, ForgeDaemon};
pub use lsp::{LspBridge, LspMessage, LspNotification};
#[cfg(feature = "daemon")]
pub use lsp_server::{LspRequest, LspResponse, LspServer, LspServerState};
pub use server::{DaemonServer, IpcCommand, IpcResponse};
pub use state::{DaemonStateManager, ProjectState, ToolState, ToolStatus};
pub use worker::{TaskPriority, WorkerPool, WorkerTask};
