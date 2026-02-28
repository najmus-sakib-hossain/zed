//! State Module - Dirty-Bit Tracking & Atomic Sync
//!
//! O(1) change detection inspired by dx-style's dirty-bit system.

mod atomic_sync;
mod dirty_bits;
mod shared_rules;
mod snapshot;

pub use atomic_sync::{AtomicSync, SyncState};
pub use dirty_bits::{DirtyBits, DirtyMask};
pub use shared_rules::{RuleRef, SharedRules};
pub use snapshot::{RuleSnapshot, SnapshotManager};

/// State module version
pub const STATE_VERSION: u8 = 1;
