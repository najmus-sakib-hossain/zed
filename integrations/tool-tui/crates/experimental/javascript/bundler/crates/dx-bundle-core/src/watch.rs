//! Watch mode implementation for DX Bundler
//!
//! Provides file system watching with debouncing and incremental rebuild support.

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

/// Watch mode configuration
#[derive(Clone, Debug)]
pub struct WatchConfig {
    /// Debounce delay in milliseconds
    pub debounce_ms: u64,
    /// File extensions to watch
    pub extensions: Vec<String>,
    /// Directories to ignore
    pub ignore_dirs: Vec<String>,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 100,
            extensions: vec![
                "js".to_string(),
                "jsx".to_string(),
                "ts".to_string(),
                "tsx".to_string(),
                "mjs".to_string(),
                "cjs".to_string(),
                "json".to_string(),
            ],
            ignore_dirs: vec![
                "node_modules".to_string(),
                ".git".to_string(),
                "dist".to_string(),
                ".dx-cache".to_string(),
            ],
        }
    }
}

/// File watcher for watch mode
pub struct FileWatcher {
    watcher: RecommendedWatcher,
    rx: Receiver<notify::Result<Event>>,
    watched_paths: HashSet<PathBuf>,
    config: WatchConfig,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new(config: WatchConfig) -> Result<Self, WatchError> {
        let (tx, rx) = mpsc::channel();

        let watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default(),
        )
        .map_err(|e| WatchError::InitError(e.to_string()))?;

        Ok(Self {
            watcher,
            rx,
            watched_paths: HashSet::new(),
            config,
        })
    }

    /// Add a path to watch recursively
    pub fn watch(&mut self, path: &Path) -> Result<(), WatchError> {
        if !self.watched_paths.contains(path) {
            self.watcher
                .watch(path, RecursiveMode::Recursive)
                .map_err(|e| WatchError::WatchError(e.to_string()))?;
            self.watched_paths.insert(path.to_path_buf());
        }
        Ok(())
    }

    /// Stop watching a path
    pub fn unwatch(&mut self, path: &Path) -> Result<(), WatchError> {
        if self.watched_paths.contains(path) {
            self.watcher.unwatch(path).map_err(|e| WatchError::WatchError(e.to_string()))?;
            self.watched_paths.remove(path);
        }
        Ok(())
    }

    /// Wait for file changes with debouncing
    /// Returns the set of changed file paths
    pub fn wait_for_changes(&self) -> Vec<PathBuf> {
        let mut changed = HashSet::new();
        let debounce_duration = Duration::from_millis(self.config.debounce_ms);
        let mut last_event_time = None;

        loop {
            let timeout = match last_event_time {
                Some(t) => {
                    let elapsed = Instant::now().duration_since(t);
                    if elapsed >= debounce_duration {
                        break;
                    }
                    debounce_duration - elapsed
                }
                None => Duration::from_secs(3600), // Wait indefinitely for first event
            };

            match self.rx.recv_timeout(timeout) {
                Ok(Ok(event)) => {
                    if self.is_relevant_event(&event) {
                        for path in event.paths {
                            if self.should_include_path(&path) {
                                changed.insert(path);
                                last_event_time = Some(Instant::now());
                            }
                        }
                    }
                }
                Ok(Err(e)) => {
                    eprintln!("Watch error: {}", e);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if last_event_time.is_some() {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break;
                }
            }
        }

        changed.into_iter().collect()
    }

    /// Check if an event is relevant (create, modify, remove)
    fn is_relevant_event(&self, event: &Event) -> bool {
        matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_))
    }

    /// Check if a path should be included based on extension and ignore rules
    fn should_include_path(&self, path: &Path) -> bool {
        // Check if in ignored directory
        for component in path.components() {
            if let std::path::Component::Normal(name) = component {
                if let Some(name_str) = name.to_str() {
                    if self.config.ignore_dirs.contains(&name_str.to_string()) {
                        return false;
                    }
                }
            }
        }

        // Check extension
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            self.config.extensions.contains(&ext.to_string())
        } else {
            false
        }
    }
}

/// Watch mode errors
#[derive(Debug, thiserror::Error)]
pub enum WatchError {
    #[error("Failed to initialize watcher: {0}")]
    InitError(String),
    #[error("Failed to watch path: {0}")]
    WatchError(String),
    #[error("Rebuild failed: {0}")]
    RebuildError(String),
}

/// HMR update notification
#[derive(Clone, Debug)]
pub struct HmrUpdate {
    /// Module ID that was updated
    pub module_id: String,
    /// Content hash of the new module
    pub hash: String,
    /// Whether a full reload is required
    pub full_reload: bool,
    /// Changed file paths
    pub changed_files: Vec<PathBuf>,
}

/// HMR server for notifying connected clients of updates
pub struct HmrServer {
    clients: Vec<tokio::sync::mpsc::Sender<HmrUpdate>>,
}

impl HmrServer {
    /// Create a new HMR server
    pub fn new() -> Self {
        Self {
            clients: Vec::new(),
        }
    }

    /// Register a new client and return a receiver for updates
    pub fn register_client(&mut self) -> tokio::sync::mpsc::Receiver<HmrUpdate> {
        let (tx, rx) = tokio::sync::mpsc::channel(16);
        self.clients.push(tx);
        rx
    }

    /// Notify all connected clients of an update
    pub async fn notify(&mut self, update: HmrUpdate) {
        // Remove disconnected clients
        self.clients.retain(|client| !client.is_closed());

        // Send update to all connected clients
        for client in &self.clients {
            let _ = client.send(update.clone()).await;
        }
    }

    /// Get the number of connected clients
    pub fn client_count(&self) -> usize {
        self.clients.iter().filter(|c| !c.is_closed()).count()
    }
}

impl Default for HmrServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_watch_config_default() {
        let config = WatchConfig::default();
        assert_eq!(config.debounce_ms, 100);
        assert!(config.extensions.contains(&"js".to_string()));
        assert!(config.extensions.contains(&"ts".to_string()));
        assert!(config.ignore_dirs.contains(&"node_modules".to_string()));
    }

    #[test]
    fn test_hmr_server() {
        let server = HmrServer::new();
        assert_eq!(server.client_count(), 0);
    }

    #[test]
    fn test_hmr_update() {
        let update = HmrUpdate {
            module_id: "test".to_string(),
            hash: "abc123".to_string(),
            full_reload: false,
            changed_files: vec![PathBuf::from("test.js")],
        };
        assert_eq!(update.module_id, "test");
        assert!(!update.full_reload);
    }
}
