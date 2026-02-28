//! Core types for dx-js-project-manager
//!
//! Defines shared data structures used across all components.

use bytemuck::{Pod, Zeroable};
use std::path::PathBuf;

// ============================================================================
// Package Entry (BWM)
// ============================================================================

/// Package entry in the Binary Workspace Manifest.
/// Fixed-size (32 bytes) for O(1) indexing via memory mapping.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, PartialEq, Eq)]
pub struct PackageEntry {
    /// Index into string table for package name
    pub name_idx: u32,
    /// Index into string table for package path
    pub path_idx: u32,
    /// Package version encoded as (major << 20) | (minor << 10) | patch
    pub version_packed: u32,
    /// Offset to dependency list in the manifest
    pub deps_offset: u32,
    /// Number of dependencies
    pub deps_count: u16,
    /// Offset to script definitions
    pub scripts_offset: u32,
    /// Number of scripts
    pub scripts_count: u16,
    /// Flags: is_private (bit 0), has_bin (bit 1), etc.
    pub flags: u16,
    /// Padding for alignment
    pub _padding: u16,
}

impl PackageEntry {
    /// Size of PackageEntry in bytes
    pub const SIZE: usize = std::mem::size_of::<Self>();

    /// Create a new package entry
    pub fn new(
        name_idx: u32,
        path_idx: u32,
        version: (u16, u16, u16),
        deps_offset: u32,
        deps_count: u16,
    ) -> Self {
        Self {
            name_idx,
            path_idx,
            version_packed: Self::pack_version(version.0, version.1, version.2),
            deps_offset,
            deps_count,
            scripts_offset: 0,
            scripts_count: 0,
            flags: 0,
            _padding: 0,
        }
    }

    /// Pack version components into a single u32
    #[inline]
    pub fn pack_version(major: u16, minor: u16, patch: u16) -> u32 {
        ((major as u32) << 20) | ((minor as u32 & 0x3FF) << 10) | (patch as u32 & 0x3FF)
    }

    /// Unpack version from packed u32
    #[inline]
    pub fn unpack_version(packed: u32) -> (u16, u16, u16) {
        let major = (packed >> 20) as u16;
        let minor = ((packed >> 10) & 0x3FF) as u16;
        let patch = (packed & 0x3FF) as u16;
        (major, minor, patch)
    }

    /// Get unpacked version tuple
    #[inline]
    pub fn version(&self) -> (u16, u16, u16) {
        Self::unpack_version(self.version_packed)
    }

    /// Check if package is private
    #[inline]
    pub fn is_private(&self) -> bool {
        self.flags & 0x01 != 0
    }

    /// Check if package has binary
    #[inline]
    pub fn has_bin(&self) -> bool {
        self.flags & 0x02 != 0
    }

    /// Set private flag
    #[inline]
    pub fn set_private(&mut self, private: bool) {
        if private {
            self.flags |= 0x01;
        } else {
            self.flags &= !0x01;
        }
    }

    /// Set has_bin flag
    #[inline]
    pub fn set_has_bin(&mut self, has_bin: bool) {
        if has_bin {
            self.flags |= 0x02;
        } else {
            self.flags &= !0x02;
        }
    }
}

// ============================================================================
// Task Entry (BTG)
// ============================================================================

/// Task entry in the Binary Task Graph.
/// Fixed-size (40 bytes) for O(1) access via memory mapping.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, PartialEq, Eq)]
pub struct TaskEntry {
    /// Index into string table for task name (e.g., "build", "test")
    pub name_idx: u32,
    /// Package index this task belongs to
    pub package_idx: u32,
    /// Index into string table for command
    pub command_idx: u32,
    /// Offset to input glob patterns
    pub inputs_offset: u32,
    /// Number of input patterns
    pub inputs_count: u16,
    /// Offset to output glob patterns
    pub outputs_offset: u32,
    /// Number of output patterns
    pub outputs_count: u16,
    /// Pre-computed hash of task definition (first 8 bytes of Blake3)
    pub definition_hash: [u8; 8],
    /// Flags: cacheable (bit 0), persistent (bit 1), etc.
    pub flags: u16,
    /// Frame budget in microseconds (0 = unlimited)
    pub frame_budget_us: u32,
    /// Padding for alignment
    pub _padding: u16,
}

impl TaskEntry {
    /// Size of TaskEntry in bytes
    pub const SIZE: usize = std::mem::size_of::<Self>();

    /// Create a new task entry
    pub fn new(name_idx: u32, package_idx: u32, command_idx: u32) -> Self {
        Self {
            name_idx,
            package_idx,
            command_idx,
            inputs_offset: 0,
            inputs_count: 0,
            outputs_offset: 0,
            outputs_count: 0,
            definition_hash: [0; 8],
            flags: 0,
            frame_budget_us: 0,
            _padding: 0,
        }
    }

    /// Check if task output is cacheable
    #[inline]
    pub fn is_cacheable(&self) -> bool {
        self.flags & 0x01 != 0
    }

    /// Check if task is persistent (long-running)
    #[inline]
    pub fn is_persistent(&self) -> bool {
        self.flags & 0x02 != 0
    }

    /// Set cacheable flag
    #[inline]
    pub fn set_cacheable(&mut self, cacheable: bool) {
        if cacheable {
            self.flags |= 0x01;
        } else {
            self.flags &= !0x01;
        }
    }

    /// Set persistent flag
    #[inline]
    pub fn set_persistent(&mut self, persistent: bool) {
        if persistent {
            self.flags |= 0x02;
        } else {
            self.flags &= !0x02;
        }
    }

    /// Check if task has a frame budget
    #[inline]
    pub fn has_frame_budget(&self) -> bool {
        self.frame_budget_us > 0
    }
}

// ============================================================================
// Task Instance (Stack-allocated for zero-allocation execution)
// ============================================================================

/// Task state during execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TaskState {
    /// Task is pending execution
    Pending = 0,
    /// Task is currently running
    Running = 1,
    /// Task completed successfully
    Completed = 2,
    /// Task failed
    Failed = 3,
    /// Task yielded due to frame budget
    Yielded = 4,
}

/// Task instance for execution.
/// Stack-allocated (96 bytes) for zero-allocation task creation.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TaskInstance {
    /// Index into task graph
    pub task_idx: u32,
    /// Start time in nanoseconds (from monotonic clock)
    pub start_time_ns: u64,
    /// Current state
    pub state: TaskState,
    /// Inline storage for small outputs (avoids heap allocation)
    pub inline_output: [u8; 64],
    /// Length of inline output used
    pub inline_len: u8,
    /// Padding for alignment
    _padding: [u8; 14],
}

impl TaskInstance {
    /// Size of TaskInstance in bytes
    pub const SIZE: usize = std::mem::size_of::<Self>();

    /// Maximum inline output size
    pub const MAX_INLINE_OUTPUT: usize = 64;

    /// Create a new task instance
    #[inline]
    pub fn new(task_idx: u32) -> Self {
        Self {
            task_idx,
            start_time_ns: 0,
            state: TaskState::Pending,
            inline_output: [0; 64],
            inline_len: 0,
            _padding: [0; 14],
        }
    }

    /// Start the task (record start time)
    #[inline]
    pub fn start(&mut self, now_ns: u64) {
        self.start_time_ns = now_ns;
        self.state = TaskState::Running;
    }

    /// Get elapsed time in microseconds
    #[inline]
    pub fn elapsed_us(&self, now_ns: u64) -> u64 {
        (now_ns.saturating_sub(self.start_time_ns)) / 1000
    }

    /// Write inline output (returns false if output too large)
    #[inline]
    pub fn write_inline(&mut self, data: &[u8]) -> bool {
        if data.len() > Self::MAX_INLINE_OUTPUT {
            return false;
        }
        self.inline_output[..data.len()].copy_from_slice(data);
        self.inline_len = data.len() as u8;
        true
    }

    /// Get inline output slice
    #[inline]
    pub fn inline_output(&self) -> &[u8] {
        &self.inline_output[..self.inline_len as usize]
    }
}

// ============================================================================
// File Hash
// ============================================================================

/// File hash with metadata for change detection.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable, PartialEq, Eq)]
pub struct FileHash {
    /// Hash of the file path (for quick comparison)
    pub path_hash: u64,
    /// Blake3 hash of file content
    pub content_hash: [u8; 32],
    /// File size in bytes
    pub size: u64,
    /// Modification time in nanoseconds since epoch
    pub mtime_ns: u64,
}

impl FileHash {
    /// Size of FileHash in bytes
    pub const SIZE: usize = std::mem::size_of::<Self>();

    /// Create a new file hash
    pub fn new(path_hash: u64, content_hash: [u8; 32], size: u64, mtime_ns: u64) -> Self {
        Self {
            path_hash,
            content_hash,
            size,
            mtime_ns,
        }
    }

    /// Check if content has changed compared to another hash
    #[inline]
    pub fn content_changed(&self, other: &FileHash) -> bool {
        self.content_hash != other.content_hash
    }

    /// Check if metadata has changed (size or mtime)
    #[inline]
    pub fn metadata_changed(&self, other: &FileHash) -> bool {
        self.size != other.size || self.mtime_ns != other.mtime_ns
    }
}

// ============================================================================
// Import Statement (for SIMD detection)
// ============================================================================

/// Detected import statement
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportStatement {
    /// Import type
    pub kind: ImportKind,
    /// Module specifier (e.g., "lodash", "./utils")
    pub specifier: String,
    /// Line number (1-indexed)
    pub line: u32,
    /// Column number (1-indexed)
    pub column: u32,
}

/// Type of import statement
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportKind {
    /// ES6 import: `import x from 'y'`
    Es6Import,
    /// ES6 export from: `export { x } from 'y'`
    Es6ExportFrom,
    /// CommonJS require: `require('y')`
    CommonJsRequire,
    /// Dynamic import: `import('y')`
    DynamicImport,
}

// ============================================================================
// Ghost Dependency
// ============================================================================

/// A ghost (undeclared) dependency
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhostDependency {
    /// Package name being imported
    pub package_name: String,
    /// File containing the import
    pub importing_file: PathBuf,
    /// Line number of import
    pub line: u32,
    /// Column of import
    pub column: u32,
}

// ============================================================================
// Performance Metrics
// ============================================================================

/// Performance tracking for validation
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    /// Workspace load time in microseconds
    pub workspace_load_us: u64,
    /// Task graph load time in microseconds
    pub task_graph_load_us: u64,
    /// Cache lookup time in microseconds
    pub cache_lookup_us: u64,
    /// File hash time for N files in microseconds
    pub file_hash_us: u64,
    /// Number of files hashed
    pub file_hash_count: u32,
    /// Affected detection time in microseconds
    pub affected_detection_us: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_entry_size() {
        // Ensure PackageEntry is packed (28 bytes with packing)
        assert_eq!(PackageEntry::SIZE, 28);
    }

    #[test]
    fn test_task_entry_size() {
        // Ensure TaskEntry is exactly 40 bytes
        assert_eq!(TaskEntry::SIZE, 40);
    }

    #[test]
    fn test_task_instance_size() {
        // Ensure TaskInstance fits in cache line (96 bytes)
        assert!(TaskInstance::SIZE <= 96);
    }

    #[test]
    fn test_file_hash_size() {
        // Ensure FileHash is exactly 56 bytes
        assert_eq!(FileHash::SIZE, 56);
    }

    #[test]
    fn test_version_packing() {
        let packed = PackageEntry::pack_version(1, 2, 3);
        let (major, minor, patch) = PackageEntry::unpack_version(packed);
        assert_eq!((major, minor, patch), (1, 2, 3));

        // Test edge cases
        let packed = PackageEntry::pack_version(4095, 1023, 1023);
        let (major, minor, patch) = PackageEntry::unpack_version(packed);
        assert_eq!((major, minor, patch), (4095, 1023, 1023));
    }

    #[test]
    fn test_package_entry_flags() {
        let mut entry = PackageEntry::new(0, 0, (1, 0, 0), 0, 0);
        assert!(!entry.is_private());
        assert!(!entry.has_bin());

        entry.set_private(true);
        assert!(entry.is_private());
        assert!(!entry.has_bin());

        entry.set_has_bin(true);
        assert!(entry.is_private());
        assert!(entry.has_bin());

        entry.set_private(false);
        assert!(!entry.is_private());
        assert!(entry.has_bin());
    }

    #[test]
    fn test_task_entry_flags() {
        let mut entry = TaskEntry::new(0, 0, 0);
        assert!(!entry.is_cacheable());
        assert!(!entry.is_persistent());

        entry.set_cacheable(true);
        assert!(entry.is_cacheable());

        entry.set_persistent(true);
        assert!(entry.is_persistent());
    }

    #[test]
    fn test_task_instance_inline_output() {
        let mut instance = TaskInstance::new(0);
        assert!(instance.write_inline(b"hello"));
        assert_eq!(instance.inline_output(), b"hello");

        // Test max size
        let large_data = [0u8; 64];
        assert!(instance.write_inline(&large_data));

        // Test too large
        let too_large = [0u8; 65];
        assert!(!instance.write_inline(&too_large));
    }

    #[test]
    fn test_file_hash_comparison() {
        let hash1 = FileHash::new(123, [1; 32], 1000, 12345);
        let hash2 = FileHash::new(123, [1; 32], 1000, 12345);
        let hash3 = FileHash::new(123, [2; 32], 1000, 12345);
        let hash4 = FileHash::new(123, [1; 32], 2000, 12345);

        assert!(!hash1.content_changed(&hash2));
        assert!(hash1.content_changed(&hash3));
        assert!(!hash1.metadata_changed(&hash2));
        assert!(hash1.metadata_changed(&hash4));
    }
}
