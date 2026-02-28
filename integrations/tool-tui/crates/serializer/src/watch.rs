//! File Watcher for .sr and dx config files
//!
//! Monitors filesystem for changes to:
//! - Root `dx` config file (no extension)
//! - All `.sr` rule definition files
//!
//! Provides hot-reload capability for development mode.
//!
//! ## Output Directory
//!
//! When .sr files change, outputs are generated to `.dx/serializer/`:
//! - `{name}.human` - Human-readable format for editor display
//! - `{name}.machine` - Binary format for runtime loading

use std::path::PathBuf;

#[cfg(feature = "watch")]
use notify::{
    Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher,
};
#[cfg(feature = "watch")]
use std::path::Path;
#[cfg(feature = "watch")]
use std::sync::mpsc::{Receiver, Sender, channel};
#[cfg(feature = "watch")]
use std::time::Duration;

/// File change event
#[derive(Debug, Clone)]
pub enum FileChange {
    /// Root dx config file changed
    ConfigChanged(PathBuf),
    /// A .sr rule file changed
    RuleFileChanged(PathBuf),
    /// A .sr rule file was created
    RuleFileCreated(PathBuf),
    /// A .sr rule file was deleted
    RuleFileDeleted(PathBuf),
}
/// File watcher for dx config and .sr files
#[cfg(feature = "watch")]
pub struct DxWatcher {
    watcher: RecommendedWatcher,
    receiver: Receiver<FileChange>,
    watched_paths: Vec<PathBuf>,
}

#[cfg(feature = "watch")]
impl DxWatcher {
    /// Create a new file watcher
    ///
    /// # Arguments
    /// * `debounce_ms` - Debounce delay in milliseconds (prevents duplicate events)
    ///
    /// # Example
    /// ```no_run
    /// use serializer::watch::DxWatcher;
    ///
    /// let mut watcher = DxWatcher::new(250).unwrap();
    /// watcher.watch_directory(".").unwrap();
    ///
    /// for change in watcher.changes() {
    ///     println!("File changed: {:?}", change);
    /// }
    /// ```
    pub fn new(debounce_ms: u64) -> Result<Self, notify::Error> {
        let (tx, rx) = channel();

        let watcher = Self::create_watcher(tx, debounce_ms)?;

        Ok(Self {
            watcher,
            receiver: rx,
            watched_paths: Vec::new(),
        })
    }

    /// Create the underlying notify watcher
    fn create_watcher(
        tx: Sender<FileChange>,
        debounce_ms: u64,
    ) -> Result<RecommendedWatcher, notify::Error> {
        let mut config = Config::default();
        config = config.with_poll_interval(Duration::from_millis(debounce_ms));

        RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    if let Some(change) = Self::process_event(event) {
                        let _ = tx.send(change);
                    }
                }
            },
            config,
        )
    }

    /// Process a notify event into a FileChange
    fn process_event(event: Event) -> Option<FileChange> {
        let path = event.paths.first()?;

        // Check if it's a dx config file
        if path.file_name()?.to_str()? == "dx" {
            return Some(match event.kind {
                EventKind::Create(_) | EventKind::Modify(_) => {
                    FileChange::ConfigChanged(path.clone())
                }
                _ => return None,
            });
        }

        // Check if it's a .sr file
        if path.extension()?.to_str()? == "sr" {
            return Some(match event.kind {
                EventKind::Create(_) => FileChange::RuleFileCreated(path.clone()),
                EventKind::Modify(_) => FileChange::RuleFileChanged(path.clone()),
                EventKind::Remove(_) => FileChange::RuleFileDeleted(path.clone()),
                _ => return None,
            });
        }

        None
    }

    /// Watch a directory for dx and .sr files
    ///
    /// This will recursively watch the directory and all subdirectories.
    pub fn watch_directory<P: AsRef<Path>>(&mut self, path: P) -> Result<(), notify::Error> {
        let path = path.as_ref();
        self.watcher.watch(path, RecursiveMode::Recursive)?;
        self.watched_paths.push(path.to_path_buf());
        Ok(())
    }

    /// Watch a specific file
    pub fn watch_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), notify::Error> {
        let path = path.as_ref();
        self.watcher.watch(path, RecursiveMode::NonRecursive)?;
        self.watched_paths.push(path.to_path_buf());
        Ok(())
    }

    /// Stop watching a path
    pub fn unwatch<P: AsRef<Path>>(&mut self, path: P) -> Result<(), notify::Error> {
        let path = path.as_ref();
        self.watcher.unwatch(path)?;
        self.watched_paths.retain(|p| p != path);
        Ok(())
    }

    /// Get an iterator over file changes
    ///
    /// This will block until a change occurs. Use `try_recv()` for non-blocking.
    pub fn changes(&self) -> impl Iterator<Item = FileChange> + '_ {
        std::iter::from_fn(move || self.receiver.recv().ok())
    }

    /// Try to receive a change without blocking
    pub fn try_recv(&self) -> Option<FileChange> {
        self.receiver.try_recv().ok()
    }

    /// Get all currently watched paths
    pub fn watched_paths(&self) -> &[PathBuf] {
        &self.watched_paths
    }
}

/// Helper function to find all .sr files in a directory
#[cfg(feature = "watch")]
pub fn find_dxs_files<P: AsRef<Path>>(dir: P) -> Result<Vec<PathBuf>, std::io::Error> {
    use std::fs;

    let mut dxs_files = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "sr" {
                    dxs_files.push(path);
                }
            }
        } else if path.is_dir() {
            // Recursively search subdirectories
            dxs_files.extend(find_dxs_files(&path)?);
        }
    }

    Ok(dxs_files)
}

/// Helper function to find the root dx config file
#[cfg(feature = "watch")]
pub fn find_dx_config<P: AsRef<Path>>(start_dir: P) -> Option<PathBuf> {
    let mut current = start_dir.as_ref().to_path_buf();

    loop {
        let config_path = current.join("dx");
        if config_path.exists() && config_path.is_file() {
            return Some(config_path);
        }

        // Move up one directory
        if !current.pop() {
            break;
        }
    }

    None
}

/// Stub implementations when watch feature is not enabled
#[cfg(not(feature = "watch"))]
pub struct DxWatcher;

#[cfg(not(feature = "watch"))]
impl DxWatcher {
    pub fn new(_debounce_ms: u64) -> Result<Self, &'static str> {
        Err("Watch feature not enabled. Enable with --features watch")
    }
}

#[cfg(test)]
#[cfg(feature = "watch")]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_find_dxs_files() {
        let temp = tempdir().unwrap();
        let rules_dir = temp.path().join("rules");
        fs::create_dir(&rules_dir).unwrap();

        // Create some .sr files
        fs::write(rules_dir.join("js-rules.sr"), "# JS rules").unwrap();
        fs::write(rules_dir.join("py-rules.sr"), "# Python rules").unwrap();
        fs::write(rules_dir.join("other.txt"), "not a rule file").unwrap();

        let dxs_files = find_dxs_files(&rules_dir).unwrap();
        assert_eq!(dxs_files.len(), 2);

        let names: Vec<_> = dxs_files.iter().filter_map(|p| p.file_name()?.to_str()).collect();
        assert!(names.contains(&"js-rules.sr"));
        assert!(names.contains(&"py-rules.sr"));
    }

    #[test]
    fn test_find_dx_config() {
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("dx");
        fs::write(&config_path, "# dx config").unwrap();

        let found = find_dx_config(temp.path()).unwrap();
        assert_eq!(found, config_path);
    }

    #[test]
    fn test_find_dx_config_in_parent() {
        let temp = tempdir().unwrap();
        let config_path = temp.path().join("dx");
        fs::write(&config_path, "# dx config").unwrap();

        // Create subdirectory
        let sub_dir = temp.path().join("src");
        fs::create_dir(&sub_dir).unwrap();

        // Should find config in parent
        let found = find_dx_config(&sub_dir).unwrap();
        assert_eq!(found, config_path);
    }

    #[test]
    fn test_watcher_creation() {
        let watcher = DxWatcher::new(250);
        assert!(watcher.is_ok());
    }
}
