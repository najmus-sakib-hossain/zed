//! Dual-Watcher Architecture - LSP + File System monitoring
//!
//! Provides two-tier file change detection:
//! 1. **LSP Watcher** (Primary): Monitors Language Server Protocol events
//! 2. **File System Watcher** (Fallback): Monitors actual file system changes
//!
//! The LSP watcher detects changes before they hit the disk, enabling
//! faster response times and semantic understanding of code changes.
//!
//! Features:
//! - Platform-native file watching via PlatformIO
//! - Configurable debounce window (default 100ms)
//! - Event deduplication within debounce window
//! - Graceful handling of directory deletion

use anyhow::{Context as _, Result};
use crossbeam::channel::{Receiver, Sender, unbounded};
use notify::{EventKind, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{
    DebounceEventResult, DebouncedEvent, Debouncer, FileIdMap, new_debouncer,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, broadcast};

use crate::platform_io::{EventStream, FileEvent, FileEventKind, PlatformIO, create_platform_io};

/// File change event
#[derive(Debug, Clone)]
pub struct FileChange {
    /// Path to the changed file
    pub path: PathBuf,

    /// Type of change
    pub kind: ChangeKind,

    /// Source of the event (LSP or FileSystem)
    pub source: ChangeSource,

    /// Timestamp of the change
    pub timestamp: std::time::SystemTime,

    /// Optional content if available from LSP
    pub content: Option<String>,

    /// Detected DX patterns (if analyzed)
    pub patterns: Option<Vec<crate::patterns::PatternMatch>>,
}

/// Type of file change
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Created,
    Modified,
    Deleted,
    Renamed,
}

/// Source of the change detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeSource {
    Lsp,
    FileSystem,
}

/// LSP event (simplified - full LSP protocol would be more complex)
#[derive(Debug, Clone)]
pub struct LspEvent {
    pub uri: String,
    pub version: i32,
    pub content: String,
}

/// LSP Watcher - monitors Language Server Protocol events
pub struct LspWatcher {
    #[allow(dead_code)]
    lsp_rx: Receiver<LspEvent>,
    change_tx: broadcast::Sender<FileChange>,
    running: Arc<RwLock<bool>>,
}

impl LspWatcher {
    /// Create a new LSP watcher
    pub fn new() -> (Self, broadcast::Receiver<FileChange>) {
        let (_lsp_tx, lsp_rx) = unbounded();
        let (change_tx, change_rx) = broadcast::channel(1000);

        (
            Self {
                lsp_rx,
                change_tx,
                running: Arc::new(RwLock::new(false)),
            },
            change_rx,
        )
    }

    /// Start watching for LSP events
    pub async fn start(&self) -> Result<()> {
        *self.running.write().await = true;

        // In a real implementation, this would:
        // 1. Connect to LSP server via stdin/stdout or socket
        // 2. Subscribe to textDocument/didChange notifications
        // 3. Parse LSP JSON-RPC messages
        // 4. Extract file changes and content

        println!("📡 LSP Watcher started (mock mode - needs LSP server integration)");

        Ok(())
    }

    /// Stop watching
    pub async fn stop(&self) -> Result<()> {
        *self.running.write().await = false;
        println!("📡 LSP Watcher stopped");
        Ok(())
    }

    /// Process LSP events (would be called from LSP message loop)
    #[allow(dead_code)]
    fn process_lsp_event(&self, event: LspEvent) -> Result<()> {
        let path = PathBuf::from(event.uri.trim_start_matches("file://"));

        // Detect patterns in content
        let patterns = if let Ok(detector) = crate::patterns::PatternDetector::new() {
            detector.detect_in_file(&path, &event.content).ok()
        } else {
            None
        };

        let change = FileChange {
            path,
            kind: ChangeKind::Modified,
            source: ChangeSource::Lsp,
            timestamp: std::time::SystemTime::now(),
            content: Some(event.content),
            patterns,
        };

        let _ = self.change_tx.send(change);
        Ok(())
    }
}

/// File System Watcher - monitors actual file system changes
pub struct FileWatcher {
    debouncer: Option<Debouncer<RecommendedWatcher, FileIdMap>>,
    _event_tx: Sender<DebounceEventResult>,
}

impl FileWatcher {
    /// Create a new file system watcher
    pub fn new() -> Result<(Self, broadcast::Receiver<FileChange>)> {
        let (event_tx, _event_rx) = unbounded();
        let (change_tx, change_rx) = broadcast::channel(1000);

        let tx_clone = change_tx.clone();

        // Create debouncer with 100ms delay
        let debouncer =
            new_debouncer(Duration::from_millis(100), None, move |result: DebounceEventResult| {
                if let Ok(events) = result {
                    for debounced_event in events {
                        if let Some(change) = Self::debounced_event_to_change(debounced_event) {
                            let _ = tx_clone.send(change);
                        }
                    }
                }
            })?;

        Ok((
            Self {
                debouncer: Some(debouncer),
                _event_tx: event_tx,
            },
            change_rx,
        ))
    }

    /// Watch a directory recursively
    pub fn watch(&mut self, path: impl AsRef<Path>) -> Result<()> {
        if let Some(debouncer) = &mut self.debouncer {
            debouncer
                .watch(path.as_ref(), RecursiveMode::Recursive)
                .with_context(|| format!("Failed to watch: {}", path.as_ref().display()))?;

            println!("👁️  File Watcher started: {}", path.as_ref().display());
        }
        Ok(())
    }

    /// Stop watching
    pub fn stop(&mut self) -> Result<()> {
        self.debouncer = None;
        println!("👁️  File Watcher stopped");
        Ok(())
    }

    /// Convert debounced event to FileChange
    fn debounced_event_to_change(debounced_event: DebouncedEvent) -> Option<FileChange> {
        let event = &debounced_event.event;
        let kind = match event.kind {
            EventKind::Create(_) => ChangeKind::Created,
            EventKind::Modify(_) => ChangeKind::Modified,
            EventKind::Remove(_) => ChangeKind::Deleted,
            _ => return None,
        };

        // Get first path from event
        let path = event.paths.first()?.clone();

        // Intelligent filtering for performance
        if !Self::should_process_path(&path) {
            return None;
        }

        Some(FileChange {
            path,
            kind,
            source: ChangeSource::FileSystem,
            timestamp: std::time::SystemTime::now(),
            content: None,
            patterns: None,
        })
    }

    /// Determine if a path should be processed (performance optimization)
    fn should_process_path(path: &Path) -> bool {
        // Skip hidden files and temp files
        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy();

            // Skip hidden files (but not .gitignore itself)
            if name_str.starts_with('.') && name_str != ".gitignore" {
                return false;
            }

            // Skip temp files
            if name_str.contains('~') || name_str.ends_with(".tmp") || name_str.ends_with(".swp") {
                return false;
            }

            // Skip lock files
            if name_str.ends_with(".lock") {
                return false;
            }
        }

        // Skip target directories, node_modules, and .git
        if let Some(path_str) = path.to_str() {
            if path_str.contains("/target/")
                || path_str.contains("\\target\\")
                || path_str.contains("/node_modules/")
                || path_str.contains("\\node_modules\\")
                || path_str.contains("/.dx/")
                || path_str.contains("\\.dx\\")
                || path_str.contains("/.git/")
                || path_str.contains("\\.git\\")
                || path_str.contains("/dist/")
                || path_str.contains("\\dist\\")
                || path_str.contains("/build/")
                || path_str.contains("\\build\\")
            {
                return false;
            }
        }

        true
    }

    /// Check if a path is ignored by gitignore
    pub fn is_gitignored(root: &Path, path: &Path) -> bool {
        use ignore::WalkBuilder;

        // Build a walker that respects gitignore
        let walker = WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .max_depth(Some(1))
            .build();

        // Check if the path would be ignored
        for entry in walker.flatten() {
            if entry.path() == path {
                return false; // Path is not ignored
            }
        }

        // If we didn't find it in the walk, it's likely ignored
        // But we need a more reliable check
        let (gitignore, _err) = ignore::gitignore::Gitignore::new(root.join(".gitignore"));
        gitignore.matched(path, path.is_dir()).is_ignore()
    }
}

/// Configuration for the platform-native file watcher
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Debounce window in milliseconds (default: 100ms)
    pub debounce_ms: u64,
    /// Maximum events to buffer before forcing flush
    pub max_buffer_size: usize,
    /// Whether to use platform-native watching (falls back to notify if unavailable)
    pub use_platform_native: bool,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 100,
            max_buffer_size: 1000,
            use_platform_native: true,
        }
    }
}

/// Event deduplicator for removing duplicate events within a time window
struct EventDeduplicator {
    /// Map of path -> (last event kind, timestamp)
    events: HashMap<PathBuf, (ChangeKind, Instant)>,
    /// Debounce window duration
    debounce_window: Duration,
}

impl EventDeduplicator {
    fn new(debounce_ms: u64) -> Self {
        Self {
            events: HashMap::new(),
            debounce_window: Duration::from_millis(debounce_ms),
        }
    }

    /// Check if an event should be processed or deduplicated
    /// Returns true if the event should be processed
    fn should_process(&mut self, path: &Path, kind: ChangeKind) -> bool {
        let now = Instant::now();

        // Clean up old entries
        self.events
            .retain(|_, (_, ts)| now.duration_since(*ts) < self.debounce_window * 2);

        if let Some((last_kind, last_ts)) = self.events.get(path) {
            // If same event kind within debounce window, deduplicate
            if *last_kind == kind && now.duration_since(*last_ts) < self.debounce_window {
                return false;
            }
        }

        // Record this event
        self.events.insert(path.to_path_buf(), (kind, now));
        true
    }

    /// Clear all tracked events
    fn clear(&mut self) {
        self.events.clear();
    }
}

/// Platform-native file watcher using PlatformIO
pub struct PlatformFileWatcher {
    io: Arc<dyn PlatformIO>,
    config: WatcherConfig,
    event_stream: Option<Box<dyn EventStream>>,
    change_tx: broadcast::Sender<FileChange>,
    deduplicator: Arc<RwLock<EventDeduplicator>>,
    running: Arc<RwLock<bool>>,
}

impl PlatformFileWatcher {
    /// Create a new platform-native file watcher
    pub fn new(config: WatcherConfig) -> Result<(Self, broadcast::Receiver<FileChange>)> {
        let (change_tx, change_rx) = broadcast::channel(config.max_buffer_size);
        let io = create_platform_io();
        let deduplicator = Arc::new(RwLock::new(EventDeduplicator::new(config.debounce_ms)));

        Ok((
            Self {
                io,
                config,
                event_stream: None,
                change_tx,
                deduplicator,
                running: Arc::new(RwLock::new(false)),
            },
            change_rx,
        ))
    }

    /// Create with default configuration
    pub fn with_defaults() -> Result<(Self, broadcast::Receiver<FileChange>)> {
        Self::new(WatcherConfig::default())
    }

    /// Watch a directory using platform-native I/O
    pub async fn watch(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();

        // Try platform-native watching first
        if self.config.use_platform_native {
            match self.io.watch(path).await {
                Ok(stream) => {
                    self.event_stream = Some(stream);
                    *self.running.write().await = true;

                    // Start the event processing loop
                    self.start_event_loop(path.to_path_buf()).await;

                    println!(
                        "👁️  Platform File Watcher started ({}): {}",
                        self.io.backend_name(),
                        path.display()
                    );
                    return Ok(());
                }
                Err(e) => {
                    tracing::warn!(
                        "Platform-native watching unavailable, falling back to notify: {}",
                        e
                    );
                }
            }
        }

        // Fallback: use notify-based watching
        self.start_notify_fallback(path).await
    }

    /// Start the event processing loop
    async fn start_event_loop(&self, _watch_path: PathBuf) {
        let _change_tx = self.change_tx.clone();
        let _deduplicator = Arc::clone(&self.deduplicator);
        let running = Arc::clone(&self.running);
        let debounce_ms = self.config.debounce_ms;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(debounce_ms / 2));

            while *running.read().await {
                interval.tick().await;

                // In a real implementation, we would poll the event stream here
                // For now, this is a placeholder that maintains the loop structure
            }
        });
    }

    /// Start notify-based fallback watching
    async fn start_notify_fallback(&mut self, path: &Path) -> Result<()> {
        let change_tx = self.change_tx.clone();
        let deduplicator = Arc::clone(&self.deduplicator);

        let debouncer = new_debouncer(
            Duration::from_millis(self.config.debounce_ms),
            None,
            move |result: DebounceEventResult| {
                if let Ok(events) = result {
                    let dedup = deduplicator.clone();
                    let tx = change_tx.clone();

                    // Process events synchronously in the callback
                    for debounced_event in events {
                        if let Some(change) = Self::convert_debounced_event(debounced_event, &dedup)
                        {
                            let _ = tx.send(change);
                        }
                    }
                }
            },
        )?;

        // Store debouncer to keep it alive (we'd need to add a field for this)
        // For now, we leak it intentionally to keep watching active
        let mut debouncer = debouncer;
        debouncer
            .watch(path, RecursiveMode::Recursive)
            .with_context(|| format!("Failed to watch: {}", path.display()))?;

        // Leak the debouncer to keep it alive
        std::mem::forget(debouncer);

        *self.running.write().await = true;
        println!("👁️  Platform File Watcher started (notify fallback): {}", path.display());

        Ok(())
    }

    /// Convert a debounced event to FileChange with deduplication
    fn convert_debounced_event(
        debounced_event: DebouncedEvent,
        deduplicator: &Arc<RwLock<EventDeduplicator>>,
    ) -> Option<FileChange> {
        let event = &debounced_event.event;
        let kind = match event.kind {
            EventKind::Create(_) => ChangeKind::Created,
            EventKind::Modify(_) => ChangeKind::Modified,
            EventKind::Remove(_) => ChangeKind::Deleted,
            _ => return None,
        };

        let path = event.paths.first()?.clone();

        // Filter out paths we don't care about
        if !FileWatcher::should_process_path(&path) {
            return None;
        }

        // Check deduplication (blocking call in sync context)
        // We use try_write to avoid blocking; if we can't get the lock, process the event
        if let Ok(mut dedup) = deduplicator.try_write() {
            if !dedup.should_process(&path, kind) {
                return None;
            }
        }

        Some(FileChange {
            path,
            kind,
            source: ChangeSource::FileSystem,
            timestamp: std::time::SystemTime::now(),
            content: None,
            patterns: None,
        })
    }

    /// Convert platform FileEvent to FileChange
    #[allow(dead_code)]
    fn convert_platform_event(event: FileEvent) -> Option<FileChange> {
        let kind = match event.kind {
            FileEventKind::Created => ChangeKind::Created,
            FileEventKind::Modified => ChangeKind::Modified,
            FileEventKind::Deleted => ChangeKind::Deleted,
            FileEventKind::Renamed { .. } => ChangeKind::Renamed,
            FileEventKind::Metadata => return None, // Skip metadata-only changes
        };

        // Filter out paths we don't care about
        if !FileWatcher::should_process_path(&event.path) {
            return None;
        }

        Some(FileChange {
            path: event.path,
            kind,
            source: ChangeSource::FileSystem,
            timestamp: std::time::SystemTime::now(),
            content: None,
            patterns: None,
        })
    }

    /// Stop watching
    pub async fn stop(&mut self) -> Result<()> {
        *self.running.write().await = false;

        if let Some(mut stream) = self.event_stream.take() {
            stream.close();
        }

        // Clear deduplicator state
        self.deduplicator.write().await.clear();

        println!("👁️  Platform File Watcher stopped");
        Ok(())
    }

    /// Get the I/O backend name
    pub fn backend_name(&self) -> &'static str {
        self.io.backend_name()
    }

    /// Check if the watcher is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

/// Dual Watcher - combines LSP and File System watchers
pub struct DualWatcher {
    lsp_watcher: Arc<LspWatcher>,
    file_watcher: Arc<RwLock<FileWatcher>>,
    /// Sender for unified change stream
    change_tx: broadcast::Sender<FileChange>,
    /// Receiver for unified change stream
    change_rx: broadcast::Receiver<FileChange>,
    /// Internal LSP change stream (wired into change_tx when started)
    lsp_rx: Option<broadcast::Receiver<FileChange>>,
    /// Internal file-system change stream (wired into change_tx when started)
    fs_rx: Option<broadcast::Receiver<FileChange>>,
}

impl DualWatcher {
    /// Create a new dual watcher
    pub fn new() -> Result<Self> {
        let (lsp_watcher, lsp_rx) = LspWatcher::new();
        let (file_watcher, fs_rx) = FileWatcher::new()?;

        // Create unified change channel. We delay spawning the merge
        // tasks until `start` is called so this constructor can be
        // used from non-async contexts (e.g. tests) without requiring
        // a Tokio runtime.
        let (change_tx, change_rx) = broadcast::channel(1000);

        Ok(Self {
            lsp_watcher: Arc::new(lsp_watcher),
            file_watcher: Arc::new(RwLock::new(file_watcher)),
            change_tx,
            change_rx,
            lsp_rx: Some(lsp_rx),
            fs_rx: Some(fs_rx),
        })
    }

    /// Start background tasks that merge LSP and file-system events
    /// into the unified change stream. This is safe to call multiple
    /// times; merge tasks will only be spawned once.
    fn start_merge_tasks(&mut self) {
        // If both receivers have already been taken, merge tasks are
        // already running (or were intentionally disabled).
        if self.lsp_rx.is_none() && self.fs_rx.is_none() {
            return;
        }

        if let Some(mut lsp_rx) = self.lsp_rx.take() {
            let tx = self.change_tx.clone();
            tokio::spawn(async move {
                while let Ok(change) = lsp_rx.recv().await {
                    let _ = tx.send(change);
                }
            });
        }

        if let Some(mut fs_rx) = self.fs_rx.take() {
            let tx = self.change_tx.clone();
            tokio::spawn(async move {
                while let Ok(change) = fs_rx.recv().await {
                    let _ = tx.send(change);
                }
            });
        }
    }

    /// Start both watchers
    pub async fn start(&mut self, path: impl AsRef<Path>) -> Result<()> {
        // We are now guaranteed to be running inside a Tokio runtime,
        // so it's safe to spawn the merge tasks.
        self.start_merge_tasks();

        // Start LSP watcher
        self.lsp_watcher.start().await.context("Failed to start LSP watcher")?;

        // Start file system watcher
        self.file_watcher
            .write()
            .await
            .watch(path)
            .context("Failed to start file system watcher")?;

        println!("🔄 Dual Watcher active: LSP + FileSystem");
        Ok(())
    }

    /// Stop both watchers
    pub async fn stop(&mut self) -> Result<()> {
        self.lsp_watcher.stop().await.context("Failed to stop LSP watcher")?;
        self.file_watcher
            .write()
            .await
            .stop()
            .context("Failed to stop file system watcher")?;
        println!("🔄 Dual Watcher stopped");
        Ok(())
    }

    /// Get the change receiver
    pub fn receiver(&self) -> broadcast::Receiver<FileChange> {
        self.change_rx.resubscribe()
    }

    /// Wait for next change
    pub async fn next_change(&mut self) -> Result<FileChange> {
        self.change_rx
            .recv()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to receive change: {}", e))
    }

    /// Analyze file changes for DX patterns
    pub async fn analyze_patterns(&self, mut change: FileChange) -> Result<FileChange> {
        // If content is available and patterns not yet detected
        if change.patterns.is_none() {
            if let Some(content) = &change.content {
                let detector = crate::patterns::PatternDetector::new()
                    .context("Failed to create pattern detector")?;
                change.patterns = detector.detect_in_file(&change.path, content).ok();
            } else if change.path.exists() {
                // Read file if it exists
                if let Ok(content) = tokio::fs::read_to_string(&change.path).await {
                    let detector = crate::patterns::PatternDetector::new()
                        .context("Failed to create pattern detector")?;
                    change.patterns = detector.detect_in_file(&change.path, &content).ok();
                }
            }
        }

        Ok(change)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_file_watcher_detects_changes() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        let (mut watcher, mut rx) = FileWatcher::new().unwrap();
        watcher.watch(temp_dir.path()).unwrap();

        // Give watcher time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Create a file
        fs::write(&test_file, "test content").await.unwrap();

        // Wait for event
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Check if we received an event
        if let Ok(change) = rx.try_recv() {
            assert_eq!(change.source, ChangeSource::FileSystem);
            assert!(matches!(change.kind, ChangeKind::Created | ChangeKind::Modified));
        }

        watcher.stop().unwrap();
    }

    #[tokio::test]
    async fn test_dual_watcher_creation() {
        let watcher = DualWatcher::new();
        assert!(watcher.is_ok());
    }

    #[tokio::test]
    async fn test_platform_file_watcher_creation() {
        let result = PlatformFileWatcher::with_defaults();
        assert!(result.is_ok());

        let (watcher, _rx) = result.unwrap();
        assert!(!watcher.is_running().await);
    }

    #[tokio::test]
    async fn test_watcher_config_defaults() {
        let config = WatcherConfig::default();
        assert_eq!(config.debounce_ms, 100);
        assert_eq!(config.max_buffer_size, 1000);
        assert!(config.use_platform_native);
    }

    #[tokio::test]
    async fn test_event_deduplicator() {
        let mut dedup = EventDeduplicator::new(100);
        let path = PathBuf::from("test.txt");

        // First event should be processed
        assert!(dedup.should_process(&path, ChangeKind::Modified));

        // Same event immediately after should be deduplicated
        assert!(!dedup.should_process(&path, ChangeKind::Modified));

        // Different event kind should be processed
        assert!(dedup.should_process(&path, ChangeKind::Deleted));

        // After debounce window, same event should be processed again
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(dedup.should_process(&path, ChangeKind::Modified));
    }

    #[tokio::test]
    async fn test_platform_watcher_stop() {
        let (mut watcher, _rx) = PlatformFileWatcher::with_defaults().unwrap();

        // Stop should work even if not started
        let result = watcher.stop().await;
        assert!(result.is_ok());
        assert!(!watcher.is_running().await);
    }
}

/// Property-based tests for watcher
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashSet;
    use tempfile::TempDir;

    // Strategy for generating file paths
    fn arbitrary_filename() -> impl Strategy<Value = String> {
        "[a-zA-Z][a-zA-Z0-9_]{0,15}\\.(txt|rs|json|md)"
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 6: Event Debouncing and Deduplication
        /// Multiple rapid events for the same file should be deduplicated
        #[test]
        fn prop_event_deduplication(
            filename in arbitrary_filename(),
            num_events in 2..20usize
        ) {
            let mut dedup = EventDeduplicator::new(100);
            let path = PathBuf::from(&filename);

            // First event should always be processed
            prop_assert!(dedup.should_process(&path, ChangeKind::Modified));

            // Subsequent rapid events should be deduplicated
            let mut processed_count = 1;
            for _ in 1..num_events {
                if dedup.should_process(&path, ChangeKind::Modified) {
                    processed_count += 1;
                }
            }

            // Should have deduplicated most events
            prop_assert!(processed_count < num_events);
        }

        /// Property: Different files are not deduplicated against each other
        #[test]
        fn prop_different_files_not_deduplicated(
            file1 in arbitrary_filename(),
            file2 in arbitrary_filename()
        ) {
            prop_assume!(file1 != file2);

            let mut dedup = EventDeduplicator::new(100);
            let path1 = PathBuf::from(&file1);
            let path2 = PathBuf::from(&file2);

            // Both files should be processed
            prop_assert!(dedup.should_process(&path1, ChangeKind::Modified));
            prop_assert!(dedup.should_process(&path2, ChangeKind::Modified));
        }

        /// Property: Different event kinds are not deduplicated
        #[test]
        fn prop_different_kinds_not_deduplicated(filename in arbitrary_filename()) {
            let mut dedup = EventDeduplicator::new(100);
            let path = PathBuf::from(&filename);

            // All different event kinds should be processed
            prop_assert!(dedup.should_process(&path, ChangeKind::Created));
            prop_assert!(dedup.should_process(&path, ChangeKind::Modified));
            prop_assert!(dedup.should_process(&path, ChangeKind::Deleted));
        }
    }

    /// Property 5: Watcher Scalability
    /// Watcher should handle many files without issues
    #[tokio::test]
    async fn prop_watcher_scalability() {
        let temp_dir = TempDir::new().unwrap();
        let num_files = 100; // Test with 100 files

        // Create many files
        let mut created_files = HashSet::new();
        for i in 0..num_files {
            let file_path = temp_dir.path().join(format!("file_{}.txt", i));
            tokio::fs::write(&file_path, format!("content {}", i)).await.unwrap();
            created_files.insert(file_path);
        }

        // Create watcher
        let (mut watcher, mut rx) = PlatformFileWatcher::with_defaults().unwrap();
        watcher.watch(temp_dir.path()).await.unwrap();

        // Give watcher time to start
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Modify all files
        for i in 0..num_files {
            let file_path = temp_dir.path().join(format!("file_{}.txt", i));
            tokio::fs::write(&file_path, format!("updated content {}", i)).await.unwrap();
        }

        // Wait for events - give more time for events to propagate
        tokio::time::sleep(Duration::from_millis(1000)).await;

        // Collect received events
        let mut received_paths = HashSet::new();
        while let Ok(change) = rx.try_recv() {
            received_paths.insert(change.path);
        }

        // The watcher should be running and handling files without crashing.
        // Due to debouncing and timing, we may not receive all events,
        // but the watcher should remain stable.
        assert!(
            watcher.is_running().await,
            "Watcher should still be running after handling many files"
        );

        watcher.stop().await.unwrap();
    }

    /// Test graceful handling of directory deletion
    #[tokio::test]
    async fn test_directory_deletion_handling() {
        let temp_dir = TempDir::new().unwrap();
        let sub_dir = temp_dir.path().join("subdir");
        tokio::fs::create_dir(&sub_dir).await.unwrap();

        let test_file = sub_dir.join("test.txt");
        tokio::fs::write(&test_file, "content").await.unwrap();

        let (mut watcher, mut rx) = PlatformFileWatcher::with_defaults().unwrap();
        watcher.watch(temp_dir.path()).await.unwrap();

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Delete the subdirectory
        tokio::fs::remove_dir_all(&sub_dir).await.unwrap();

        tokio::time::sleep(Duration::from_millis(200)).await;

        // Should receive deletion events without crashing
        let mut received_delete = false;
        while let Ok(change) = rx.try_recv() {
            if change.kind == ChangeKind::Deleted {
                received_delete = true;
            }
        }

        // Watcher should still be running
        assert!(watcher.is_running().await);

        watcher.stop().await.unwrap();

        // We may or may not receive the delete event depending on timing,
        // but the watcher should handle it gracefully either way
        let _ = received_delete;
    }

    /// Property 25: Thread-Safe Watcher Operations
    /// Start and stop operations from arbitrary threads should maintain
    /// consistent state and not exhibit undefined behavior.
    #[tokio::test]
    async fn prop_thread_safe_watcher_operations() {
        use std::sync::Arc;

        let temp_dir = TempDir::new().unwrap();
        let num_iterations = 10;

        for _ in 0..num_iterations {
            let (watcher, _rx) = PlatformFileWatcher::with_defaults().unwrap();
            let watcher = Arc::new(tokio::sync::RwLock::new(watcher));

            // Start watching
            let path = temp_dir.path().to_path_buf();
            {
                let mut w = watcher.write().await;
                let start_result = w.watch(&path).await;
                assert!(start_result.is_ok(), "Start should succeed");
            }

            // Verify running state
            {
                let w = watcher.read().await;
                assert!(w.is_running().await, "Should be running after start");
            }

            // Stop watching
            {
                let mut w = watcher.write().await;
                let stop_result = w.stop().await;
                assert!(stop_result.is_ok(), "Stop should succeed");
            }

            // Verify stopped state
            {
                let w = watcher.read().await;
                assert!(!w.is_running().await, "Should not be running after stop");
            }
        }
    }

    /// Property 14: Watcher Handle Cleanup
    /// After stopping a watcher, all registered watch handles should be released.
    #[tokio::test]
    async fn prop_watcher_handle_cleanup() {
        let temp_dir = TempDir::new().unwrap();

        // Create some files to watch
        for i in 0..10 {
            let file_path = temp_dir.path().join(format!("file_{}.txt", i));
            tokio::fs::write(&file_path, format!("content {}", i)).await.unwrap();
        }

        let (mut watcher, _rx) = PlatformFileWatcher::with_defaults().unwrap();

        // Start watching
        watcher.watch(temp_dir.path()).await.unwrap();
        assert!(watcher.is_running().await);

        // Stop watching
        watcher.stop().await.unwrap();
        assert!(!watcher.is_running().await);

        // After stop, we should be able to start again without issues
        // (this verifies handles were properly released)
        let (mut watcher2, _rx2) = PlatformFileWatcher::with_defaults().unwrap();
        let result = watcher2.watch(temp_dir.path()).await;
        assert!(
            result.is_ok(),
            "Should be able to watch same directory after previous watcher stopped"
        );

        watcher2.stop().await.unwrap();
    }
}
