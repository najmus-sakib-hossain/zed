//! File system watching functionality.
//!
//! This module provides Node.js `fs.watch()` and `fs.watchFile()` compatibility
//! using the `notify` crate for native OS file system events.

use crate::error::{ErrorCode, NodeError, NodeResult};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;

/// Watch event types matching Node.js fs.watch events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEventType {
    /// File content was modified.
    Change,
    /// File was renamed or deleted.
    Rename,
}

/// Watch event data matching Node.js fs.watch callback parameters.
#[derive(Debug, Clone)]
pub struct WatchEvent {
    /// The type of event that occurred.
    pub event_type: WatchEventType,
    /// The filename that triggered the event (relative to watched path).
    pub filename: Option<String>,
}

/// Error event for file watching.
#[derive(Debug, Clone)]
pub struct WatchError {
    /// Error code.
    pub code: ErrorCode,
    /// Error message.
    pub message: String,
    /// Path that caused the error.
    pub path: Option<PathBuf>,
}

impl From<notify::Error> for WatchError {
    fn from(err: notify::Error) -> Self {
        let code = match err.kind {
            notify::ErrorKind::PathNotFound => ErrorCode::ENOENT,
            notify::ErrorKind::WatchNotFound => ErrorCode::ENOENT,
            notify::ErrorKind::MaxFilesWatch => ErrorCode::UNKNOWN, // Too many open files
            _ => ErrorCode::UNKNOWN,
        };
        WatchError {
            code,
            message: err.to_string(),
            path: err.paths.first().cloned(),
        }
    }
}

/// File system watcher matching Node.js fs.FSWatcher.
pub struct FSWatcher {
    /// The underlying notify watcher.
    watcher: RecommendedWatcher,
    /// Set of watched paths.
    watched_paths: Arc<Mutex<HashSet<PathBuf>>>,
    /// Whether the watcher is closed.
    closed: Arc<Mutex<bool>>,
    /// Event receiver for async iteration.
    event_rx: mpsc::UnboundedReceiver<Result<WatchEvent, WatchError>>,
}

impl FSWatcher {
    /// Create a new file system watcher.
    ///
    /// # Arguments
    /// * `path` - The path to watch.
    /// * `recursive` - Whether to watch subdirectories recursively.
    ///
    /// # Returns
    /// A new FSWatcher instance.
    pub fn new(path: impl AsRef<Path>, recursive: bool) -> NodeResult<Self> {
        let path = path.as_ref().to_path_buf();
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let watched_paths = Arc::new(Mutex::new(HashSet::new()));
        let closed = Arc::new(Mutex::new(false));
        let closed_clone = closed.clone();
        let base_path = path.clone();

        // Create the notify watcher with event handler
        let watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if *closed_clone.lock() {
                    return;
                }

                match res {
                    Ok(event) => {
                        let watch_event = convert_notify_event(&event, &base_path);
                        if let Some(evt) = watch_event {
                            let _ = event_tx.send(Ok(evt));
                        }
                    }
                    Err(err) => {
                        let _ = event_tx.send(Err(WatchError::from(err)));
                    }
                }
            },
            Config::default(),
        )
        .map_err(|e| NodeError::new(ErrorCode::UNKNOWN, e.to_string()))?;

        let mut fs_watcher = FSWatcher {
            watcher,
            watched_paths,
            closed,
            event_rx,
        };

        // Add the initial path to watch
        fs_watcher.watch_path(&path, recursive)?;

        Ok(fs_watcher)
    }


    /// Add a path to watch.
    ///
    /// # Arguments
    /// * `path` - The path to watch.
    /// * `recursive` - Whether to watch subdirectories recursively.
    pub fn watch_path(&mut self, path: impl AsRef<Path>, recursive: bool) -> NodeResult<()> {
        let path = path.as_ref().to_path_buf();
        
        if *self.closed.lock() {
            return Err(NodeError::new(
                ErrorCode::UNKNOWN,
                "Watcher is closed".to_string(),
            ));
        }

        // Check if path exists
        if !path.exists() {
            return Err(NodeError::enoent(path.display().to_string()));
        }

        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        self.watcher
            .watch(&path, mode)
            .map_err(|e| NodeError::new(ErrorCode::UNKNOWN, e.to_string()))?;

        self.watched_paths.lock().insert(path);
        Ok(())
    }

    /// Remove a path from watching.
    ///
    /// # Arguments
    /// * `path` - The path to stop watching.
    pub fn unwatch(&mut self, path: impl AsRef<Path>) -> NodeResult<()> {
        let path = path.as_ref().to_path_buf();

        if *self.closed.lock() {
            return Err(NodeError::new(
                ErrorCode::UNKNOWN,
                "Watcher is closed".to_string(),
            ));
        }

        self.watcher
            .unwatch(&path)
            .map_err(|e| NodeError::new(ErrorCode::UNKNOWN, e.to_string()))?;

        self.watched_paths.lock().remove(&path);
        Ok(())
    }

    /// Close the watcher and release all resources.
    pub fn close(&mut self) {
        let mut closed = self.closed.lock();
        if *closed {
            return;
        }
        *closed = true;

        // Unwatch all paths
        let paths: Vec<PathBuf> = self.watched_paths.lock().drain().collect();
        for path in paths {
            let _ = self.watcher.unwatch(&path);
        }
    }

    /// Check if the watcher is closed.
    pub fn is_closed(&self) -> bool {
        *self.closed.lock()
    }

    /// Receive the next watch event asynchronously.
    ///
    /// Returns `None` if the watcher is closed.
    pub async fn recv(&mut self) -> Option<Result<WatchEvent, WatchError>> {
        if *self.closed.lock() {
            return None;
        }
        self.event_rx.recv().await
    }

    /// Get the set of currently watched paths.
    pub fn watched_paths(&self) -> Vec<PathBuf> {
        self.watched_paths.lock().iter().cloned().collect()
    }
}

impl Drop for FSWatcher {
    fn drop(&mut self) {
        self.close();
    }
}

/// Convert a notify event to a WatchEvent.
fn convert_notify_event(event: &Event, base_path: &Path) -> Option<WatchEvent> {
    use notify::EventKind;

    let event_type = match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) => WatchEventType::Change,
        EventKind::Remove(_) => WatchEventType::Rename,
        EventKind::Access(_) => return None, // Ignore access events
        EventKind::Other => return None,
        EventKind::Any => WatchEventType::Change,
    };

    // Get the filename relative to the base path
    let filename = event.paths.first().and_then(|p| {
        p.strip_prefix(base_path)
            .ok()
            .map(|rel| rel.to_string_lossy().to_string())
            .or_else(|| Some(p.file_name()?.to_string_lossy().to_string()))
    });

    Some(WatchEvent {
        event_type,
        filename,
    })
}


/// File watcher using polling (for fs.watchFile compatibility).
///
/// This provides stat-based change detection with configurable polling interval.
pub struct FSWatchFile {
    /// Polling interval.
    interval: Duration,
    /// Watched files with their last known stats.
    watched: Arc<Mutex<HashMap<PathBuf, Option<WatchFileStats>>>>,
    /// Whether the watcher is active.
    active: Arc<Mutex<bool>>,
    /// Event sender.
    event_tx: mpsc::UnboundedSender<WatchFileEvent>,
    /// Event receiver.
    event_rx: mpsc::UnboundedReceiver<WatchFileEvent>,
    /// Polling task handle.
    poll_handle: Option<tokio::task::JoinHandle<()>>,
}

/// Stats for watchFile comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct WatchFileStats {
    /// File size in bytes.
    pub size: u64,
    /// Last modification time.
    pub mtime: SystemTime,
    /// Last access time.
    pub atime: SystemTime,
    /// Creation time.
    pub ctime: SystemTime,
    /// Whether the file exists.
    pub exists: bool,
}

impl WatchFileStats {
    /// Create stats from a path, returning default stats if file doesn't exist.
    pub fn from_path(path: &Path) -> Self {
        match std::fs::metadata(path) {
            Ok(metadata) => Self {
                size: metadata.len(),
                mtime: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                atime: metadata.accessed().unwrap_or(SystemTime::UNIX_EPOCH),
                ctime: metadata.created().unwrap_or(SystemTime::UNIX_EPOCH),
                exists: true,
            },
            Err(_) => Self {
                size: 0,
                mtime: SystemTime::UNIX_EPOCH,
                atime: SystemTime::UNIX_EPOCH,
                ctime: SystemTime::UNIX_EPOCH,
                exists: false,
            },
        }
    }
}

/// Event from watchFile.
#[derive(Debug, Clone)]
pub struct WatchFileEvent {
    /// Path that changed.
    pub path: PathBuf,
    /// Current stats.
    pub current: WatchFileStats,
    /// Previous stats.
    pub previous: WatchFileStats,
}

impl FSWatchFile {
    /// Create a new polling file watcher.
    ///
    /// # Arguments
    /// * `interval` - Polling interval in milliseconds.
    pub fn new(interval_ms: u64) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        FSWatchFile {
            interval: Duration::from_millis(interval_ms),
            watched: Arc::new(Mutex::new(HashMap::new())),
            active: Arc::new(Mutex::new(false)),
            event_tx,
            event_rx,
            poll_handle: None,
        }
    }

    /// Start watching a file.
    ///
    /// # Arguments
    /// * `path` - The file path to watch.
    pub fn watch(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref().to_path_buf();
        let stats = WatchFileStats::from_path(&path);
        self.watched.lock().insert(path, Some(stats));

        // Start polling if not already active
        if !*self.active.lock() {
            self.start_polling();
        }
    }

    /// Stop watching a file.
    ///
    /// # Arguments
    /// * `path` - The file path to stop watching.
    pub fn unwatch(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref().to_path_buf();
        self.watched.lock().remove(&path);

        // Stop polling if no more files to watch
        if self.watched.lock().is_empty() {
            self.stop_polling();
        }
    }

    /// Start the polling task.
    fn start_polling(&mut self) {
        *self.active.lock() = true;

        let watched = self.watched.clone();
        let active = self.active.clone();
        let event_tx = self.event_tx.clone();
        let interval = self.interval;

        let handle = tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);

            loop {
                ticker.tick().await;

                if !*active.lock() {
                    break;
                }

                let paths: Vec<PathBuf> = watched.lock().keys().cloned().collect();

                for path in paths {
                    let current = WatchFileStats::from_path(&path);
                    let mut watched_guard = watched.lock();

                    if let Some(prev_opt) = watched_guard.get_mut(&path) {
                        if let Some(prev) = prev_opt.take() {
                            if current != prev {
                                let _ = event_tx.send(WatchFileEvent {
                                    path: path.clone(),
                                    current: current.clone(),
                                    previous: prev,
                                });
                            }
                        }
                        *prev_opt = Some(current);
                    }
                }
            }
        });

        self.poll_handle = Some(handle);
    }

    /// Stop the polling task.
    fn stop_polling(&mut self) {
        *self.active.lock() = false;
        if let Some(handle) = self.poll_handle.take() {
            handle.abort();
        }
    }

    /// Receive the next watch event asynchronously.
    pub async fn recv(&mut self) -> Option<WatchFileEvent> {
        self.event_rx.recv().await
    }

    /// Close the watcher and release all resources.
    pub fn close(&mut self) {
        self.stop_polling();
        self.watched.lock().clear();
    }
}

impl Drop for FSWatchFile {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_fs_watcher_creation() {
        let dir = tempdir().unwrap();
        let watcher = FSWatcher::new(dir.path(), false);
        assert!(watcher.is_ok());
    }

    #[tokio::test]
    async fn test_fs_watcher_nonexistent_path() {
        let result = FSWatcher::new("/nonexistent/path/that/does/not/exist", false);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fs_watcher_close() {
        let dir = tempdir().unwrap();
        let mut watcher = FSWatcher::new(dir.path(), false).unwrap();
        assert!(!watcher.is_closed());
        watcher.close();
        assert!(watcher.is_closed());
    }

    #[tokio::test]
    async fn test_watch_file_stats() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "hello").unwrap();

        let stats = WatchFileStats::from_path(&file_path);
        assert!(stats.exists);
        assert_eq!(stats.size, 5);
    }

    #[tokio::test]
    async fn test_watch_file_nonexistent() {
        let stats = WatchFileStats::from_path(Path::new("/nonexistent/file"));
        assert!(!stats.exists);
        assert_eq!(stats.size, 0);
    }

    #[tokio::test]
    async fn test_fs_watch_file_creation() {
        let watcher = FSWatchFile::new(100);
        assert!(watcher.watched.lock().is_empty());
    }

    #[tokio::test]
    async fn test_fs_watch_file_watch_unwatch() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "hello").unwrap();

        let mut watcher = FSWatchFile::new(100);
        watcher.watch(&file_path);
        assert_eq!(watcher.watched.lock().len(), 1);

        watcher.unwatch(&file_path);
        assert!(watcher.watched.lock().is_empty());
    }
}
