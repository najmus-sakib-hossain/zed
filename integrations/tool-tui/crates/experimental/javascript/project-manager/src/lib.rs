//! # dx-js-project-manager
//!
//! Binary-first project management system achieving 30-100x performance improvements
//! over traditional JSON-based tools (pnpm workspaces, Turborepo).
//!
//! ## Architecture
//!
//! The system eliminates all parsing overhead through:
//!
//! - **Binary Workspace Manifest (BWM)**: Memory-mapped workspace structure
//! - **Binary Task Graph (BTG)**: Pre-compiled task pipelines
//! - **DXC Cache Format**: Zero-copy task output caching
//! - **DXL-Workspace Lockfile**: O(1) dependency resolution
//! - **SIMD Change Detection**: Blake3 hashing with AVX2 pattern matching

pub mod error;
pub mod types;

// Binary format modules
pub mod bag;
pub mod btg;
pub mod bwm;
pub mod dxc;
pub mod dxl;

// Core components
pub mod affected;
pub mod cache;
pub mod change;
pub mod cli;
pub mod executor;
pub mod fusion;
pub mod ghost;
pub mod remote;
pub mod watch;
pub mod workspace;

// Property-based tests
#[cfg(test)]
mod proptest_tests;

// Re-exports
pub use affected::AffectedDetector;
pub use cache::CacheManager;
pub use change::ChangeDetector;
pub use cli::{Cli, CliResult, Command};
pub use dxl::LockfileResolver;
pub use error::{CacheError, LockfileError, TaskError, WorkspaceError};
pub use executor::TaskExecutor;
pub use fusion::FusionAnalyzer;
pub use ghost::GhostDetector;
pub use remote::RemoteCacheClient;
pub use types::{FileHash, PackageEntry, TaskEntry, TaskInstance};
pub use watch::WatchManager;
pub use workspace::WorkspaceManager;

/// Magic bytes for Binary Workspace Manifest
pub const BWM_MAGIC: [u8; 4] = *b"DXWM";

/// Magic bytes for Binary Task Graph
pub const BTG_MAGIC: [u8; 4] = *b"DXTG";

/// Magic bytes for DXC Cache
pub const DXC_MAGIC: [u8; 4] = *b"DXC\0";

/// Magic bytes for DXL-Workspace Lockfile
pub const DXL_MAGIC: [u8; 4] = *b"DXLW";

/// Magic bytes for Binary Affected Graph
pub const BAG_MAGIC: [u8; 4] = *b"DXAG";

/// Current format version
pub const FORMAT_VERSION: u32 = 1;
