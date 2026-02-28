//! File system watcher for auto-sync

use crate::{DrivenError, Result};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::{Receiver, channel};

/// Watches for file changes
#[derive(Debug)]
pub struct FileWatcher {
    /// The watcher instance
    _watcher: RecommendedWatcher,
    /// Receiver for events
    rx: Receiver<notify::Result<notify::Event>>,
}

impl FileWatcher {
    /// Create a new file watcher
    pub fn new(path: &Path) -> Result<Self> {
        let (tx, rx) = channel();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default(),
        )
        .map_err(|e| DrivenError::Sync(format!("Failed to create watcher: {}", e)))?;

        watcher
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| DrivenError::Sync(format!("Failed to watch path: {}", e)))?;

        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    /// Get pending events (non-blocking)
    pub fn pending_events(&self) -> Vec<WatchEvent> {
        let mut events = Vec::new();

        while let Ok(result) = self.rx.try_recv() {
            if let Ok(event) = result {
                events.push(WatchEvent::from_notify(event));
            }
        }

        events
    }

    /// Wait for next event (blocking)
    pub fn wait_for_event(&self) -> Option<WatchEvent> {
        self.rx.recv().ok().and_then(|r| r.ok()).map(WatchEvent::from_notify)
    }
}

/// Simplified watch event
#[derive(Debug, Clone)]
pub struct WatchEvent {
    /// Event kind
    pub kind: WatchEventKind,
    /// Affected paths
    pub paths: Vec<std::path::PathBuf>,
}

/// Watch event kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchEventKind {
    /// File created
    Create,
    /// File modified
    Modify,
    /// File deleted
    Remove,
    /// Other event
    Other,
}

impl WatchEvent {
    fn from_notify(event: notify::Event) -> Self {
        let kind = match event.kind {
            notify::EventKind::Create(_) => WatchEventKind::Create,
            notify::EventKind::Modify(_) => WatchEventKind::Modify,
            notify::EventKind::Remove(_) => WatchEventKind::Remove,
            _ => WatchEventKind::Other,
        };

        Self {
            kind,
            paths: event.paths,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watch_event_kind() {
        assert_ne!(WatchEventKind::Create, WatchEventKind::Modify);
    }
}
