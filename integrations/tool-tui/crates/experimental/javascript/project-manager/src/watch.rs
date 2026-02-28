//! Watch Manager
//!
//! Monitors file system events with intelligent debouncing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// File change type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    /// File was created
    Created,
    /// File was modified
    Modified,
    /// File was deleted
    Deleted,
    /// File was renamed
    Renamed,
}

/// File change event
#[derive(Debug, Clone)]
pub struct FileChange {
    /// Path to the changed file
    pub path: PathBuf,
    /// Type of change
    pub change_type: ChangeType,
    /// Timestamp in nanoseconds
    pub timestamp_ns: u64,
}

/// Debounce configuration
#[derive(Debug, Clone)]
pub struct DebounceConfig {
    /// Minimum wait time before triggering (ms)
    pub min_wait_ms: u32,
    /// Maximum wait time (ms)
    pub max_wait_ms: u32,
    /// Coalesce rapid changes to same file
    pub coalesce: bool,
}

impl Default for DebounceConfig {
    fn default() -> Self {
        Self {
            min_wait_ms: 50,
            max_wait_ms: 500,
            coalesce: true,
        }
    }
}

/// Type alias for predictive execution callback
type PredictiveCallback = Box<dyn Fn(&Path) -> Vec<u32> + Send + Sync>;

/// Watch Manager for file system monitoring
pub struct WatchManager {
    /// Debounce configuration
    debounce_config: DebounceConfig,
    /// Pending changes (coalesced)
    pending_changes: Arc<Mutex<HashMap<PathBuf, FileChange>>>,
    /// Last change time per file
    last_change_time: Arc<Mutex<HashMap<PathBuf, Instant>>>,
    /// Predictive execution callback
    on_predicted: Option<PredictiveCallback>,
    /// Whether watching is active
    watching: bool,
    /// Watched paths
    watched_paths: Vec<PathBuf>,
}

impl WatchManager {
    /// Create a new watch manager
    pub fn new() -> Self {
        Self {
            debounce_config: DebounceConfig::default(),
            pending_changes: Arc::new(Mutex::new(HashMap::new())),
            last_change_time: Arc::new(Mutex::new(HashMap::new())),
            on_predicted: None,
            watching: false,
            watched_paths: Vec::new(),
        }
    }

    /// Start watching workspace
    pub fn start(&mut self) -> Result<(), crate::error::WatchError> {
        self.watching = true;
        // In a real implementation, this would start the notify watcher
        Ok(())
    }

    /// Stop watching
    pub fn stop(&mut self) {
        self.watching = false;
    }

    /// Check if watching is active
    pub fn is_watching(&self) -> bool {
        self.watching
    }

    /// Add path to watch
    pub fn watch_path(&mut self, path: PathBuf) {
        self.watched_paths.push(path);
    }

    /// Set predictive execution callback
    pub fn on_predicted_change<F>(&mut self, callback: F)
    where
        F: Fn(&Path) -> Vec<u32> + Send + Sync + 'static,
    {
        self.on_predicted = Some(Box::new(callback));
    }

    /// Configure debounce settings
    pub fn set_debounce(&mut self, config: DebounceConfig) {
        self.debounce_config = config;
    }

    /// Get pending changes (coalesced)
    pub fn pending_changes(&self) -> Vec<FileChange> {
        let pending = self.pending_changes.lock().unwrap();
        pending.values().cloned().collect()
    }

    /// Clear pending changes
    pub fn clear_pending(&self) {
        let mut pending = self.pending_changes.lock().unwrap();
        pending.clear();
    }

    /// Record a file change (for testing or manual triggering)
    pub fn record_change(&self, path: PathBuf, change_type: ChangeType) {
        let now = Instant::now();
        let timestamp_ns = now.elapsed().as_nanos() as u64;

        if self.debounce_config.coalesce {
            // Check if we should coalesce with existing change
            let mut last_times = self.last_change_time.lock().unwrap();

            if let Some(last_time) = last_times.get(&path) {
                let elapsed = now.duration_since(*last_time);
                if elapsed < Duration::from_millis(self.debounce_config.min_wait_ms as u64) {
                    // Update timestamp but don't add new change
                    last_times.insert(path.clone(), now);

                    // Update existing pending change
                    let mut pending = self.pending_changes.lock().unwrap();
                    if let Some(change) = pending.get_mut(&path) {
                        change.timestamp_ns = timestamp_ns;
                        change.change_type = change_type;
                    }
                    return;
                }
            }

            last_times.insert(path.clone(), now);
        }

        // Add new change
        let mut pending = self.pending_changes.lock().unwrap();
        pending.insert(
            path.clone(),
            FileChange {
                path,
                change_type,
                timestamp_ns,
            },
        );
    }

    /// Get tasks to run for a file change (using predictive callback)
    pub fn predict_tasks(&self, path: &Path) -> Vec<u32> {
        match &self.on_predicted {
            Some(callback) => callback(path),
            None => Vec::new(),
        }
    }

    /// Check if changes are ready to process (debounce complete)
    pub fn changes_ready(&self) -> bool {
        let pending = self.pending_changes.lock().unwrap();
        if pending.is_empty() {
            return false;
        }

        let last_times = self.last_change_time.lock().unwrap();
        let now = Instant::now();
        let min_wait = Duration::from_millis(self.debounce_config.min_wait_ms as u64);

        // Check if all pending changes have waited long enough
        for path in pending.keys() {
            if let Some(last_time) = last_times.get(path) {
                if now.duration_since(*last_time) < min_wait {
                    return false;
                }
            }
        }

        true
    }

    /// Flush pending changes and return them
    pub fn flush_changes(&self) -> Vec<FileChange> {
        let mut pending = self.pending_changes.lock().unwrap();
        let changes: Vec<_> = pending.drain().map(|(_, v)| v).collect();
        changes
    }
}

impl Default for WatchManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_watch_manager_creation() {
        let manager = WatchManager::new();
        assert!(!manager.is_watching());
        assert!(manager.pending_changes().is_empty());
    }

    #[test]
    fn test_start_stop() {
        let mut manager = WatchManager::new();

        manager.start().unwrap();
        assert!(manager.is_watching());

        manager.stop();
        assert!(!manager.is_watching());
    }

    #[test]
    fn test_record_change() {
        let manager = WatchManager::new();

        manager.record_change(PathBuf::from("src/index.ts"), ChangeType::Modified);

        let pending = manager.pending_changes();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].path, PathBuf::from("src/index.ts"));
        assert_eq!(pending[0].change_type, ChangeType::Modified);
    }

    #[test]
    fn test_change_coalescing() {
        let mut manager = WatchManager::new();
        manager.set_debounce(DebounceConfig {
            min_wait_ms: 100,
            max_wait_ms: 500,
            coalesce: true,
        });

        let path = PathBuf::from("src/index.ts");

        // Record multiple rapid changes
        manager.record_change(path.clone(), ChangeType::Modified);
        manager.record_change(path.clone(), ChangeType::Modified);
        manager.record_change(path.clone(), ChangeType::Modified);

        // Should be coalesced into one change
        let pending = manager.pending_changes();
        assert_eq!(pending.len(), 1);
    }

    #[test]
    fn test_multiple_files() {
        let manager = WatchManager::new();

        manager.record_change(PathBuf::from("a.ts"), ChangeType::Modified);
        manager.record_change(PathBuf::from("b.ts"), ChangeType::Created);
        manager.record_change(PathBuf::from("c.ts"), ChangeType::Deleted);

        let pending = manager.pending_changes();
        assert_eq!(pending.len(), 3);
    }

    #[test]
    fn test_flush_changes() {
        let manager = WatchManager::new();

        manager.record_change(PathBuf::from("a.ts"), ChangeType::Modified);
        manager.record_change(PathBuf::from("b.ts"), ChangeType::Modified);

        let flushed = manager.flush_changes();
        assert_eq!(flushed.len(), 2);

        // Pending should be empty after flush
        assert!(manager.pending_changes().is_empty());
    }

    #[test]
    fn test_predictive_callback() {
        let mut manager = WatchManager::new();

        manager.on_predicted_change(|path| {
            if path.to_string_lossy().contains("src") {
                vec![0, 1, 2] // Build tasks
            } else {
                vec![]
            }
        });

        let tasks = manager.predict_tasks(Path::new("src/index.ts"));
        assert_eq!(tasks, vec![0, 1, 2]);

        let tasks = manager.predict_tasks(Path::new("docs/readme.md"));
        assert!(tasks.is_empty());
    }

    #[test]
    fn test_debounce_config() {
        let mut manager = WatchManager::new();

        manager.set_debounce(DebounceConfig {
            min_wait_ms: 200,
            max_wait_ms: 1000,
            coalesce: false,
        });

        // With coalesce disabled, each change should be recorded
        let path = PathBuf::from("test.ts");
        manager.record_change(path.clone(), ChangeType::Modified);

        // Wait a bit
        thread::sleep(Duration::from_millis(10));

        manager.record_change(path.clone(), ChangeType::Modified);

        // Both changes recorded (coalesce disabled, but same key overwrites)
        let pending = manager.pending_changes();
        assert_eq!(pending.len(), 1); // HashMap overwrites same key
    }

    #[test]
    fn test_clear_pending() {
        let manager = WatchManager::new();

        manager.record_change(PathBuf::from("a.ts"), ChangeType::Modified);
        assert!(!manager.pending_changes().is_empty());

        manager.clear_pending();
        assert!(manager.pending_changes().is_empty());
    }
}
