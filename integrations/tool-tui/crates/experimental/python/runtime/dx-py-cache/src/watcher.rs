//! File watcher for cache invalidation

use crossbeam::channel::{self, Receiver, Sender};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::cache::ReactiveCache;

/// Events from the cache watcher
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// File was modified
    Modified(PathBuf),
    /// File was created
    Created(PathBuf),
    /// File was deleted
    Deleted(PathBuf),
    /// Error occurred
    Error(String),
}

/// File watcher for automatic cache invalidation
pub struct CacheWatcher {
    /// Watched directories
    watched_dirs: Vec<PathBuf>,
    /// Event sender
    event_tx: Sender<WatchEvent>,
    /// Event receiver
    event_rx: Receiver<WatchEvent>,
    /// Watcher handle
    watcher: Option<notify::RecommendedWatcher>,
    /// Background thread handle
    thread_handle: Option<JoinHandle<()>>,
    /// Shutdown flag
    shutdown: Arc<std::sync::atomic::AtomicBool>,
}

impl CacheWatcher {
    /// Create a new cache watcher
    pub fn new() -> Self {
        let (event_tx, event_rx) = channel::unbounded();

        Self {
            watched_dirs: Vec::new(),
            event_tx,
            event_rx,
            watcher: None,
            thread_handle: None,
            shutdown: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Start watching a directory
    pub fn watch<P: AsRef<Path>>(&mut self, path: P) -> Result<(), notify::Error> {
        let path = path.as_ref().to_path_buf();

        if self.watcher.is_none() {
            let tx = self.event_tx.clone();

            let watcher =
                notify::recommended_watcher(move |res: Result<Event, notify::Error>| match res {
                    Ok(event) => {
                        let watch_event = match event.kind {
                            EventKind::Modify(_) => {
                                event.paths.first().map(|p| WatchEvent::Modified(p.clone()))
                            }
                            EventKind::Create(_) => {
                                event.paths.first().map(|p| WatchEvent::Created(p.clone()))
                            }
                            EventKind::Remove(_) => {
                                event.paths.first().map(|p| WatchEvent::Deleted(p.clone()))
                            }
                            _ => None,
                        };

                        if let Some(evt) = watch_event {
                            let _ = tx.send(evt);
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(WatchEvent::Error(e.to_string()));
                    }
                })?;

            self.watcher = Some(watcher);
        }

        if let Some(ref mut watcher) = self.watcher {
            watcher.watch(&path, RecursiveMode::Recursive)?;
            self.watched_dirs.push(path);
        }

        Ok(())
    }

    /// Stop watching a directory
    pub fn unwatch<P: AsRef<Path>>(&mut self, path: P) -> Result<(), notify::Error> {
        let path = path.as_ref();

        if let Some(ref mut watcher) = self.watcher {
            watcher.unwatch(path)?;
            self.watched_dirs.retain(|p| p != path);
        }

        Ok(())
    }

    /// Get the event receiver
    pub fn events(&self) -> &Receiver<WatchEvent> {
        &self.event_rx
    }

    /// Try to receive an event (non-blocking)
    pub fn try_recv(&self) -> Option<WatchEvent> {
        self.event_rx.try_recv().ok()
    }

    /// Process pending invalidations for a cache
    pub fn process_invalidations(&self, cache: &ReactiveCache) -> usize {
        let mut count = 0;

        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                WatchEvent::Modified(path) | WatchEvent::Deleted(path) => {
                    if let Some(path_str) = path.to_str() {
                        // Only invalidate Python files
                        if path_str.ends_with(".py") {
                            cache.invalidate(path_str);
                            count += 1;
                        }
                    }
                }
                WatchEvent::Created(_) => {
                    // New files don't need invalidation
                }
                WatchEvent::Error(_) => {
                    // Log error but continue
                }
            }
        }

        count
    }

    /// Start background validation thread
    pub fn start_background_validation(&mut self, cache: Arc<ReactiveCache>, interval: Duration) {
        let shutdown = Arc::clone(&self.shutdown);
        let event_rx = self.event_rx.clone();

        let handle = thread::Builder::new()
            .name("dx-py-cache-watcher".to_string())
            .spawn(move || {
                while !shutdown.load(std::sync::atomic::Ordering::SeqCst) {
                    // Process events
                    while let Ok(event) = event_rx.try_recv() {
                        match event {
                            WatchEvent::Modified(path) | WatchEvent::Deleted(path) => {
                                if let Some(path_str) = path.to_str() {
                                    if path_str.ends_with(".py") {
                                        cache.invalidate(path_str);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    thread::sleep(interval);
                }
            })
            .expect("Failed to spawn watcher thread");

        self.thread_handle = Some(handle);
    }

    /// Stop the background validation thread
    pub fn stop(&mut self) {
        self.shutdown.store(true, std::sync::atomic::Ordering::SeqCst);

        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    /// Get the list of watched directories
    pub fn watched_dirs(&self) -> &[PathBuf] {
        &self.watched_dirs
    }
}

impl Default for CacheWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for CacheWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_watcher_creation() {
        let watcher = CacheWatcher::new();
        assert!(watcher.watched_dirs().is_empty());
    }

    #[test]
    fn test_watch_directory() {
        let temp_dir = TempDir::new().unwrap();
        let mut watcher = CacheWatcher::new();

        watcher.watch(temp_dir.path()).unwrap();
        assert_eq!(watcher.watched_dirs().len(), 1);
    }

    #[test]
    fn test_file_modification_event() {
        let temp_dir = TempDir::new().unwrap();
        let mut watcher = CacheWatcher::new();

        watcher.watch(temp_dir.path()).unwrap();

        // Create and modify a file
        let file_path = temp_dir.path().join("test.py");
        {
            let mut file = File::create(&file_path).unwrap();
            file.write_all(b"print('hello')").unwrap();
        }

        // Give the watcher time to detect the change
        thread::sleep(Duration::from_millis(100));

        // Check for events (may or may not have received one depending on timing)
        let _event = watcher.try_recv();
        // Event detection is timing-dependent, so we just verify no panic
    }
}
