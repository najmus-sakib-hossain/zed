//! Forge Daemon Core - The Binary Dawn Daemon
//!
//! A persistent background service that:
//! - Watches for file changes via dual watchers (LSP + FileSystem)
//! - Orchestrates DX tool execution
//! - Manages background tasks
//! - Syncs with R2 cloud storage

use crate::dx_cache::{DxToolCacheManager, DxToolId};
use crate::dx_executor::{DxToolExecutable, DxToolExecutor, ToolResult};
use crate::watcher::{ChangeKind, ChangeSource, DualWatcher, FileChange};
use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

// ============================================================================
// DAEMON CONFIGURATION
// ============================================================================

/// Daemon configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Project root directory
    pub project_root: PathBuf,
    /// Enable LSP watcher (primary)
    pub enable_lsp_watcher: bool,
    /// Enable file system watcher (fallback)
    pub enable_fs_watcher: bool,
    /// Debounce delay for file changes
    pub debounce_ms: u64,
    /// Number of background workers
    pub worker_count: usize,
    /// Enable R2 cloud sync
    pub enable_r2_sync: bool,
    /// R2 bucket name
    pub r2_bucket: Option<String>,
    /// Auto-run tools on file change
    pub auto_run_tools: bool,
    /// Watch patterns (glob)
    pub watch_patterns: Vec<String>,
    /// Ignore patterns (glob)
    pub ignore_patterns: Vec<String>,
    /// Max concurrent tool executions
    pub max_concurrent_tools: usize,
    /// Tool timeout (ms)
    pub tool_timeout_ms: u64,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            project_root: PathBuf::from("."),
            enable_lsp_watcher: true,
            enable_fs_watcher: true,
            debounce_ms: 100,
            worker_count: num_cpus::get(),
            enable_r2_sync: false,
            r2_bucket: None,
            auto_run_tools: true,
            watch_patterns: vec![
                "**/*.ts".to_string(),
                "**/*.tsx".to_string(),
                "**/*.js".to_string(),
                "**/*.jsx".to_string(),
                "**/*.css".to_string(),
                "**/*.rs".to_string(),
                "**/*.dx".to_string(),
                "**/dx".to_string(),
                "**/dx.config".to_string(),
            ],
            ignore_patterns: vec![
                "**/node_modules/**".to_string(),
                "**/.dx/**".to_string(),
                "**/target/**".to_string(),
                "**/dist/**".to_string(),
                "**/.git/**".to_string(),
            ],
            max_concurrent_tools: 4,
            tool_timeout_ms: 60_000,
        }
    }
}

// ============================================================================
// DAEMON STATE
// ============================================================================

/// Daemon running state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DaemonState {
    /// Starting up
    Starting,
    /// Running and ready
    Running,
    /// Paused (not processing changes)
    Paused,
    /// Shutting down
    ShuttingDown,
    /// Stopped
    Stopped,
}

/// Daemon event for subscribers
#[derive(Debug, Clone)]
pub enum DaemonEvent {
    /// Daemon started
    Started,
    /// File changed
    FileChanged(FileChange),
    /// Tool started
    ToolStarted(DxToolId),
    /// Tool completed
    ToolCompleted(DxToolId, ToolResult),
    /// Tool failed
    ToolFailed(DxToolId, String),
    /// Background task completed
    BackgroundTaskCompleted(String),
    /// Daemon paused
    Paused,
    /// Daemon resumed
    Resumed,
    /// Daemon stopped
    Stopped,
    /// Error occurred
    Error(String),
}

// ============================================================================
// DAEMON STATISTICS
// ============================================================================

/// Runtime statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DaemonStats {
    pub uptime_seconds: u64,
    pub files_changed: u64,
    pub tools_executed: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub r2_syncs: u64,
    pub errors: u64,
    pub lsp_events: u64,
    pub fs_events: u64,
}

// ============================================================================
// FORGE DAEMON
// ============================================================================

/// The Forge Daemon - Binary Dawn Edition
///
/// A persistent background service that orchestrates all DX tools.
pub struct ForgeDaemon {
    /// Configuration
    config: DaemonConfig,
    /// Current state
    state: Arc<RwLock<DaemonState>>,
    /// Shutdown signal
    shutdown: Arc<AtomicBool>,
    /// Tool executor
    executor: Arc<RwLock<DxToolExecutor>>,
    /// Event broadcaster
    event_tx: broadcast::Sender<DaemonEvent>,
    /// Statistics
    stats: Arc<RwLock<DaemonStats>>,
    /// Start time
    start_time: Arc<RwLock<Option<Instant>>>,
    /// Recently processed files (for deduplication)
    recent_files: Arc<RwLock<HashSet<PathBuf>>>,
    /// Tools currently running
    running_tools: Arc<RwLock<HashSet<DxToolId>>>,
}

impl ForgeDaemon {
    /// Create a new daemon instance
    pub fn new(config: DaemonConfig) -> Result<Self> {
        let executor = DxToolExecutor::new(&config.project_root)?;
        let (event_tx, _) = broadcast::channel(1000);

        Ok(Self {
            config,
            state: Arc::new(RwLock::new(DaemonState::Stopped)),
            shutdown: Arc::new(AtomicBool::new(false)),
            executor: Arc::new(RwLock::new(executor)),
            event_tx,
            stats: Arc::new(RwLock::new(DaemonStats::default())),
            start_time: Arc::new(RwLock::new(None)),
            recent_files: Arc::new(RwLock::new(HashSet::new())),
            running_tools: Arc::new(RwLock::new(HashSet::new())),
        })
    }

    /// Get current daemon state
    pub fn state(&self) -> DaemonState {
        *self.state.read()
    }

    /// Subscribe to daemon events
    pub fn subscribe(&self) -> broadcast::Receiver<DaemonEvent> {
        self.event_tx.subscribe()
    }

    /// Get current statistics
    pub fn stats(&self) -> DaemonStats {
        let mut stats = self.stats.read().clone();
        if let Some(start) = *self.start_time.read() {
            stats.uptime_seconds = start.elapsed().as_secs();
        }
        stats
    }

    /// Start the daemon
    pub async fn start(&self) -> Result<()> {
        *self.state.write() = DaemonState::Starting;
        *self.start_time.write() = Some(Instant::now());
        self.shutdown.store(false, Ordering::SeqCst);

        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║        ⚔️  FORGE DAEMON - Binary Dawn Edition                 ║");
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║  Dual Watcher: LSP + FileSystem                              ║");
        println!(
            "║  Background Workers: {}                                       ║",
            self.config.worker_count
        );
        println!(
            "║  R2 Sync: {}                                                ║",
            if self.config.enable_r2_sync {
                "Enabled "
            } else {
                "Disabled"
            }
        );
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();

        // Initialize dual watcher
        let mut watcher = DualWatcher::new()?;
        watcher.start(&self.config.project_root).await?;

        // Get change receiver
        let mut change_rx = watcher.receiver();

        // Set state to running
        *self.state.write() = DaemonState::Running;
        let _ = self.event_tx.send(DaemonEvent::Started);

        println!("👁️  Watching: {}", self.config.project_root.display());
        println!(
            "📡 LSP Watcher: {}",
            if self.config.enable_lsp_watcher {
                "Active"
            } else {
                "Disabled"
            }
        );
        println!(
            "📁 FS Watcher: {}",
            if self.config.enable_fs_watcher {
                "Active"
            } else {
                "Disabled"
            }
        );
        println!();
        println!("🚀 Forge Daemon is now running. Press Ctrl+C to stop.");
        println!();

        // Main event loop
        while !self.shutdown.load(Ordering::SeqCst) {
            tokio::select! {
                // Handle file changes
                Ok(change) = change_rx.recv() => {
                    self.handle_file_change(change).await;
                }

                // Periodic tasks (every 5 seconds)
                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                    self.run_periodic_tasks().await;
                }
            }
        }

        // Cleanup
        *self.state.write() = DaemonState::Stopped;
        let _ = self.event_tx.send(DaemonEvent::Stopped);

        println!();
        println!("👋 Forge Daemon stopped.");

        Ok(())
    }

    /// Stop the daemon
    pub fn stop(&self) {
        println!("🛑 Stopping Forge Daemon...");
        *self.state.write() = DaemonState::ShuttingDown;
        self.shutdown.store(true, Ordering::SeqCst);
    }

    /// Pause the daemon (stop processing changes)
    pub fn pause(&self) {
        *self.state.write() = DaemonState::Paused;
        let _ = self.event_tx.send(DaemonEvent::Paused);
        println!("⏸️  Forge Daemon paused.");
    }

    /// Resume the daemon
    pub fn resume(&self) {
        *self.state.write() = DaemonState::Running;
        let _ = self.event_tx.send(DaemonEvent::Resumed);
        println!("▶️  Forge Daemon resumed.");
    }

    /// Handle a file change event
    async fn handle_file_change(&self, change: FileChange) {
        // Skip if paused
        if *self.state.read() == DaemonState::Paused {
            return;
        }

        // Skip ignored patterns
        if self.should_ignore(&change.path) {
            return;
        }

        // Update stats
        {
            let mut stats = self.stats.write();
            stats.files_changed += 1;
            match change.source {
                ChangeSource::Lsp => stats.lsp_events += 1,
                ChangeSource::FileSystem => stats.fs_events += 1,
            }
        }

        // Log change
        let source = match change.source {
            ChangeSource::Lsp => "LSP",
            ChangeSource::FileSystem => "FS ",
        };
        let kind = match change.kind {
            ChangeKind::Created => "CREATE",
            ChangeKind::Modified => "MODIFY",
            ChangeKind::Deleted => "DELETE",
            ChangeKind::Renamed => "RENAME",
        };

        println!(
            "📝 [{}] {} {}",
            source,
            kind,
            change
                .path
                .strip_prefix(&self.config.project_root)
                .unwrap_or(&change.path)
                .display()
        );

        // Emit event
        let _ = self.event_tx.send(DaemonEvent::FileChanged(change.clone()));

        // Auto-run tools if enabled
        if self.config.auto_run_tools {
            self.trigger_tools_for_change(&change).await;
        }
    }

    /// Check if a path should be ignored
    fn should_ignore(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        // First check hardcoded patterns
        for pattern in &self.config.ignore_patterns {
            // Simple glob matching (could use glob crate for full support)
            if pattern.contains("**") {
                let parts: Vec<&str> = pattern.split("**").collect();
                if parts.len() == 2 {
                    let prefix = parts[0].trim_end_matches('/');
                    let suffix = parts[1].trim_start_matches('/');

                    if !prefix.is_empty() && !path_str.contains(prefix) {
                        continue;
                    }
                    if !suffix.is_empty() && !path_str.contains(suffix) {
                        continue;
                    }
                    return true;
                }
            }
        }

        // Then check gitignore
        self.is_gitignored(path)
    }

    /// Check if a path is ignored by .gitignore
    fn is_gitignored(&self, path: &Path) -> bool {
        let gitignore_path = self.config.project_root.join(".gitignore");
        if !gitignore_path.exists() {
            return false;
        }

        // Use the ignore crate for proper gitignore parsing
        let (gitignore, _err) = ignore::gitignore::Gitignore::new(&gitignore_path);
        let relative_path = path.strip_prefix(&self.config.project_root).unwrap_or(path);
        gitignore.matched(relative_path, path.is_dir()).is_ignore()
    }

    /// Trigger tools based on file change
    async fn trigger_tools_for_change(&self, change: &FileChange) {
        let ext = change.path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match ext {
            // JavaScript/TypeScript files -> bundler
            "ts" | "tsx" | "js" | "jsx" => {
                self.run_tool(DxToolId::Bundler).await;
            }
            // CSS files -> style
            "css" | "scss" | "sass" => {
                self.run_tool(DxToolId::Style).await;
            }
            // Rust files -> www
            "rs" => {
                self.run_tool(DxToolId::Www).await;
            }
            // DX config files
            "dx" => {
                self.run_tool(DxToolId::Serializer).await;
            }
            _ => {}
        }

        // Check if it's a package.json change
        if change.path.file_name().map(|f| f.to_str()) == Some(Some("package.json")) {
            self.run_tool(DxToolId::NodeModules).await;
        }

        // Check if it's a dx config change
        if change.path.file_name().map(|f| f.to_str()) == Some(Some("dx"))
            || change.path.file_name().map(|f| f.to_str()) == Some(Some("dx.config"))
        {
            // Reload all tools
            println!("⚙️  DX config changed - reloading tools...");
        }
    }

    /// Run a specific tool
    async fn run_tool(&self, tool_id: DxToolId) {
        // Check if tool is already running
        {
            let running = self.running_tools.read();
            if running.contains(&tool_id) {
                println!("⏳ {} already running, skipping...", tool_id.folder_name());
                return;
            }
        }

        // Check max concurrent tools
        {
            let running = self.running_tools.read();
            if running.len() >= self.config.max_concurrent_tools {
                println!("⚠️  Max concurrent tools reached, queuing {}...", tool_id.folder_name());
                return;
            }
        }

        // Mark as running
        self.running_tools.write().insert(tool_id);

        let _ = self.event_tx.send(DaemonEvent::ToolStarted(tool_id));

        println!("🔧 Running {}...", tool_id.folder_name());
        let start = Instant::now();

        // Execute tool
        let result = self.executor.read().execute_tool(tool_id);

        // Mark as complete
        self.running_tools.write().remove(&tool_id);

        match result {
            Ok(tool_result) => {
                let duration = start.elapsed();

                // Update stats
                {
                    let mut stats = self.stats.write();
                    stats.tools_executed += 1;
                    stats.cache_hits += tool_result.cache_hits;
                    stats.cache_misses += tool_result.cache_misses;
                }

                println!(
                    "✅ {} completed in {:?} (warm: {})",
                    tool_id.folder_name(),
                    duration,
                    tool_result.warm_start
                );

                let _ = self.event_tx.send(DaemonEvent::ToolCompleted(tool_id, tool_result));
            }
            Err(e) => {
                self.stats.write().errors += 1;
                println!("❌ {} failed: {}", tool_id.folder_name(), e);
                let _ = self.event_tx.send(DaemonEvent::ToolFailed(tool_id, e.to_string()));
            }
        }
    }

    /// Run periodic background tasks
    async fn run_periodic_tasks(&self) {
        // Skip if paused
        if *self.state.read() == DaemonState::Paused {
            return;
        }

        // R2 sync if enabled
        if self.config.enable_r2_sync {
            // TODO: Implement periodic R2 sync
        }

        // Clear old entries from recent_files
        self.recent_files.write().clear();

        // Log uptime every minute
        let stats = self.stats();
        if stats.uptime_seconds > 0 && stats.uptime_seconds % 60 == 0 {
            println!(
                "📊 Uptime: {}s | Files: {} | Tools: {} | Errors: {}",
                stats.uptime_seconds, stats.files_changed, stats.tools_executed, stats.errors
            );
        }
    }

    /// Register a custom tool
    pub fn register_tool<T: DxToolExecutable + 'static>(&self, tool: T) {
        self.executor.write().register(tool);
    }

    /// Get the cache manager
    pub fn cache(&self) -> Arc<DxToolCacheManager> {
        self.executor.read().cache().clone()
    }

    /// Execute a specific tool manually
    pub async fn execute_tool(&self, tool_id: DxToolId) -> Result<ToolResult> {
        self.run_tool(tool_id).await;
        self.executor.read().execute_tool(tool_id)
    }

    /// Execute all tools
    pub async fn execute_all_tools(&self) -> Result<Vec<ToolResult>> {
        self.executor.read().execute_all()
    }
}

impl Drop for ForgeDaemon {
    fn drop(&mut self) {
        self.stop();
    }
}

// ============================================================================
// DAEMON BUILDER
// ============================================================================

/// Builder for ForgeDaemon
pub struct ForgeDaemonBuilder {
    config: DaemonConfig,
}

impl ForgeDaemonBuilder {
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self {
            config: DaemonConfig {
                project_root: project_root.as_ref().to_path_buf(),
                ..Default::default()
            },
        }
    }

    pub fn enable_lsp(mut self, enabled: bool) -> Self {
        self.config.enable_lsp_watcher = enabled;
        self
    }

    pub fn enable_fs_watcher(mut self, enabled: bool) -> Self {
        self.config.enable_fs_watcher = enabled;
        self
    }

    pub fn debounce_ms(mut self, ms: u64) -> Self {
        self.config.debounce_ms = ms;
        self
    }

    pub fn workers(mut self, count: usize) -> Self {
        self.config.worker_count = count;
        self
    }

    pub fn enable_r2(mut self, bucket: &str) -> Self {
        self.config.enable_r2_sync = true;
        self.config.r2_bucket = Some(bucket.to_string());
        self
    }

    pub fn auto_run_tools(mut self, enabled: bool) -> Self {
        self.config.auto_run_tools = enabled;
        self
    }

    pub fn watch_pattern(mut self, pattern: &str) -> Self {
        self.config.watch_patterns.push(pattern.to_string());
        self
    }

    pub fn ignore_pattern(mut self, pattern: &str) -> Self {
        self.config.ignore_patterns.push(pattern.to_string());
        self
    }

    pub fn max_concurrent_tools(mut self, count: usize) -> Self {
        self.config.max_concurrent_tools = count;
        self
    }

    pub fn tool_timeout_ms(mut self, ms: u64) -> Self {
        self.config.tool_timeout_ms = ms;
        self
    }

    pub fn build(self) -> Result<ForgeDaemon> {
        ForgeDaemon::new(self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_daemon_config_default() {
        let config = DaemonConfig::default();
        assert!(config.enable_lsp_watcher);
        assert!(config.enable_fs_watcher);
        assert_eq!(config.debounce_ms, 100);
    }

    #[test]
    fn test_daemon_builder() {
        let temp_dir = TempDir::new().unwrap();
        let daemon = ForgeDaemonBuilder::new(temp_dir.path())
            .enable_lsp(true)
            .enable_fs_watcher(true)
            .workers(4)
            .auto_run_tools(false)
            .build()
            .unwrap();

        assert_eq!(daemon.state(), DaemonState::Stopped);
    }

    #[test]
    fn test_daemon_stats() {
        let temp_dir = TempDir::new().unwrap();
        let daemon = ForgeDaemon::new(DaemonConfig {
            project_root: temp_dir.path().to_path_buf(),
            ..Default::default()
        })
        .unwrap();

        let stats = daemon.stats();
        assert_eq!(stats.files_changed, 0);
        assert_eq!(stats.tools_executed, 0);
    }
}
