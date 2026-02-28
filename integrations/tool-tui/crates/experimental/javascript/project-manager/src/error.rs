//! Error types for dx-js-project-manager
//!
//! Defines all error types used throughout the project management system.

use std::path::PathBuf;
use thiserror::Error;

/// Errors related to workspace operations
#[derive(Error, Debug)]
pub enum WorkspaceError {
    /// Binary Workspace Manifest file not found
    #[error("workspace manifest not found at {path}")]
    ManifestNotFound { path: PathBuf },

    /// Manifest file is corrupted or invalid
    #[error("workspace manifest corrupted: {reason}")]
    ManifestCorrupted { reason: String, hash_mismatch: bool },

    /// Referenced package does not exist
    #[error("package not found: {name}")]
    PackageNotFound { name: String },

    /// Cyclic dependency detected in the workspace
    #[error("cyclic dependency detected: {}", cycle.join(" -> "))]
    CyclicDependency { cycle: Vec<String> },

    /// Invalid magic bytes in manifest
    #[error("invalid manifest magic bytes: expected DXWM, got {found:?}")]
    InvalidMagic { found: [u8; 4] },

    /// Version mismatch
    #[error("manifest version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: u32, found: u32 },

    /// IO error during workspace operations
    #[error("workspace IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors related to task execution
#[derive(Error, Debug)]
pub enum TaskError {
    /// Task not found in the graph
    #[error("task not found: {package}:{task}")]
    TaskNotFound { package: String, task: String },

    /// A dependency task failed
    #[error("dependency task {task_idx} failed: {reason}")]
    DependencyFailed { task_idx: u32, reason: String },

    /// Task exceeded its frame budget
    #[error("task {task_idx} exceeded frame budget: {elapsed_us}us > {budget_us}us")]
    FrameBudgetExceeded {
        task_idx: u32,
        elapsed_us: u64,
        budget_us: u64,
    },

    /// Task execution failed with non-zero exit code
    #[error("task execution failed with exit code {exit_code}: {stderr}")]
    ExecutionFailed { exit_code: i32, stderr: String },

    /// Invalid magic bytes in task graph
    #[error("invalid task graph magic bytes: expected DXTG, got {found:?}")]
    InvalidMagic { found: [u8; 4] },

    /// Task graph file not found
    #[error("task graph not found at {path}")]
    GraphNotFound { path: PathBuf },

    /// IO error during task operations
    #[error("task IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors related to cache operations
#[derive(Error, Debug)]
pub enum CacheError {
    /// Cache entry not found (cache miss)
    #[error("cache entry not found for hash {}", hex::encode(hash))]
    EntryNotFound { hash: [u8; 32] },

    /// Ed25519 signature verification failed
    #[error("cache entry signature invalid")]
    SignatureInvalid,

    /// Content hash doesn't match stored hash
    #[error("cache integrity check failed: content hash mismatch")]
    IntegrityCheckFailed,

    /// Cache storage is full
    #[error("cache storage full: {used_bytes} / {max_bytes} bytes")]
    StorageFull { used_bytes: u64, max_bytes: u64 },

    /// Invalid magic bytes in cache entry
    #[error("invalid cache magic bytes: expected DXC\\0, got {found:?}")]
    InvalidMagic { found: [u8; 4] },

    /// IO error during cache operations
    #[error("cache IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors related to lockfile operations
#[derive(Error, Debug)]
pub enum LockfileError {
    /// Lockfile not found
    #[error("lockfile not found at {path}")]
    NotFound { path: PathBuf },

    /// Lockfile is corrupted
    #[error("lockfile corrupted: {reason}")]
    Corrupted { reason: String },

    /// CRDT merge conflict that couldn't be auto-resolved
    #[error("lockfile merge conflict: {}", conflicts.join(", "))]
    MergeConflict { conflicts: Vec<String> },

    /// Lockfile format version mismatch
    #[error("lockfile version mismatch: expected {expected}, found {found}")]
    VersionMismatch { expected: u32, found: u32 },

    /// Invalid magic bytes in lockfile
    #[error("invalid lockfile magic bytes: expected DXLW, got {found:?}")]
    InvalidMagic { found: [u8; 4] },

    /// IO error during lockfile operations
    #[error("lockfile IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors related to remote cache operations
#[derive(Error, Debug)]
pub enum RemoteError {
    /// Connection to remote cache failed
    #[error("remote cache connection failed: {reason}")]
    ConnectionFailed { reason: String },

    /// Authentication failed
    #[error("remote cache authentication failed")]
    AuthenticationFailed,

    /// Transfer was interrupted
    #[error("transfer interrupted at byte {checkpoint}")]
    TransferInterrupted { checkpoint: u64 },

    /// Server returned an error
    #[error("remote cache server error: {status} - {message}")]
    ServerError { status: u16, message: String },

    /// IO error during remote operations
    #[error("remote IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors related to ghost dependency detection
#[derive(Error, Debug)]
pub enum ScanError {
    /// Failed to read source file
    #[error("failed to read source file {path}: {reason}")]
    ReadFailed { path: PathBuf, reason: String },

    /// Failed to parse imports
    #[error("failed to parse imports in {path}: {reason}")]
    ParseFailed { path: PathBuf, reason: String },

    /// IO error during scanning
    #[error("scan IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors related to watch mode
#[derive(Error, Debug)]
pub enum WatchError {
    /// Failed to start file watcher
    #[error("failed to start file watcher: {reason}")]
    StartFailed { reason: String },

    /// Watch event error
    #[error("watch event error: {reason}")]
    EventError { reason: String },

    /// IO error during watch operations
    #[error("watch IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Helper module for hex encoding (used in error messages)
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_error_display() {
        let err = WorkspaceError::PackageNotFound {
            name: "my-package".to_string(),
        };
        assert_eq!(err.to_string(), "package not found: my-package");
    }

    #[test]
    fn test_task_error_display() {
        let err = TaskError::FrameBudgetExceeded {
            task_idx: 5,
            elapsed_us: 20000,
            budget_us: 16000,
        };
        assert!(err.to_string().contains("exceeded frame budget"));
    }

    #[test]
    fn test_cache_error_display() {
        let err = CacheError::SignatureInvalid;
        assert_eq!(err.to_string(), "cache entry signature invalid");
    }

    #[test]
    fn test_lockfile_error_display() {
        let err = LockfileError::MergeConflict {
            conflicts: vec!["pkg-a".to_string(), "pkg-b".to_string()],
        };
        assert!(err.to_string().contains("pkg-a, pkg-b"));
    }
}
