//! Version control and tool registry
//!
//! Provides semantic versioning, version requirements, tool registry management,
//! and Git-like version control with snapshots and branching.

pub mod registry;
pub mod snapshot;
pub mod types;

pub use registry::{ToolInfo, ToolRegistry, ToolSource};
pub use snapshot::{
    Branch, FileSnapshot, Snapshot, SnapshotDiff, SnapshotId, SnapshotManager, ToolState,
};
pub use types::{Version, VersionReq};
