//! Worker Pool - Background Task Processing
//!
//! Multi-threaded worker pool for background tasks:
//! - Cache warming
//! - R2 cloud sync
//! - Pattern analysis
//! - Package prefetching
//! - Cleanup operations

use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{Semaphore, mpsc};

use crate::dx_cache::{DxToolCacheManager, DxToolId};

// ============================================================================
// TASK PRIORITY
// ============================================================================

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum TaskPriority {
    /// Critical - run immediately
    Critical = 0,
    /// High - run soon
    High = 1,
    /// Normal - default priority
    #[default]
    Normal = 2,
    /// Low - run when idle
    Low = 3,
    /// Background - run in spare cycles
    Background = 4,
}

// ============================================================================
// WORKER TASKS
// ============================================================================

/// Background worker tasks
#[derive(Debug, Clone)]
pub enum WorkerTask {
    /// Warm up cache for a tool
    WarmCache { tool: String },
    /// Sync to R2 cloud
    SyncToR2 { tool: String, paths: Vec<String> },
    /// Pull from R2 cloud
    PullFromR2 { tool: String },
    /// Analyze codebase patterns
    AnalyzePatterns { paths: Vec<String> },
    /// Prefetch a package
    PrefetchPackage { name: String, version: String },
    /// Clean old cache entries
    CleanCache { tool: String, max_age_days: u32 },
    /// Clean all caches
    CleanAllCaches,
    /// Build tool cache
    BuildCache {
        tool: String,
        output_paths: Vec<String>,
    },
    /// Index project files
    IndexProject { root: String },
    /// Custom task
    Custom {
        name: String,
        data: serde_json::Value,
    },
}

impl WorkerTask {
    /// Get task name
    pub fn name(&self) -> &str {
        match self {
            WorkerTask::WarmCache { .. } => "WarmCache",
            WorkerTask::SyncToR2 { .. } => "SyncToR2",
            WorkerTask::PullFromR2 { .. } => "PullFromR2",
            WorkerTask::AnalyzePatterns { .. } => "AnalyzePatterns",
            WorkerTask::PrefetchPackage { .. } => "PrefetchPackage",
            WorkerTask::CleanCache { .. } => "CleanCache",
            WorkerTask::CleanAllCaches => "CleanAllCaches",
            WorkerTask::BuildCache { .. } => "BuildCache",
            WorkerTask::IndexProject { .. } => "IndexProject",
            WorkerTask::Custom { name, .. } => name,
        }
    }
}

/// Prioritized task wrapper
#[derive(Debug, Clone)]
pub struct PrioritizedTask {
    pub task: WorkerTask,
    pub priority: TaskPriority,
    pub created_at: u64,
}

impl PrioritizedTask {
    pub fn new(task: WorkerTask, priority: TaskPriority) -> Self {
        Self {
            task,
            priority,
            // SystemTime::now() is always after UNIX_EPOCH on any reasonable system,
            // but we use unwrap_or(0) for safety in edge cases
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }
    }
}

// ============================================================================
// WORKER POOL
// ============================================================================

/// Worker pool statistics
#[derive(Debug, Clone, Default)]
pub struct WorkerPoolStats {
    pub workers: usize,
    pub tasks_queued: u64,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub busy_workers: usize,
}

/// Worker pool for background task processing
pub struct WorkerPool {
    /// Number of workers
    worker_count: usize,
    /// Task sender
    task_tx: mpsc::Sender<PrioritizedTask>,
    /// Running flag
    running: Arc<AtomicBool>,
    /// Statistics
    tasks_completed: Arc<AtomicU64>,
    tasks_failed: Arc<AtomicU64>,
    tasks_queued: Arc<AtomicU64>,
    busy_workers: Arc<AtomicU64>,
}

impl WorkerPool {
    /// Create a new worker pool
    pub fn new(worker_count: usize) -> Self {
        let (task_tx, mut task_rx) = mpsc::channel::<PrioritizedTask>(1000);
        let running = Arc::new(AtomicBool::new(true));
        let tasks_completed = Arc::new(AtomicU64::new(0));
        let tasks_failed = Arc::new(AtomicU64::new(0));
        let tasks_queued = Arc::new(AtomicU64::new(0));
        let busy_workers = Arc::new(AtomicU64::new(0));
        let semaphore = Arc::new(Semaphore::new(worker_count));

        // Spawn worker coordinator
        let running_clone = running.clone();
        let completed_clone = tasks_completed.clone();
        let failed_clone = tasks_failed.clone();
        let queued_clone = tasks_queued.clone();
        let busy_clone = busy_workers.clone();
        let sem_clone = semaphore.clone();

        tokio::spawn(async move {
            println!("👷 Worker Pool started with {} workers", worker_count);

            while running_clone.load(Ordering::SeqCst) {
                match task_rx.recv().await {
                    Some(prioritized_task) => {
                        queued_clone.fetch_sub(1, Ordering::SeqCst);

                        // Acquire semaphore permit
                        let permit = sem_clone.clone().acquire_owned().await;
                        if permit.is_err() {
                            continue;
                        }

                        busy_clone.fetch_add(1, Ordering::SeqCst);

                        let task = prioritized_task.task;
                        let completed = completed_clone.clone();
                        let failed = failed_clone.clone();
                        let busy = busy_clone.clone();

                        tokio::spawn(async move {
                            let task_name = task.name().to_string();
                            let start = Instant::now();

                            match Self::execute_task(task).await {
                                Ok(()) => {
                                    completed.fetch_add(1, Ordering::SeqCst);
                                    println!(
                                        "✅ [BG] {} completed in {:?}",
                                        task_name,
                                        start.elapsed()
                                    );
                                }
                                Err(e) => {
                                    failed.fetch_add(1, Ordering::SeqCst);
                                    eprintln!("❌ [BG] {} failed: {}", task_name, e);
                                }
                            }

                            busy.fetch_sub(1, Ordering::SeqCst);
                            drop(permit);
                        });
                    }
                    None => break,
                }
            }

            println!("👷 Worker Pool stopped");
        });

        Self {
            worker_count,
            task_tx,
            running,
            tasks_completed,
            tasks_failed,
            tasks_queued,
            busy_workers,
        }
    }

    /// Execute a task
    async fn execute_task(task: WorkerTask) -> Result<()> {
        match task {
            WorkerTask::WarmCache { tool } => {
                Self::execute_warm_cache(&tool).await?;
            }
            WorkerTask::SyncToR2 { tool, paths } => {
                Self::execute_sync_to_r2(&tool, &paths).await?;
            }
            WorkerTask::PullFromR2 { tool } => {
                Self::execute_pull_from_r2(&tool).await?;
            }
            WorkerTask::AnalyzePatterns { paths } => {
                Self::execute_analyze_patterns(&paths).await?;
            }
            WorkerTask::PrefetchPackage { name, version } => {
                Self::execute_prefetch_package(&name, &version).await?;
            }
            WorkerTask::CleanCache { tool, max_age_days } => {
                Self::execute_clean_cache(&tool, max_age_days).await?;
            }
            WorkerTask::CleanAllCaches => {
                Self::execute_clean_all_caches().await?;
            }
            WorkerTask::BuildCache { tool, output_paths } => {
                Self::execute_build_cache(&tool, &output_paths).await?;
            }
            WorkerTask::IndexProject { root } => {
                Self::execute_index_project(&root).await?;
            }
            WorkerTask::Custom { name, data } => {
                tracing::info!("⚙️  [BG] Running custom task: {} with {:?}", name, data);
                // Custom tasks are user-defined, just log them
            }
        }

        Ok(())
    }

    /// Warm up cache for a tool - loads cache index and preloads hot entries
    async fn execute_warm_cache(tool: &str) -> Result<()> {
        tracing::info!("🔥 [BG] Warming cache for {}...", tool);

        // Get the current working directory as project root
        let project_root = std::env::current_dir().context("Failed to get current directory")?;

        let cache_manager =
            DxToolCacheManager::new(&project_root).context("Failed to create cache manager")?;

        // Parse tool ID
        let tool_id = Self::parse_tool_id(tool)?;

        // Perform warm start
        let result = cache_manager.warm_start(tool_id).context("Failed to warm cache")?;

        tracing::info!(
            "🔥 [BG] Cache warmed for {}: {} entries, {} bytes, ready={}",
            tool,
            result.cached_entries,
            result.total_size,
            result.ready
        );

        Ok(())
    }

    /// Sync files to R2 cloud storage
    async fn execute_sync_to_r2(tool: &str, _paths: &[String]) -> Result<()> {
        tracing::info!("☁️  [BG] Syncing to R2 for {}...", tool);

        let project_root = std::env::current_dir().context("Failed to get current directory")?;

        let cache_manager =
            DxToolCacheManager::new(&project_root).context("Failed to create cache manager")?;

        if !cache_manager.is_r2_configured() {
            tracing::warn!("☁️  [BG] R2 not configured, skipping sync");
            return Ok(());
        }

        let tool_id = Self::parse_tool_id(tool)?;
        let result = cache_manager.sync_to_r2(tool_id).await.context("Failed to sync to R2")?;

        tracing::info!(
            "☁️  [BG] R2 sync complete for {}: {} uploaded, {} skipped, {} failed",
            tool,
            result.uploaded,
            result.skipped,
            result.failed
        );

        Ok(())
    }

    /// Pull cache from R2 cloud storage
    async fn execute_pull_from_r2(tool: &str) -> Result<()> {
        tracing::info!("⬇️  [BG] Pulling cache from R2 for {}...", tool);

        let project_root = std::env::current_dir().context("Failed to get current directory")?;

        let cache_manager =
            DxToolCacheManager::new(&project_root).context("Failed to create cache manager")?;

        if !cache_manager.is_r2_configured() {
            tracing::warn!("⬇️  [BG] R2 not configured, skipping pull");
            return Ok(());
        }

        let tool_id = Self::parse_tool_id(tool)?;
        let result = cache_manager.pull_from_r2(tool_id).await.context("Failed to pull from R2")?;

        tracing::info!(
            "⬇️  [BG] R2 pull complete for {}: {} downloaded, {} skipped, {} failed",
            tool,
            result.uploaded,
            result.skipped,
            result.failed
        );

        Ok(())
    }

    /// Analyze codebase for DX patterns (dxButton, dxiIcon, etc.)
    async fn execute_analyze_patterns(paths: &[String]) -> Result<()> {
        tracing::info!("🔍 [BG] Analyzing patterns in {} files...", paths.len());

        let dx_patterns = [
            "dxButton",
            "dxInput",
            "dxSelect",
            "dxModal",
            "dxTable",
            "dxiIcon",
            "dxiLogo",
            "dxiSpinner",
            "dx-",
            "dxi-",
        ];

        let mut total_matches = 0;
        let mut files_with_patterns = 0;

        for path_str in paths {
            let path = Path::new(path_str);
            if !path.exists() || !path.is_file() {
                continue;
            }

            // Only analyze source files
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !["ts", "tsx", "js", "jsx", "vue", "svelte", "html"].contains(&ext) {
                continue;
            }

            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let mut file_matches = 0;
            for pattern in &dx_patterns {
                file_matches += content.matches(pattern).count();
            }

            if file_matches > 0 {
                files_with_patterns += 1;
                total_matches += file_matches;
            }
        }

        tracing::info!(
            "🔍 [BG] Pattern analysis complete: {} matches in {} files",
            total_matches,
            files_with_patterns
        );

        Ok(())
    }

    /// Prefetch a package to local cache
    async fn execute_prefetch_package(name: &str, version: &str) -> Result<()> {
        tracing::info!("📦 [BG] Prefetching {}@{}...", name, version);

        // Check if package registry is configured
        let registry_url = std::env::var("DX_PACKAGE_REGISTRY")
            .unwrap_or_else(|_| "https://registry.dx.dev".to_string());

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        let url = format!("{}/packages/{}/{}", registry_url, name, version);

        // Try to fetch package metadata
        let response = client.head(&url).send().await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                tracing::info!("📦 [BG] Package {}@{} available for prefetch", name, version);
                // In a full implementation, we would download and cache the package here
            }
            Ok(resp) => {
                tracing::warn!(
                    "📦 [BG] Package {}@{} not found (status: {})",
                    name,
                    version,
                    resp.status()
                );
            }
            Err(e) => {
                tracing::warn!("📦 [BG] Failed to check package {}@{}: {}", name, version, e);
            }
        }

        Ok(())
    }

    /// Clean old cache entries for a tool
    async fn execute_clean_cache(tool: &str, max_age_days: u32) -> Result<()> {
        tracing::info!("🧹 [BG] Cleaning cache for {} (max age: {} days)...", tool, max_age_days);

        let project_root = std::env::current_dir().context("Failed to get current directory")?;

        let cache_manager =
            DxToolCacheManager::new(&project_root).context("Failed to create cache manager")?;

        let tool_id = Self::parse_tool_id(tool)?;
        let tool_dir = cache_manager
            .tool_dir(tool_id)
            .ok_or_else(|| anyhow::anyhow!("Tool directory not found"))?;

        let max_age_secs = max_age_days as u64 * 24 * 60 * 60;
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut removed = 0;
        let mut kept = 0;

        // Walk the tool directory and remove old files
        if let Ok(entries) = std::fs::read_dir(tool_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Check subdirectory entries
                    if let Ok(sub_entries) = std::fs::read_dir(&path) {
                        for sub_entry in sub_entries.flatten() {
                            let sub_path = sub_entry.path();
                            if let Ok(metadata) = sub_path.metadata() {
                                let modified = metadata
                                    .modified()
                                    .ok()
                                    .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                                    .map(|d| d.as_secs())
                                    .unwrap_or(0);

                                if now - modified > max_age_secs {
                                    if std::fs::remove_file(&sub_path).is_ok() {
                                        removed += 1;
                                    }
                                } else {
                                    kept += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        tracing::info!("🧹 [BG] Cache cleanup for {}: {} removed, {} kept", tool, removed, kept);

        Ok(())
    }

    /// Clean all caches
    async fn execute_clean_all_caches() -> Result<()> {
        tracing::info!("🧹 [BG] Cleaning all caches...");

        let project_root = std::env::current_dir().context("Failed to get current directory")?;

        let cache_manager =
            DxToolCacheManager::new(&project_root).context("Failed to create cache manager")?;

        cache_manager.clear_all().context("Failed to clear all caches")?;

        tracing::info!("🧹 [BG] All caches cleared");

        Ok(())
    }

    /// Build cache for tool outputs
    async fn execute_build_cache(tool: &str, output_paths: &[String]) -> Result<()> {
        tracing::info!("📦 [BG] Building cache for {} ({} files)...", tool, output_paths.len());

        let project_root = std::env::current_dir().context("Failed to get current directory")?;

        let cache_manager =
            DxToolCacheManager::new(&project_root).context("Failed to create cache manager")?;

        let tool_id = Self::parse_tool_id(tool)?;

        let mut cached = 0;
        let mut failed = 0;

        for path_str in output_paths {
            let path = Path::new(path_str);
            if !path.exists() || !path.is_file() {
                continue;
            }

            match std::fs::read(path) {
                Ok(content) => match cache_manager.cache_content(tool_id, path, &content) {
                    Ok(_) => cached += 1,
                    Err(e) => {
                        tracing::warn!("Failed to cache {}: {}", path_str, e);
                        failed += 1;
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to read {}: {}", path_str, e);
                    failed += 1;
                }
            }
        }

        tracing::info!(
            "📦 [BG] Build cache complete for {}: {} cached, {} failed",
            tool,
            cached,
            failed
        );

        Ok(())
    }

    /// Index project files
    async fn execute_index_project(root: &str) -> Result<()> {
        tracing::info!("📁 [BG] Indexing project at {}...", root);

        let root_path = Path::new(root);
        if !root_path.exists() {
            return Err(anyhow::anyhow!("Project root does not exist: {}", root));
        }

        let mut file_count = 0;
        let mut dir_count = 0;
        let mut total_size: u64 = 0;

        // Walk directory tree
        fn walk_dir(
            path: &Path,
            file_count: &mut usize,
            dir_count: &mut usize,
            total_size: &mut u64,
        ) {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();

                    // Skip hidden files and common ignore patterns
                    if let Some(name) = entry_path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with('.') || name == "node_modules" || name == "target" {
                            continue;
                        }
                    }

                    if entry_path.is_dir() {
                        *dir_count += 1;
                        walk_dir(&entry_path, file_count, dir_count, total_size);
                    } else if entry_path.is_file() {
                        *file_count += 1;
                        if let Ok(metadata) = entry_path.metadata() {
                            *total_size += metadata.len();
                        }
                    }
                }
            }
        }

        walk_dir(root_path, &mut file_count, &mut dir_count, &mut total_size);

        tracing::info!(
            "📁 [BG] Project indexed: {} files, {} directories, {} bytes",
            file_count,
            dir_count,
            total_size
        );

        Ok(())
    }

    /// Parse tool name to DxToolId
    fn parse_tool_id(tool: &str) -> Result<DxToolId> {
        match tool.to_lowercase().as_str() {
            "cache" => Ok(DxToolId::Cache),
            "forge" => Ok(DxToolId::Forge),
            "bundler" => Ok(DxToolId::Bundler),
            "node_modules" | "nodemodules" => Ok(DxToolId::NodeModules),
            "test" => Ok(DxToolId::Test),
            "style" => Ok(DxToolId::Style),
            "icon" => Ok(DxToolId::Icon),
            "font" => Ok(DxToolId::Font),
            "media" => Ok(DxToolId::Media),
            "i18n" => Ok(DxToolId::I18n),
            "ui" => Ok(DxToolId::Ui),
            "serializer" => Ok(DxToolId::Serializer),
            "generator" => Ok(DxToolId::Generator),
            "driven" => Ok(DxToolId::Driven),
            "workspace" => Ok(DxToolId::Workspace),
            "www" => Ok(DxToolId::Www),
            _ => Err(anyhow::anyhow!("Unknown tool: {}", tool)),
        }
    }

    /// Queue a task
    pub async fn queue(&self, task: WorkerTask) {
        self.queue_with_priority(task, TaskPriority::Normal).await;
    }

    /// Queue a task with priority
    pub async fn queue_with_priority(&self, task: WorkerTask, priority: TaskPriority) {
        let prioritized = PrioritizedTask::new(task, priority);
        self.tasks_queued.fetch_add(1, Ordering::SeqCst);
        let _ = self.task_tx.send(prioritized).await;
    }

    /// Queue multiple tasks
    pub async fn queue_many(&self, tasks: Vec<WorkerTask>) {
        for task in tasks {
            self.queue(task).await;
        }
    }

    /// Get statistics
    pub fn stats(&self) -> WorkerPoolStats {
        WorkerPoolStats {
            workers: self.worker_count,
            tasks_queued: self.tasks_queued.load(Ordering::SeqCst),
            tasks_completed: self.tasks_completed.load(Ordering::SeqCst),
            tasks_failed: self.tasks_failed.load(Ordering::SeqCst),
            busy_workers: self.busy_workers.load(Ordering::SeqCst) as usize,
        }
    }

    /// Stop the worker pool
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Check if pool is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Get worker count
    pub fn worker_count(&self) -> usize {
        self.worker_count
    }

    /// Get busy worker count
    pub fn busy_workers(&self) -> usize {
        self.busy_workers.load(Ordering::SeqCst) as usize
    }

    /// Wait for all tasks to complete
    pub async fn wait_for_completion(&self) {
        while self.tasks_queued.load(Ordering::SeqCst) > 0
            || self.busy_workers.load(Ordering::SeqCst) > 0
        {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }
}

impl Default for WorkerPool {
    fn default() -> Self {
        Self::new(num_cpus::get())
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_priority_order() {
        assert!(TaskPriority::Critical < TaskPriority::High);
        assert!(TaskPriority::High < TaskPriority::Normal);
        assert!(TaskPriority::Normal < TaskPriority::Low);
        assert!(TaskPriority::Low < TaskPriority::Background);
    }

    #[test]
    fn test_worker_task_name() {
        assert_eq!(
            WorkerTask::WarmCache {
                tool: "bundler".to_string()
            }
            .name(),
            "WarmCache"
        );
        assert_eq!(WorkerTask::CleanAllCaches.name(), "CleanAllCaches");
    }

    #[tokio::test]
    async fn test_worker_pool_creation() {
        let pool = WorkerPool::new(2);
        assert_eq!(pool.worker_count(), 2);
        assert!(pool.is_running());

        pool.stop();
    }

    #[tokio::test]
    async fn test_worker_pool_queue() {
        let pool = WorkerPool::new(2);

        pool.queue(WorkerTask::WarmCache {
            tool: "test".to_string(),
        })
        .await;

        // Wait a bit for task to process
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let stats = pool.stats();
        assert!(stats.tasks_completed >= 1 || stats.tasks_queued > 0);

        pool.stop();
    }
}
