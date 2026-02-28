//! # File Watcher
//!
//! Watches source files for changes and triggers rebuilds.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::error::{DxError, DxResult};

// =============================================================================
// File Watcher
// =============================================================================

/// Watches files for changes.
pub struct FileWatcher {
    /// The underlying notify watcher
    watcher: RecommendedWatcher,
    /// Channel receiver for events
    rx: Receiver<notify::Result<Event>>,
    /// Watched paths
    watched_paths: Vec<PathBuf>,
}

impl FileWatcher {
    /// Create a new file watcher.
    pub fn new(root: &Path) -> DxResult<Self> {
        let (tx, rx) = channel();

        let watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default().with_poll_interval(Duration::from_millis(100)),
        )
        .map_err(|e| DxError::IoError {
            path: Some(root.to_path_buf()),
            message: format!("Failed to create watcher: {}", e),
        })?;

        let mut fw = Self {
            watcher,
            rx,
            watched_paths: Vec::new(),
        };

        // Watch standard directories
        fw.watch_directory(&root.join("pages"))?;
        fw.watch_directory(&root.join("components"))?;
        fw.watch_directory(&root.join("api"))?;
        fw.watch_directory(&root.join("styles"))?;
        fw.watch_directory(&root.join("public"))?;

        Ok(fw)
    }

    /// Watch a directory recursively.
    pub fn watch_directory(&mut self, path: &Path) -> DxResult<()> {
        if path.exists() {
            self.watcher
                .watch(path, RecursiveMode::Recursive)
                .map_err(|e| DxError::IoError {
                    path: Some(path.to_path_buf()),
                    message: format!("Failed to watch: {}", e),
                })?;
            self.watched_paths.push(path.to_path_buf());
        }
        Ok(())
    }

    /// Unwatch a directory.
    pub fn unwatch_directory(&mut self, path: &Path) -> DxResult<()> {
        self.watcher.unwatch(path).map_err(|e| DxError::IoError {
            path: Some(path.to_path_buf()),
            message: format!("Failed to unwatch: {}", e),
        })?;
        self.watched_paths.retain(|p| p != path);
        Ok(())
    }

    /// Poll for file change events.
    pub fn poll(&self) -> Option<FileChangeEvent> {
        match self.rx.try_recv() {
            Ok(Ok(event)) => {
                let paths: Vec<PathBuf> = event.paths;
                if paths.is_empty() {
                    return None;
                }

                let kind = match event.kind {
                    notify::EventKind::Create(_) => ChangeKind::Created,
                    notify::EventKind::Modify(_) => ChangeKind::Modified,
                    notify::EventKind::Remove(_) => ChangeKind::Deleted,
                    _ => return None,
                };

                Some(FileChangeEvent { paths, kind })
            }
            _ => None,
        }
    }

    /// Get the list of watched paths.
    pub fn watched_paths(&self) -> &[PathBuf] {
        &self.watched_paths
    }

    /// Stop the watcher.
    pub fn stop(self) -> DxResult<()> {
        // Watcher is dropped automatically
        Ok(())
    }
}

// =============================================================================
// File Change Event
// =============================================================================

/// A file change event.
#[derive(Debug, Clone)]
pub struct FileChangeEvent {
    /// Paths that changed
    pub paths: Vec<PathBuf>,
    /// Kind of change
    pub kind: ChangeKind,
}

/// Kind of file change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// File was created
    Created,
    /// File was modified
    Modified,
    /// File was deleted
    Deleted,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_change_kind_eq() {
        assert_eq!(ChangeKind::Created, ChangeKind::Created);
        assert_ne!(ChangeKind::Created, ChangeKind::Modified);
    }
}
