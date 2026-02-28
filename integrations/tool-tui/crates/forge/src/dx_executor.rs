//! DX Tool Executor
//!
//! Orchestrates execution of all DX tools with warm start caching.
//! Each tool execution:
//! 1. Checks for warm cache (10x faster)
//! 2. Runs the tool
//! 3. Builds cache for next run
//! 4. Syncs to R2 for shared cache

use crate::dx_cache::{DxToolCacheManager, DxToolId, WarmStartResult};
use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool: String,
    pub success: bool,
    pub duration_ms: u64,
    pub warm_start: bool,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub output_files: Vec<PathBuf>,
    pub errors: Vec<String>,
}

/// Tool execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolConfig {
    pub enabled: bool,
    pub parallel: bool,
    pub cache_enabled: bool,
    pub r2_sync: bool,
    pub timeout_ms: u64,
    pub extra: HashMap<String, serde_json::Value>,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            parallel: true,
            cache_enabled: true,
            r2_sync: false,
            timeout_ms: 30_000,
            extra: HashMap::new(),
        }
    }
}

/// DX Tool trait - implement for each tool
pub trait DxToolExecutable: Send + Sync {
    /// Tool identifier
    fn id(&self) -> DxToolId;

    /// Execute the tool
    fn execute(&self, ctx: &ExecutionContext) -> Result<ToolResult>;

    /// Check if tool should run (file changes, dependencies, etc.)
    fn should_run(&self, ctx: &ExecutionContext) -> bool;

    /// Get dependencies (tools that must run first)
    fn dependencies(&self) -> &[DxToolId];

    /// Build cache after execution
    fn build_cache(&self, ctx: &ExecutionContext, result: &ToolResult) -> Result<()>;
}

/// Execution context passed to each tool
pub struct ExecutionContext {
    /// Project root directory
    pub project_root: PathBuf,
    /// Cache manager
    pub cache: Arc<DxToolCacheManager>,
    /// Tool configurations
    pub configs: HashMap<DxToolId, ToolConfig>,
    /// Warm start results
    pub warm_starts: HashMap<DxToolId, WarmStartResult>,
    /// Previous results (for dependencies)
    pub results: Arc<RwLock<HashMap<DxToolId, ToolResult>>>,
}

impl ExecutionContext {
    /// Create new execution context
    pub fn new(project_root: &Path, cache: Arc<DxToolCacheManager>) -> Self {
        Self {
            project_root: project_root.to_path_buf(),
            cache,
            configs: HashMap::new(),
            warm_starts: HashMap::new(),
            results: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if warm start is available for tool
    pub fn has_warm_cache(&self, tool: DxToolId) -> bool {
        self.warm_starts.get(&tool).map(|w| w.ready).unwrap_or(false)
    }

    /// Get warm start result
    pub fn warm_start(&self, tool: DxToolId) -> Option<&WarmStartResult> {
        self.warm_starts.get(&tool)
    }

    /// Get config for tool
    pub fn config(&self, tool: DxToolId) -> ToolConfig {
        self.configs.get(&tool).cloned().unwrap_or_default()
    }
}

/// DX Tool Executor
///
/// Manages execution of all DX tools with caching and R2 sync.
pub struct DxToolExecutor {
    /// Cache manager
    cache: Arc<DxToolCacheManager>,
    /// Registered tools
    tools: HashMap<DxToolId, Arc<dyn DxToolExecutable>>,
    /// Tool configurations
    configs: HashMap<DxToolId, ToolConfig>,
}

impl DxToolExecutor {
    /// Create new executor
    pub fn new(project_root: &Path) -> Result<Self> {
        let cache = Arc::new(DxToolCacheManager::new(project_root)?);
        Ok(Self {
            cache,
            tools: HashMap::new(),
            configs: HashMap::new(),
        })
    }

    /// Register a tool
    pub fn register<T: DxToolExecutable + 'static>(&mut self, tool: T) {
        let id = tool.id();
        self.tools.insert(id, Arc::new(tool));
    }

    /// Configure a tool
    pub fn configure(&mut self, tool: DxToolId, config: ToolConfig) {
        self.configs.insert(tool, config);
    }

    /// Get cache manager
    pub fn cache(&self) -> &Arc<DxToolCacheManager> {
        &self.cache
    }

    /// Initialize warm starts for all tools
    pub fn warm_up(&self) -> Result<HashMap<DxToolId, WarmStartResult>> {
        let mut results = HashMap::new();

        for tool_id in DxToolId::all() {
            match self.cache.warm_start(*tool_id) {
                Ok(result) => {
                    if result.ready {
                        log::info!(
                            "Warm start ready for {}: {} entries, {} bytes in {}ms",
                            result.tool,
                            result.cached_entries,
                            result.total_size,
                            result.load_time_ms
                        );
                    }
                    results.insert(*tool_id, result);
                }
                Err(e) => {
                    log::warn!("Failed to warm start {}: {}", tool_id.folder_name(), e);
                }
            }
        }

        Ok(results)
    }

    /// Execute a single tool
    pub fn execute_tool(&self, tool_id: DxToolId) -> Result<ToolResult> {
        let tool =
            self.tools.get(&tool_id).context(format!("Tool {:?} not registered", tool_id))?;

        // Build execution context
        let warm_starts = self.warm_up()?;
        let mut ctx = ExecutionContext::new(
            self.cache.dx_root().parent().unwrap_or(Path::new(".")),
            self.cache.clone(),
        );
        ctx.configs = self.configs.clone();
        ctx.warm_starts = warm_starts;

        // Check if should run
        if !tool.should_run(&ctx) {
            return Ok(ToolResult {
                tool: tool_id.folder_name().to_string(),
                success: true,
                duration_ms: 0,
                warm_start: true,
                cache_hits: 0,
                cache_misses: 0,
                output_files: vec![],
                errors: vec![],
            });
        }

        let start = Instant::now();

        // Execute tool
        let result = tool.execute(&ctx)?;

        // Build cache after execution
        if result.success {
            if let Err(e) = tool.build_cache(&ctx, &result) {
                log::warn!("Failed to build cache for {:?}: {}", tool_id, e);
            }
        }

        log::info!(
            "Tool {} completed in {}ms (warm: {})",
            tool_id.folder_name(),
            start.elapsed().as_millis(),
            result.warm_start
        );

        Ok(result)
    }

    /// Execute all tools in dependency order
    pub fn execute_all(&self) -> Result<Vec<ToolResult>> {
        let execution_order = self.resolve_dependencies()?;
        let mut results = Vec::new();

        // Build execution context with warm starts
        let warm_starts = self.warm_up()?;
        let mut ctx = ExecutionContext::new(
            self.cache.dx_root().parent().unwrap_or(Path::new(".")),
            self.cache.clone(),
        );
        ctx.configs = self.configs.clone();
        ctx.warm_starts = warm_starts;

        for tool_id in execution_order {
            if let Some(tool) = self.tools.get(&tool_id) {
                let config = ctx.config(tool_id);
                if !config.enabled {
                    continue;
                }

                if !tool.should_run(&ctx) {
                    continue;
                }

                let start = Instant::now();

                match tool.execute(&ctx) {
                    Ok(result) => {
                        // Build cache
                        if result.success && config.cache_enabled {
                            let _ = tool.build_cache(&ctx, &result);
                        }
                        ctx.results.write().insert(tool_id, result.clone());
                        results.push(result);
                    }
                    Err(e) => {
                        let result = ToolResult {
                            tool: tool_id.folder_name().to_string(),
                            success: false,
                            duration_ms: start.elapsed().as_millis() as u64,
                            warm_start: false,
                            cache_hits: 0,
                            cache_misses: 0,
                            output_files: vec![],
                            errors: vec![e.to_string()],
                        };
                        results.push(result);
                    }
                }
            }
        }

        Ok(results)
    }

    /// Resolve dependency order using topological sort
    fn resolve_dependencies(&self) -> Result<Vec<DxToolId>> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut in_progress = std::collections::HashSet::new();

        fn visit(
            tool_id: DxToolId,
            tools: &HashMap<DxToolId, Arc<dyn DxToolExecutable>>,
            visited: &mut std::collections::HashSet<DxToolId>,
            in_progress: &mut std::collections::HashSet<DxToolId>,
            result: &mut Vec<DxToolId>,
        ) -> Result<()> {
            if in_progress.contains(&tool_id) {
                anyhow::bail!("Circular dependency detected for {:?}", tool_id);
            }
            if visited.contains(&tool_id) {
                return Ok(());
            }

            in_progress.insert(tool_id);

            if let Some(tool) = tools.get(&tool_id) {
                for dep in tool.dependencies() {
                    visit(*dep, tools, visited, in_progress, result)?;
                }
            }

            in_progress.remove(&tool_id);
            visited.insert(tool_id);
            result.push(tool_id);

            Ok(())
        }

        for tool_id in self.tools.keys() {
            visit(*tool_id, &self.tools, &mut visited, &mut in_progress, &mut result)?;
        }

        Ok(result)
    }

    /// Sync all caches to R2
    pub async fn sync_to_r2(&self) -> Result<()> {
        for tool_id in DxToolId::all() {
            if self.configs.get(tool_id).map(|c| c.r2_sync).unwrap_or(false) {
                self.cache.sync_to_r2(*tool_id).await?;
            }
        }
        Ok(())
    }
}

// ===== Built-in Tool Implementations =====

/// Bundler tool (dx-js-bundler)
pub struct BundlerTool;

impl DxToolExecutable for BundlerTool {
    fn id(&self) -> DxToolId {
        DxToolId::Bundler
    }

    fn execute(&self, ctx: &ExecutionContext) -> Result<ToolResult> {
        let start = Instant::now();
        let warm = ctx.has_warm_cache(self.id());

        // TODO: Call actual dx-js-bundler
        // For now, simulate execution

        Ok(ToolResult {
            tool: "bundler".to_string(),
            success: true,
            duration_ms: start.elapsed().as_millis() as u64,
            warm_start: warm,
            cache_hits: if warm { 100 } else { 0 },
            cache_misses: if warm { 0 } else { 100 },
            output_files: vec![],
            errors: vec![],
        })
    }

    fn should_run(&self, ctx: &ExecutionContext) -> bool {
        ctx.config(self.id()).enabled
    }

    fn dependencies(&self) -> &[DxToolId] {
        &[DxToolId::NodeModules]
    }

    fn build_cache(&self, ctx: &ExecutionContext, result: &ToolResult) -> Result<()> {
        // Cache bundled outputs
        for output in &result.output_files {
            if output.exists() {
                let content = std::fs::read(output)?;
                ctx.cache.cache_content(self.id(), output, &content)?;
            }
        }
        Ok(())
    }
}

/// Package manager tool (dx-js-package-manager)
pub struct PackageManagerTool;

impl DxToolExecutable for PackageManagerTool {
    fn id(&self) -> DxToolId {
        DxToolId::NodeModules
    }

    fn execute(&self, ctx: &ExecutionContext) -> Result<ToolResult> {
        let start = Instant::now();
        let warm = ctx.has_warm_cache(self.id());

        // TODO: Call actual dx-js-package-manager
        // After fast install, build cache for warm starts

        Ok(ToolResult {
            tool: "package-manager".to_string(),
            success: true,
            duration_ms: start.elapsed().as_millis() as u64,
            warm_start: warm,
            cache_hits: if warm { 500 } else { 0 },
            cache_misses: if warm { 0 } else { 500 },
            output_files: vec![],
            errors: vec![],
        })
    }

    fn should_run(&self, ctx: &ExecutionContext) -> bool {
        // Run if package.json changed or no node_modules
        let package_json = ctx.project_root.join("package.json");
        let node_modules = ctx.project_root.join("node_modules");

        package_json.exists() && (!node_modules.exists() || !ctx.has_warm_cache(self.id()))
    }

    fn dependencies(&self) -> &[DxToolId] {
        &[] // No dependencies - runs first
    }

    fn build_cache(&self, ctx: &ExecutionContext, _result: &ToolResult) -> Result<()> {
        // Cache package metadata for warm starts
        let package_lock = ctx.project_root.join("package-lock.json");
        if package_lock.exists() {
            let content = std::fs::read(&package_lock)?;
            ctx.cache.cache_content(self.id(), &package_lock, &content)?;
        }
        Ok(())
    }
}

/// Style tool (dx-style - Binary CSS)
pub struct StyleTool;

impl DxToolExecutable for StyleTool {
    fn id(&self) -> DxToolId {
        DxToolId::Style
    }

    fn execute(&self, ctx: &ExecutionContext) -> Result<ToolResult> {
        let start = Instant::now();
        let warm = ctx.has_warm_cache(self.id());

        // TODO: Call actual dx-style

        Ok(ToolResult {
            tool: "style".to_string(),
            success: true,
            duration_ms: start.elapsed().as_millis() as u64,
            warm_start: warm,
            cache_hits: 0,
            cache_misses: 0,
            output_files: vec![],
            errors: vec![],
        })
    }

    fn should_run(&self, ctx: &ExecutionContext) -> bool {
        ctx.config(self.id()).enabled
    }

    fn dependencies(&self) -> &[DxToolId] {
        &[]
    }

    fn build_cache(&self, _ctx: &ExecutionContext, _result: &ToolResult) -> Result<()> {
        Ok(())
    }
}

/// Test runner tool (dx-js-test-runner)
pub struct TestRunnerTool;

impl DxToolExecutable for TestRunnerTool {
    fn id(&self) -> DxToolId {
        DxToolId::Test
    }

    fn execute(&self, ctx: &ExecutionContext) -> Result<ToolResult> {
        let start = Instant::now();
        let warm = ctx.has_warm_cache(self.id());

        // TODO: Call actual dx-js-test-runner

        Ok(ToolResult {
            tool: "test".to_string(),
            success: true,
            duration_ms: start.elapsed().as_millis() as u64,
            warm_start: warm,
            cache_hits: 0,
            cache_misses: 0,
            output_files: vec![],
            errors: vec![],
        })
    }

    fn should_run(&self, ctx: &ExecutionContext) -> bool {
        ctx.config(self.id()).enabled
    }

    fn dependencies(&self) -> &[DxToolId] {
        &[DxToolId::NodeModules, DxToolId::Bundler]
    }

    fn build_cache(&self, _ctx: &ExecutionContext, _result: &ToolResult) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_executor_creation() {
        let temp_dir = TempDir::new().unwrap();
        let executor = DxToolExecutor::new(temp_dir.path()).unwrap();
        assert!(executor.cache().dx_root().exists());
    }

    #[test]
    fn test_register_tools() {
        let temp_dir = TempDir::new().unwrap();
        let mut executor = DxToolExecutor::new(temp_dir.path()).unwrap();

        executor.register(BundlerTool);
        executor.register(PackageManagerTool);
        executor.register(StyleTool);
        executor.register(TestRunnerTool);

        // Verify dependency resolution
        let order = executor.resolve_dependencies().unwrap();
        assert!(!order.is_empty());
    }

    #[test]
    fn test_warm_up() {
        let temp_dir = TempDir::new().unwrap();
        let executor = DxToolExecutor::new(temp_dir.path()).unwrap();

        let warm_starts = executor.warm_up().unwrap();
        assert!(!warm_starts.is_empty());
    }
}
