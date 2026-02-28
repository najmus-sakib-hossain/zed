//! Configuration File Watcher with Hot-Reload
//!
//! Watches configuration files for changes and triggers reloads
//! with debounce to prevent rapid successive reloads.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{RwLock, broadcast, mpsc};

/// Configuration watcher that monitors files for changes
pub struct ConfigWatcher {
    /// Paths being watched
    watched_paths: Vec<PathBuf>,
    /// Debounce duration
    debounce: Duration,
    /// Broadcast sender for reload events
    event_tx: broadcast::Sender<ConfigReloadEvent>,
    /// Shutdown signal
    shutdown_tx: Option<mpsc::Sender<()>>,
    /// Whether the watcher is running
    running: Arc<RwLock<bool>>,
}

/// Configuration reload event
#[derive(Debug, Clone)]
pub struct ConfigReloadEvent {
    /// Path of the changed file
    pub path: PathBuf,
    /// Kind of change
    pub kind: ConfigChangeKind,
    /// Timestamp of the change
    pub timestamp: Instant,
}

/// Kind of configuration change
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigChangeKind {
    /// File was modified
    Modified,
    /// File was created
    Created,
    /// File was deleted
    Removed,
}

/// Watcher errors
#[derive(Debug, thiserror::Error)]
pub enum WatcherError {
    #[error("Failed to create watcher: {0}")]
    CreateError(String),

    #[error("Failed to watch path: {0}")]
    WatchError(String),

    #[error("Watcher is not running")]
    NotRunning,
}

impl ConfigWatcher {
    /// Create a new configuration watcher
    ///
    /// # Arguments
    /// - `debounce_ms` - Debounce duration in milliseconds (default: 500ms)
    pub fn new(debounce_ms: u64) -> Self {
        let (event_tx, _) = broadcast::channel(64);
        Self {
            watched_paths: Vec::new(),
            debounce: Duration::from_millis(debounce_ms),
            event_tx,
            shutdown_tx: None,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Add a path to watch
    pub fn watch_path(&mut self, path: PathBuf) {
        if !self.watched_paths.contains(&path) {
            self.watched_paths.push(path);
        }
    }

    /// Subscribe to reload events
    pub fn subscribe(&self) -> broadcast::Receiver<ConfigReloadEvent> {
        self.event_tx.subscribe()
    }

    /// Start watching for changes
    pub async fn start(&mut self) -> Result<(), WatcherError> {
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        let (notify_tx, mut notify_rx) = mpsc::channel::<Event>(100);

        // Create the file watcher
        let mut watcher: RecommendedWatcher =
            notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = notify_tx.blocking_send(event);
                }
            })
            .map_err(|e| WatcherError::CreateError(e.to_string()))?;

        // Watch all configured paths
        for path in &self.watched_paths {
            let watch_path = if path.is_file() {
                path.parent().unwrap_or(Path::new("."))
            } else {
                path.as_path()
            };

            watcher.watch(watch_path, RecursiveMode::NonRecursive).map_err(|e| {
                WatcherError::WatchError(format!("{}: {}", watch_path.display(), e))
            })?;
        }

        self.shutdown_tx = Some(shutdown_tx);
        *self.running.write().await = true;

        let event_tx = self.event_tx.clone();
        let debounce = self.debounce;
        let watched_paths = self.watched_paths.clone();
        let running = Arc::clone(&self.running);

        // Spawn the watcher task
        tokio::spawn(async move {
            let mut last_event_time: Option<Instant> = None;
            let mut pending_event: Option<ConfigReloadEvent> = None;

            loop {
                tokio::select! {
                    // Handle file system events
                    Some(event) = notify_rx.recv() => {
                        if let Some(reload_event) = process_notify_event(&event, &watched_paths) {
                            let now = Instant::now();

                            // Debounce: only emit if enough time has passed
                            if let Some(last) = last_event_time {
                                if now.duration_since(last) < debounce {
                                    // Store as pending, will be sent after debounce
                                    pending_event = Some(reload_event);
                                    continue;
                                }
                            }

                            last_event_time = Some(now);
                            let _ = event_tx.send(reload_event);
                            pending_event = None;
                        }
                    }

                    // Handle shutdown signal
                    _ = shutdown_rx.recv() => {
                        break;
                    }

                    // Debounce timer - send pending event
                    _ = tokio::time::sleep(debounce) => {
                        if let Some(event) = pending_event.take() {
                            last_event_time = Some(Instant::now());
                            let _ = event_tx.send(event);
                        }
                    }
                }
            }

            // Keep watcher alive until shutdown
            drop(watcher);
            *running.write().await = false;
        });

        Ok(())
    }

    /// Stop watching
    pub async fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
        *self.running.write().await = false;
    }

    /// Check if the watcher is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    /// Get the list of watched paths
    pub fn watched_paths(&self) -> &[PathBuf] {
        &self.watched_paths
    }
}

/// Process a notify event into a ConfigReloadEvent
fn process_notify_event(event: &Event, watched_paths: &[PathBuf]) -> Option<ConfigReloadEvent> {
    let kind = match event.kind {
        EventKind::Modify(_) => ConfigChangeKind::Modified,
        EventKind::Create(_) => ConfigChangeKind::Created,
        EventKind::Remove(_) => ConfigChangeKind::Removed,
        _ => return None,
    };

    // Check if any of the affected paths match our watched paths
    for event_path in &event.paths {
        for watched in watched_paths {
            if event_path == watched || event_path.starts_with(watched) {
                return Some(ConfigReloadEvent {
                    path: event_path.clone(),
                    kind,
                    timestamp: Instant::now(),
                });
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_watcher() {
        let watcher = ConfigWatcher::new(500);
        assert!(watcher.watched_paths.is_empty());
        assert_eq!(watcher.debounce, Duration::from_millis(500));
    }

    #[test]
    fn test_add_watch_path() {
        let mut watcher = ConfigWatcher::new(500);
        watcher.watch_path(PathBuf::from("/tmp/config.yaml"));
        watcher.watch_path(PathBuf::from("/tmp/secrets.yaml"));
        assert_eq!(watcher.watched_paths.len(), 2);
    }

    #[test]
    fn test_no_duplicate_paths() {
        let mut watcher = ConfigWatcher::new(500);
        watcher.watch_path(PathBuf::from("/tmp/config.yaml"));
        watcher.watch_path(PathBuf::from("/tmp/config.yaml"));
        assert_eq!(watcher.watched_paths.len(), 1);
    }

    #[test]
    fn test_subscribe() {
        let watcher = ConfigWatcher::new(500);
        let _rx = watcher.subscribe();
        // Should not panic
    }

    #[tokio::test]
    async fn test_is_running_starts_false() {
        let watcher = ConfigWatcher::new(500);
        assert!(!watcher.is_running().await);
    }

    #[test]
    fn test_process_notify_event_modify() {
        let watched = vec![PathBuf::from("/tmp/config.yaml")];
        let event = Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content,
            )),
            paths: vec![PathBuf::from("/tmp/config.yaml")],
            attrs: Default::default(),
        };
        let result = process_notify_event(&event, &watched);
        assert!(result.is_some());
        assert_eq!(result.unwrap().kind, ConfigChangeKind::Modified);
    }

    #[test]
    fn test_process_notify_event_unrelated_path() {
        let watched = vec![PathBuf::from("/tmp/config.yaml")];
        let event = Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content,
            )),
            paths: vec![PathBuf::from("/tmp/other.txt")],
            attrs: Default::default(),
        };
        let result = process_notify_event(&event, &watched);
        assert!(result.is_none());
    }
}
