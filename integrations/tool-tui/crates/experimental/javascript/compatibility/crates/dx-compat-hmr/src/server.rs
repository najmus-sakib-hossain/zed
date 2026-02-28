//! HMR server implementation.
//!
//! Provides file watching and WebSocket-based client communication.

use crate::error::{HmrError, HmrResult};
use crate::graph::{DependencyGraph, SharedDependencyGraph};
use crate::update::{HmrUpdate, UpdateType};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::RwLock;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::broadcast;

/// HMR server for file watching and update broadcasting.
pub struct HmrServer {
    root: PathBuf,
    graph: SharedDependencyGraph,
    watcher: Option<RecommendedWatcher>,
    update_tx: broadcast::Sender<HmrUpdate>,
    watched_extensions: RwLock<HashSet<String>>,
}

impl HmrServer {
    /// Create a new HMR server.
    pub fn new(root: impl AsRef<Path>) -> HmrResult<Self> {
        let (update_tx, _) = broadcast::channel(100);

        let mut extensions = HashSet::new();
        extensions.insert("js".to_string());
        extensions.insert("jsx".to_string());
        extensions.insert("ts".to_string());
        extensions.insert("tsx".to_string());
        extensions.insert("css".to_string());
        extensions.insert("scss".to_string());
        extensions.insert("less".to_string());

        Ok(Self {
            root: root.as_ref().to_path_buf(),
            graph: Arc::new(DependencyGraph::new()),
            watcher: None,
            update_tx,
            watched_extensions: RwLock::new(extensions),
        })
    }

    /// Get the dependency graph.
    pub fn graph(&self) -> SharedDependencyGraph {
        Arc::clone(&self.graph)
    }

    /// Subscribe to HMR updates.
    pub fn subscribe(&self) -> broadcast::Receiver<HmrUpdate> {
        self.update_tx.subscribe()
    }

    /// Add a file extension to watch.
    pub fn watch_extension(&self, ext: &str) {
        self.watched_extensions.write().insert(ext.to_string());
    }

    /// Start watching for file changes.
    pub fn start(&mut self) -> HmrResult<()> {
        let update_tx = self.update_tx.clone();
        let root = self.root.clone();
        let extensions = self.watched_extensions.read().clone();
        let graph = Arc::clone(&self.graph);

        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    Self::handle_event(&event, &root, &extensions, &graph, &update_tx);
                }
            },
            Config::default(),
        )?;

        watcher.watch(&self.root, RecursiveMode::Recursive)?;
        self.watcher = Some(watcher);

        Ok(())
    }

    /// Stop watching for file changes.
    pub fn stop(&mut self) {
        self.watcher = None;
    }

    fn handle_event(
        event: &Event,
        root: &Path,
        extensions: &HashSet<String>,
        graph: &DependencyGraph,
        update_tx: &broadcast::Sender<HmrUpdate>,
    ) {
        use notify::EventKind;

        match event.kind {
            EventKind::Modify(_) | EventKind::Create(_) => {
                for path in &event.paths {
                    if let Some(update) = Self::create_update(path, root, extensions, graph) {
                        let _ = update_tx.send(update);
                    }
                }
            }
            EventKind::Remove(_) => {
                for path in &event.paths {
                    // Remove from dependency graph
                    graph.remove_module(path);
                }
            }
            _ => {}
        }
    }

    fn create_update(
        path: &Path,
        root: &Path,
        extensions: &HashSet<String>,
        graph: &DependencyGraph,
    ) -> Option<HmrUpdate> {
        let ext = path.extension()?.to_str()?;

        if !extensions.contains(ext) {
            return None;
        }

        let relative_path = path.strip_prefix(root).ok()?;
        let path_str = relative_path.to_string_lossy().to_string();

        // Compute simple hash based on modification time
        let hash = std::fs::metadata(path)
            .ok()
            .and_then(|m| m.modified().ok())
            .map(|t| {
                format!(
                    "{:x}",
                    t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()
                )
            })
            .unwrap_or_else(|| "unknown".to_string());

        let update_type = match ext {
            "css" | "scss" | "less" => UpdateType::Css,
            "js" | "jsx" | "ts" | "tsx" => {
                // Check if module can accept updates
                if graph.has_module(path) {
                    UpdateType::Js
                } else {
                    UpdateType::FullReload
                }
            }
            _ => UpdateType::FullReload,
        };

        Some(HmrUpdate {
            path: path_str,
            hash,
            update_type,
        })
    }

    /// Manually trigger an update for a file.
    pub fn trigger_update(&self, path: impl AsRef<Path>) -> HmrResult<()> {
        let path = path.as_ref();
        let extensions = self.watched_extensions.read().clone();

        if let Some(update) = Self::create_update(path, &self.root, &extensions, &self.graph) {
            self.update_tx.send(update).map_err(|e| HmrError::UpdateFailed(e.to_string()))?;
        }

        Ok(())
    }

    /// Get all modules that need to be invalidated when a file changes.
    pub fn get_invalidated_modules(&self, path: impl AsRef<Path>) -> Vec<PathBuf> {
        let mut modules = self.graph.get_dependents(&path);
        modules.push(path.as_ref().to_path_buf());
        modules
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_server_creation() {
        let temp_dir = TempDir::new().unwrap();
        let server = HmrServer::new(temp_dir.path()).unwrap();
        assert!(server.watcher.is_none());
    }

    #[test]
    fn test_watch_extension() {
        let temp_dir = TempDir::new().unwrap();
        let server = HmrServer::new(temp_dir.path()).unwrap();

        server.watch_extension("vue");
        assert!(server.watched_extensions.read().contains("vue"));
    }

    #[test]
    fn test_subscribe() {
        let temp_dir = TempDir::new().unwrap();
        let server = HmrServer::new(temp_dir.path()).unwrap();

        let _rx = server.subscribe();
        // Should not panic
    }
}
