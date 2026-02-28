//! Platform-native I/O abstraction layer for DX Forge.
//!
//! This module provides a unified async I/O interface that automatically selects
//! the most performant backend for the current platform:
//! - Linux: io_uring (kernel 5.1+)
//! - macOS: kqueue
//! - Windows: IOCP (I/O Completion Ports)
//! - Fallback: tokio async I/O
//!
//! # Example
//! ```rust,ignore
//! use dx_forge::platform_io::{create_platform_io, PlatformIO};
//!
//! let io = create_platform_io();
//! let data = io.read_all(Path::new("file.txt")).await?;
//! ```

use anyhow::Result;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::time::Instant;

// Re-export submodules
mod fallback;
pub use fallback::FallbackBackend;

#[cfg(target_os = "linux")]
mod io_uring;
#[cfg(target_os = "linux")]
pub use io_uring::IoUringBackend;

#[cfg(target_os = "macos")]
mod kqueue;
#[cfg(target_os = "macos")]
pub use kqueue::KqueueBackend;

#[cfg(target_os = "windows")]
mod iocp;
#[cfg(target_os = "windows")]
pub use iocp::IocpBackend;

mod selector;
pub use selector::{create_platform_io, create_platform_io_with_fallback_tracking};

// ============================================================================
// Core Traits
// ============================================================================

/// Platform-native I/O operations trait.
///
/// This trait provides a unified interface for file I/O operations across
/// different platform backends. All implementations are async and thread-safe.
#[async_trait]
pub trait PlatformIO: Send + Sync {
    /// Read file contents into buffer.
    ///
    /// Returns the number of bytes read.
    async fn read(&self, path: &Path, buf: &mut [u8]) -> Result<usize>;

    /// Write buffer contents to file.
    ///
    /// Returns the number of bytes written.
    async fn write(&self, path: &Path, buf: &[u8]) -> Result<usize>;

    /// Read entire file into Vec.
    async fn read_all(&self, path: &Path) -> Result<Vec<u8>>;

    /// Write entire buffer to file, creating or truncating as needed.
    async fn write_all(&self, path: &Path, buf: &[u8]) -> Result<()>;

    /// Watch path for changes, returns event stream.
    async fn watch(&self, path: &Path) -> Result<Box<dyn EventStream>>;

    /// Batch read multiple files (optimized for io_uring).
    ///
    /// Returns a Vec of file contents in the same order as input paths.
    async fn batch_read(&self, paths: &[PathBuf]) -> Result<Vec<Vec<u8>>>;

    /// Batch write multiple files.
    async fn batch_write(&self, ops: &[WriteOp]) -> Result<()>;

    /// Get backend name for diagnostics.
    fn backend_name(&self) -> &'static str;

    /// Check if this backend supports the current platform.
    fn is_available() -> bool
    where
        Self: Sized;
}

// ============================================================================
// Write Operation
// ============================================================================

/// Write operation for batch writes.
#[derive(Debug, Clone)]
pub struct WriteOp {
    /// Target file path.
    pub path: PathBuf,
    /// Data to write.
    pub data: Vec<u8>,
    /// Whether to fsync after write.
    pub sync: bool,
}

impl WriteOp {
    /// Create a new write operation.
    pub fn new(path: impl Into<PathBuf>, data: Vec<u8>) -> Self {
        Self {
            path: path.into(),
            data,
            sync: false,
        }
    }

    /// Create a new write operation with fsync.
    pub fn with_sync(path: impl Into<PathBuf>, data: Vec<u8>) -> Self {
        Self {
            path: path.into(),
            data,
            sync: true,
        }
    }
}

// ============================================================================
// Event Stream
// ============================================================================

/// Event stream for file watching.
///
/// This trait uses a polling-based approach for dyn compatibility.
pub trait EventStream: Send {
    /// Poll for the next file event.
    ///
    /// Returns `Some(event)` if an event is available, `None` if the stream is closed.
    fn poll_next(&mut self) -> Option<FileEvent>;

    /// Close the event stream and release resources.
    fn close(&mut self);

    /// Check if there are pending events without blocking.
    fn has_pending(&self) -> bool;
}

/// Boxed event stream type alias for convenience.
pub type BoxedEventStream = Box<dyn EventStream>;

// ============================================================================
// File Events
// ============================================================================

/// File system event.
#[derive(Debug, Clone)]
pub struct FileEvent {
    /// Path of the affected file or directory.
    pub path: PathBuf,
    /// Kind of event that occurred.
    pub kind: FileEventKind,
    /// Timestamp when the event was detected.
    pub timestamp: Instant,
}

impl FileEvent {
    /// Create a new file event.
    pub fn new(path: impl Into<PathBuf>, kind: FileEventKind) -> Self {
        Self {
            path: path.into(),
            kind,
            timestamp: Instant::now(),
        }
    }
}

/// Kind of file system event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileEventKind {
    /// File or directory was created.
    Created,
    /// File was modified.
    Modified,
    /// File or directory was deleted.
    Deleted,
    /// File or directory was renamed.
    Renamed {
        /// Original path before rename.
        from: PathBuf,
    },
    /// File metadata changed (permissions, timestamps, etc.).
    Metadata,
}

// ============================================================================
// Platform Detection Types
// ============================================================================

/// Detected operating system platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    /// Linux operating system.
    Linux,
    /// macOS operating system.
    MacOS,
    /// Windows operating system.
    Windows,
    /// Unknown or unsupported platform.
    Unknown,
}

impl Platform {
    /// Detect the current platform at runtime.
    pub fn current() -> Self {
        #[cfg(target_os = "linux")]
        {
            Platform::Linux
        }
        #[cfg(target_os = "macos")]
        {
            Platform::MacOS
        }
        #[cfg(target_os = "windows")]
        {
            Platform::Windows
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            Platform::Unknown
        }
    }

    /// Get the platform name as a string.
    pub fn name(&self) -> &'static str {
        match self {
            Platform::Linux => "linux",
            Platform::MacOS => "macos",
            Platform::Windows => "windows",
            Platform::Unknown => "unknown",
        }
    }
}

/// I/O backend type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IoBackend {
    /// Linux io_uring backend.
    IoUring,
    /// macOS kqueue backend.
    Kqueue,
    /// Windows IOCP backend.
    Iocp,
    /// Fallback tokio-based backend.
    Fallback,
}

impl IoBackend {
    /// Get the backend name as a string.
    pub fn name(&self) -> &'static str {
        match self {
            IoBackend::IoUring => "io_uring",
            IoBackend::Kqueue => "kqueue",
            IoBackend::Iocp => "iocp",
            IoBackend::Fallback => "fallback",
        }
    }
}

/// Platform and I/O backend information.
#[derive(Debug, Clone)]
pub struct PlatformInfo {
    /// Detected platform.
    pub platform: Platform,
    /// Active I/O backend.
    pub backend: IoBackend,
    /// Kernel version (if available).
    pub kernel_version: Option<String>,
    /// Supported features.
    pub features: std::collections::HashSet<String>,
}

impl PlatformInfo {
    /// Create platform info for the current system.
    pub fn current(backend: IoBackend) -> Self {
        Self {
            platform: Platform::current(),
            backend,
            kernel_version: Self::detect_kernel_version(),
            features: Self::detect_features(backend),
        }
    }

    fn detect_kernel_version() -> Option<String> {
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;
            Command::new("uname")
                .arg("-r")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
        }
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            Command::new("uname")
                .arg("-r")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
        }
        #[cfg(target_os = "windows")]
        {
            // Windows version detection would go here
            None
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            None
        }
    }

    fn detect_features(backend: IoBackend) -> std::collections::HashSet<String> {
        let mut features = std::collections::HashSet::new();

        match backend {
            IoBackend::IoUring => {
                features.insert("batch_submit".to_string());
                features.insert("sqpoll".to_string());
                features.insert("iopoll".to_string());
            }
            IoBackend::Kqueue => {
                features.insert("vnode_events".to_string());
                features.insert("timer_events".to_string());
            }
            IoBackend::Iocp => {
                features.insert("overlapped_io".to_string());
                features.insert("directory_changes".to_string());
            }
            IoBackend::Fallback => {
                features.insert("cross_platform".to_string());
            }
        }

        features
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = Platform::current();

        #[cfg(target_os = "linux")]
        assert_eq!(platform, Platform::Linux);

        #[cfg(target_os = "macos")]
        assert_eq!(platform, Platform::MacOS);

        #[cfg(target_os = "windows")]
        assert_eq!(platform, Platform::Windows);
    }

    #[test]
    fn test_platform_name() {
        assert_eq!(Platform::Linux.name(), "linux");
        assert_eq!(Platform::MacOS.name(), "macos");
        assert_eq!(Platform::Windows.name(), "windows");
        assert_eq!(Platform::Unknown.name(), "unknown");
    }

    #[test]
    fn test_backend_name() {
        assert_eq!(IoBackend::IoUring.name(), "io_uring");
        assert_eq!(IoBackend::Kqueue.name(), "kqueue");
        assert_eq!(IoBackend::Iocp.name(), "iocp");
        assert_eq!(IoBackend::Fallback.name(), "fallback");
    }

    #[test]
    fn test_write_op_creation() {
        let op = WriteOp::new("test.txt", vec![1, 2, 3]);
        assert_eq!(op.path, PathBuf::from("test.txt"));
        assert_eq!(op.data, vec![1, 2, 3]);
        assert!(!op.sync);

        let op_sync = WriteOp::with_sync("test.txt", vec![4, 5, 6]);
        assert!(op_sync.sync);
    }

    #[test]
    fn test_file_event_creation() {
        let event = FileEvent::new("test.txt", FileEventKind::Modified);
        assert_eq!(event.path, PathBuf::from("test.txt"));
        assert_eq!(event.kind, FileEventKind::Modified);
    }

    #[test]
    fn test_platform_info() {
        let info = PlatformInfo::current(IoBackend::Fallback);
        assert_eq!(info.platform, Platform::current());
        assert_eq!(info.backend, IoBackend::Fallback);
        assert!(info.features.contains("cross_platform"));
    }
}
