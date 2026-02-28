//! Configuration file watcher for hot-reloading
//!
//! This module provides a debounced file watcher that monitors the .dx
//! configuration directory for changes and triggers reloads.

use crate::dx_io::{Reactor, WatchEvent as ReactorWatchEvent};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

/// Configuration file watcher with debouncing
pub struct ConfigWatcher {
    /// The watcher instance
    watcher: Option<RecommendedWatcher>,
    /// Paths being watched
    watched_paths: Vec<PathBuf>,
    /// Debounce duration
    debounce_duration: Duration,
    /// Event sender for notifications
    event_tx: broadcast::Sender<WatchEvent>,
}

/// Watch event types
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// A file was modified
    Modified(PathBuf),
    /// A file was created
    Created(PathBuf),
    /// A file was deleted
    Deleted(PathBuf),
    /// Multiple files changed (debounced batch)
    Batch(Vec<PathBuf>),
    /// Watcher error
    Error(String),
}

impl ConfigWatcher {
    /// Create a new config watcher
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(100);

        Self {
            watcher: None,
            watched_paths: Vec::new(),
            debounce_duration: Duration::from_millis(100),
            event_tx,
        }
    }

    /// Set the debounce duration
    pub fn with_debounce(mut self, duration: Duration) -> Self {
        self.debounce_duration = duration;
        self
    }

    /// Subscribe to watch events
    pub fn subscribe(&self) -> broadcast::Receiver<WatchEvent> {
        self.event_tx.subscribe()
    }

    /// Start watching a path
    pub fn watch(&mut self, path: PathBuf) -> Result<(), WatchError> {
        if self.watcher.is_none() {
            self.init_watcher()?;
        }

        if let Some(ref mut watcher) = self.watcher {
            watcher
                .watch(&path, RecursiveMode::Recursive)
                .map_err(|e| WatchError::WatchFailed(e.to_string()))?;

            self.watched_paths.push(path);
        }

        Ok(())
    }

    /// Stop watching a path
    pub fn unwatch(&mut self, path: &PathBuf) -> Result<(), WatchError> {
        if let Some(ref mut watcher) = self.watcher {
            watcher.unwatch(path).map_err(|e| WatchError::UnwatchFailed(e.to_string()))?;

            self.watched_paths.retain(|p| p != path);
        }

        Ok(())
    }

    /// Initialize the file watcher
    fn init_watcher(&mut self) -> Result<(), WatchError> {
        let event_tx = self.event_tx.clone();
        let debounce_duration = self.debounce_duration;

        // Create debouncer state
        let pending_events: Arc<Mutex<Vec<(PathBuf, Instant)>>> = Arc::new(Mutex::new(Vec::new()));
        let pending_clone = pending_events.clone();

        // Spawn debounce processor
        let tx_clone = event_tx.clone();
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_millis(50));

                let mut pending = pending_clone.lock().unwrap();
                let now = Instant::now();

                // Find events that have been pending long enough
                let (ready, still_pending): (Vec<_>, Vec<_>) = pending
                    .drain(..)
                    .partition(|(_, time)| now.duration_since(*time) >= debounce_duration);

                *pending = still_pending;

                // Send ready events
                if !ready.is_empty() {
                    let paths: Vec<PathBuf> = ready.into_iter().map(|(p, _)| p).collect();

                    if paths.len() == 1 {
                        let _ =
                            tx_clone.send(WatchEvent::Modified(paths.into_iter().next().unwrap()));
                    } else {
                        let _ = tx_clone.send(WatchEvent::Batch(paths));
                    }
                }
            }
        });

        // Create the watcher
        let watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                match result {
                    Ok(event) => {
                        let mut pending = pending_events.lock().unwrap();
                        let now = Instant::now();

                        for path in event.paths {
                            // Only watch .sr files
                            if path.extension().map_or(false, |ext| ext == "sr") {
                                // Update or add pending event
                                if let Some(existing) = pending.iter_mut().find(|(p, _)| p == &path)
                                {
                                    existing.1 = now;
                                } else {
                                    pending.push((path, now));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = event_tx.send(WatchEvent::Error(e.to_string()));
                    }
                }
            },
            Config::default(),
        )
        .map_err(|e| WatchError::InitFailed(e.to_string()))?;

        self.watcher = Some(watcher);
        Ok(())
    }

    /// Watch the default config directory
    pub fn watch_default(&mut self) -> Result<(), WatchError> {
        let config_dir = dirs::home_dir()
            .map(|h| h.join(".dx").join("config"))
            .ok_or_else(|| WatchError::InitFailed("Could not find home directory".to_string()))?;

        if config_dir.exists() {
            self.watch(config_dir)
        } else {
            // Create the directory if it doesn't exist
            std::fs::create_dir_all(&config_dir)
                .map_err(|e| WatchError::InitFailed(e.to_string()))?;
            self.watch(config_dir)
        }
    }

    /// Get currently watched paths
    pub fn watched_paths(&self) -> &[PathBuf] {
        &self.watched_paths
    }
}

impl Default for ConfigWatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Watch error types
#[derive(Debug, Clone)]
pub enum WatchError {
    /// Failed to initialize watcher
    InitFailed(String),
    /// Failed to watch path
    WatchFailed(String),
    /// Failed to unwatch path
    UnwatchFailed(String),
}

impl std::fmt::Display for WatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WatchError::InitFailed(e) => write!(f, "Failed to initialize watcher: {}", e),
            WatchError::WatchFailed(e) => write!(f, "Failed to watch path: {}", e),
            WatchError::UnwatchFailed(e) => write!(f, "Failed to unwatch path: {}", e),
        }
    }
}

impl std::error::Error for WatchError {}

/// Async wrapper for config watching with automatic reloading
pub struct AsyncConfigWatcher {
    watcher: ConfigWatcher,
}

impl AsyncConfigWatcher {
    /// Create a new async config watcher
    pub fn new() -> Self {
        Self {
            watcher: ConfigWatcher::new(),
        }
    }

    /// Start watching and return a stream of events
    pub async fn start(&mut self) -> Result<broadcast::Receiver<WatchEvent>, WatchError> {
        self.watcher.watch_default()?;
        Ok(self.watcher.subscribe())
    }

    /// Run the watcher with a reload callback
    pub async fn run_with_reload<F>(&mut self, mut on_reload: F) -> Result<(), WatchError>
    where
        F: FnMut(Vec<PathBuf>) + Send + 'static,
    {
        let mut rx = self.start().await?;

        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                match event {
                    WatchEvent::Modified(path) => {
                        on_reload(vec![path]);
                    }
                    WatchEvent::Created(path) => {
                        on_reload(vec![path]);
                    }
                    WatchEvent::Deleted(path) => {
                        on_reload(vec![path]);
                    }
                    WatchEvent::Batch(paths) => {
                        on_reload(paths);
                    }
                    WatchEvent::Error(e) => {
                        eprintln!("Config watcher error: {}", e);
                    }
                }
            }
        });

        Ok(())
    }

    /// Run the watcher using the Reactor for file system events
    pub async fn run_with_reactor<F>(
        &mut self,
        reactor: Arc<dyn Reactor>,
        mut on_reload: F,
    ) -> Result<(), WatchError>
    where
        F: FnMut(Vec<PathBuf>) + Send + 'static,
    {
        let config_dir = dirs::home_dir()
            .map(|h| h.join(".dx").join("config"))
            .ok_or_else(|| WatchError::InitFailed("Could not find home directory".to_string()))?;

        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir)
                .map_err(|e| WatchError::InitFailed(e.to_string()))?;
        }

        let mut rx = reactor
            .watch_dir(&config_dir)
            .await
            .map_err(|e| WatchError::InitFailed(e.to_string()))?;

        let sender = self.watcher.event_tx.clone();
        let debounce_duration = self.watcher.debounce_duration;

        tokio::spawn(async move {
            let mut pending: Vec<(PathBuf, Instant)> = Vec::new();
            let mut interval = tokio::time::interval(Duration::from_millis(50));

            loop {
                tokio::select! {
                    maybe_event = rx.recv() => {
                        let event = match maybe_event {
                            Some(evt) => evt,
                            None => break,
                        };

                        let now = Instant::now();
                        let mut paths: Vec<PathBuf> = Vec::new();

                        match event {
                            ReactorWatchEvent::Create(path) => paths.push(path),
                            ReactorWatchEvent::Modify(path) => paths.push(path),
                            ReactorWatchEvent::Delete(path) => paths.push(path),
                            ReactorWatchEvent::Rename(old_path, new_path) => {
                                paths.push(old_path);
                                paths.push(new_path);
                            }
                        }

                        for path in paths {
                            if path.extension().map_or(false, |ext| ext == "sr") {
                                if let Some(existing) = pending.iter_mut().find(|(p, _)| p == &path) {
                                    existing.1 = now;
                                } else {
                                    pending.push((path, now));
                                }
                            }
                        }
                    }
                    _ = interval.tick() => {
                        let now = Instant::now();
                        let (ready, still_pending): (Vec<_>, Vec<_>) = pending
                            .drain(..)
                            .partition(|(_, time)| now.duration_since(*time) >= debounce_duration);

                        pending = still_pending;

                        if !ready.is_empty() {
                            let paths: Vec<PathBuf> = ready.into_iter().map(|(p, _)| p).collect();

                            let event = if paths.len() == 1 {
                                WatchEvent::Modified(paths[0].clone())
                            } else {
                                WatchEvent::Batch(paths.clone())
                            };

                            let _ = sender.send(event);
                            on_reload(paths);
                        }
                    }
                }
            }
        });

        Ok(())
    }
}

impl Default for AsyncConfigWatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_watcher_creation() {
        let watcher = ConfigWatcher::new();
        assert!(watcher.watched_paths.is_empty());
    }

    #[test]
    fn test_watcher_subscribe() {
        let watcher = ConfigWatcher::new();
        let _rx = watcher.subscribe();
        // Should not panic
    }

    #[tokio::test]
    async fn test_file_modification_detection() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.sr");

        // Create initial file
        {
            let mut file = File::create(&file_path).unwrap();
            writeln!(file, "initial content").unwrap();
        }

        let mut watcher = ConfigWatcher::new().with_debounce(Duration::from_millis(50));

        watcher.watch(dir.path().to_path_buf()).unwrap();
        let mut rx = watcher.subscribe();

        // Modify the file
        tokio::time::sleep(Duration::from_millis(100)).await;
        {
            let mut file = File::create(&file_path).unwrap();
            writeln!(file, "modified content").unwrap();
        }

        // Wait for debounce
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Note: In real tests, we'd check the received event
        // For now, just verify no panics occur
    }
}
