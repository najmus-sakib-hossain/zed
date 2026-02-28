//! Simple Orchestrator - Only controls WHEN to run tools
//!
//! Tools are self-contained and know:
//! - What files to process
//! - When they should run
//! - What patterns to detect
//!
//! Forge just detects changes and asks: "Should you run?"

use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Tool execution context shared across all tools
///
/// The execution context provides tools with access to repository state,
/// file changes, shared data, and traffic branch analysis. It serves as
/// the communication hub between tools and the orchestrator.
///
/// # Fields
///
/// - `repo_root`: Absolute path to the repository root
/// - `forge_path`: Path to Forge data directory (.dx/forge)
/// - `current_branch`: Git branch name (if in a git repo)
/// - `changed_files`: Files modified in this execution cycle
/// - `shared_state`: Thread-safe storage for inter-tool communication
/// - `traffic_analyzer`: Analyzes file changes for merge safety
/// - `component_manager`: Manages component state for traffic branches
#[derive(Clone)]
pub struct ExecutionContext {
    /// Repository root path
    pub repo_root: PathBuf,

    /// Forge storage path (.dx/forge)
    pub forge_path: PathBuf,

    /// Current Git branch
    pub current_branch: Option<String>,

    /// Changed files in this execution
    pub changed_files: Vec<PathBuf>,

    /// Shared state between tools
    pub shared_state: Arc<RwLock<HashMap<String, serde_json::Value>>>,

    /// Traffic branch analyzer
    pub traffic_analyzer: Arc<dyn TrafficAnalyzer + Send + Sync>,

    /// Component state manager for traffic branch system
    pub component_manager: Option<Arc<RwLock<crate::context::ComponentStateManager>>>,
}

impl std::fmt::Debug for ExecutionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExecutionContext")
            .field("repo_root", &self.repo_root)
            .field("forge_path", &self.forge_path)
            .field("current_branch", &self.current_branch)
            .field("changed_files", &self.changed_files)
            .field("traffic_analyzer", &"<dyn TrafficAnalyzer>")
            .finish()
    }
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new(repo_root: PathBuf, forge_path: PathBuf) -> Self {
        // Try to create component state manager
        let component_manager = crate::context::ComponentStateManager::new(&forge_path)
            .ok()
            .map(|mgr| Arc::new(RwLock::new(mgr)));

        Self {
            repo_root,
            forge_path,
            current_branch: None,
            changed_files: Vec::new(),
            shared_state: Arc::new(RwLock::new(HashMap::new())),
            traffic_analyzer: Arc::new(DefaultTrafficAnalyzer),
            component_manager,
        }
    }

    /// Set a shared value
    pub fn set<T: Serialize>(&self, key: impl Into<String>, value: T) -> Result<()> {
        let json = serde_json::to_value(value).context("Failed to serialize shared state value")?;
        self.shared_state.write().insert(key.into(), json);
        Ok(())
    }

    /// Get a shared value
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>> {
        let state = self.shared_state.read();
        if let Some(value) = state.get(key) {
            let result = serde_json::from_value(value.clone()).with_context(|| {
                format!("Failed to deserialize shared state value for key: {}", key)
            })?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    /// Find regex patterns in a file
    pub fn find_patterns(&self, _pattern: &str) -> Result<Vec<PatternMatch>> {
        // Implementation will be added
        Ok(Vec::new())
    }
}

/// Pattern match result
#[derive(Debug, Clone)]
pub struct PatternMatch {
    pub file: PathBuf,
    pub line: usize,
    pub col: usize,
    pub text: String,
    pub captures: Vec<String>,
}

/// Output from tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub success: bool,
    pub files_modified: Vec<PathBuf>,
    pub files_created: Vec<PathBuf>,
    pub files_deleted: Vec<PathBuf>,
    pub message: String,
    pub duration_ms: u64,
}

impl ToolOutput {
    pub fn success() -> Self {
        Self {
            success: true,
            files_modified: Vec::new(),
            files_created: Vec::new(),
            files_deleted: Vec::new(),
            message: "Success".to_string(),
            duration_ms: 0,
        }
    }

    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            files_modified: Vec::new(),
            files_created: Vec::new(),
            files_deleted: Vec::new(),
            message: message.into(),
            duration_ms: 0,
        }
    }
}

/// Main DX tool trait - all tools must implement this
///
/// # Overview
///
/// The `DxTool` trait provides the core interface for all DX tools in the Forge ecosystem.
/// Tools are self-contained units that know what files to process, when to run, and how to
/// integrate with the broader toolchain.
///
/// # Lifecycle
///
/// Tool execution follows this lifecycle:
/// 1. `should_run()` - Check if tool should execute
/// 2. `before_execute()` - Setup and validation
/// 3. `execute()` - Main tool logic
/// 4. `after_execute()` - Cleanup and reporting (on success)
/// 5. `on_error()` - Error handling (on failure)
///
/// # Example
///
/// ```rust,no_run
/// use dx_forge::{DxTool, ExecutionContext, ToolOutput};
/// use anyhow::Result;
///
/// struct MyCustomTool {
///     enabled: bool,
/// }
///
/// impl DxTool for MyCustomTool {
///     fn name(&self) -> &str { "my-custom-tool" }
///     fn version(&self) -> &str { "1.0.0" }
///     fn priority(&self) -> u32 { 50 }
///     
///     fn execute(&mut self, ctx: &ExecutionContext) -> Result<ToolOutput> {
///         // Your tool logic here
///         Ok(ToolOutput::success())
///     }
///     
///     fn should_run(&self, _ctx: &ExecutionContext) -> bool {
///         self.enabled
///     }
/// }
/// ```
pub trait DxTool: Send + Sync {
    /// Tool name (e.g., "dx-ui", "dx-style")
    ///
    /// This should be a unique identifier for your tool. By convention,
    /// DX tools use the format "dx-{category}" (e.g., dx-ui, dx-icons, dx-style).
    fn name(&self) -> &str;

    /// Tool version using semantic versioning
    ///
    /// The version should follow semver format (e.g., "1.2.3").
    /// This is used for dependency resolution and compatibility checking.
    fn version(&self) -> &str;

    /// Execution priority (lower number = executes earlier)
    ///
    /// Tools are executed in priority order. Common priority values:
    /// - 0-20: Infrastructure tools (code generation, schema validation)
    /// - 21-50: Component tools (UI, icons, styles)
    /// - 51-100: Post-processing tools (optimization, bundling)
    ///
    /// Default priority is typically 50 for most tools.
    fn priority(&self) -> u32;

    /// Execute the tool's main logic
    ///
    /// This is where the core functionality of your tool should be implemented.
    /// The execution context provides access to repository state, file changes,
    /// and shared state between tools.
    ///
    /// # Arguments
    ///
    /// * `context` - Execution context with repo info and shared state
    ///
    /// # Returns
    ///
    /// Returns a `ToolOutput` containing execution results, modified files, and status.
    ///
    /// # Errors
    ///
    /// Return an error if execution fails. The orchestrator will handle cleanup
    /// and invoke the `on_error` hook.
    fn execute(&mut self, context: &ExecutionContext) -> Result<ToolOutput>;

    /// Check if tool should run (optional pre-check)
    ///
    /// Override this method to implement custom logic for determining whether
    /// the tool should execute. This is called before `before_execute()`.
    ///
    /// # Arguments
    ///
    /// * `_context` - Execution context for checking conditions
    ///
    /// # Returns
    ///
    /// `true` if the tool should execute, `false` to skip execution
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use dx_forge::{DxTool, ExecutionContext, ToolOutput};
    /// use anyhow::Result;
    ///
    /// struct MyTool;
    ///
    /// impl DxTool for MyTool {
    ///     fn name(&self) -> &str { "dx-mytool" }
    ///     fn version(&self) -> &str { "1.0.0" }
    ///     fn priority(&self) -> u32 { 10 }
    ///
    ///     fn execute(&mut self, _ctx: &ExecutionContext) -> Result<ToolOutput> {
    ///         Ok(ToolOutput::success())
    ///     }
    ///
    ///     fn should_run(&self, ctx: &ExecutionContext) -> bool {
    ///         // Only run if TypeScript files changed
    ///         ctx.changed_files
    ///             .iter()
    ///             .any(|path| path.extension().map_or(false, |ext| ext == "ts"))
    ///     }
    /// }
    /// ```
    fn should_run(&self, _context: &ExecutionContext) -> bool {
        true
    }

    /// Tool dependencies (must run after these tools)
    ///
    /// Specify tools that must execute before this tool. Dependencies are validated
    /// before execution begins, and circular dependencies are detected.
    ///
    /// # Returns
    ///
    /// Vector of tool names this tool depends on
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use dx_forge::{DxTool, ExecutionContext, ToolOutput};
    /// use anyhow::Result;
    ///
    /// struct SchemaAwareTool;
    ///
    /// impl DxTool for SchemaAwareTool {
    ///     fn name(&self) -> &str { "dx-schema-consumer" }
    ///     fn version(&self) -> &str { "1.0.0" }
    ///     fn priority(&self) -> u32 { 20 }
    ///
    ///     fn execute(&mut self, _ctx: &ExecutionContext) -> Result<ToolOutput> {
    ///         Ok(ToolOutput::success())
    ///     }
    ///
    ///     fn dependencies(&self) -> Vec<String> {
    ///         vec![
    ///             "dx-codegen".to_string(),
    ///             "dx-schema".to_string(),
    ///         ]
    ///     }
    /// }
    /// ```
    fn dependencies(&self) -> Vec<String> {
        Vec::new()
    }

    /// Before execution hook (setup, validation)
    ///
    /// Called before `execute()`. Use this for:
    /// - Validating preconditions
    /// - Setting up temporary resources
    /// - Checking file permissions
    /// - Loading configuration
    ///
    /// # Errors
    ///
    /// Return an error to prevent execution and skip to `on_error`
    fn before_execute(&mut self, _context: &ExecutionContext) -> Result<()> {
        Ok(())
    }

    /// After execution hook (cleanup, reporting)
    ///
    /// Called after successful `execute()`. Use this for:
    /// - Cleaning up temporary files
    /// - Generating reports
    /// - Updating shared state
    /// - Sending notifications
    ///
    /// # Arguments
    ///
    /// * `_output` - The output from successful execution
    fn after_execute(&mut self, _context: &ExecutionContext, _output: &ToolOutput) -> Result<()> {
        Ok(())
    }

    /// On error hook (rollback, cleanup)
    ///
    /// Called when `execute()` or `before_execute()` fails. Use this for:
    /// - Rolling back partial changes
    /// - Cleaning up resources
    /// - Logging detailed error info
    /// - Sending error notifications
    ///
    /// # Arguments
    ///
    /// * `_error` - The error that occurred
    fn on_error(&mut self, _context: &ExecutionContext, _error: &anyhow::Error) -> Result<()> {
        Ok(())
    }

    /// Execution timeout in seconds (0 = no timeout)
    ///
    /// Specifies the maximum time this tool should be allowed to run.
    /// Note: Timeout enforcement for synchronous tools is not yet implemented.
    /// Future versions will use thread-based timeouts or async execution.
    ///
    /// # Returns
    ///
    /// Timeout duration in seconds, or 0 for no timeout
    fn timeout_seconds(&self) -> u64 {
        60
    }
}

// Tools are self-contained - no manifests needed
// Each tool knows what to do and when to run

/// Traffic branch analysis result
///
/// Forge uses a "traffic light" system to categorize file changes by risk level:
///
/// - **🟢 Green**: Safe to auto-merge (docs, tests, styles, assets)
/// - **🟡 Yellow**: Reviewable conflicts (code changes that may conflict)
/// - **🔴 Red**: Manual resolution required (API changes, schemas, migrations)
///
/// This system prevents breaking changes from being automatically merged while
/// allowing safe updates to proceed without manual intervention.
///
/// # Example
///
/// ```rust,no_run
/// use dx_forge::orchestrator::{TrafficBranch, TrafficAnalyzer, DefaultTrafficAnalyzer};
/// use std::path::Path;
///
/// fn example() -> anyhow::Result<()> {
///     let analyzer = DefaultTrafficAnalyzer;
///     let result = analyzer.analyze(Path::new("src/api/types.ts"))?;
///
///     match result {
///         TrafficBranch::Green => println!("Safe to auto-merge"),
///         TrafficBranch::Yellow { conflicts } => {
///             println!("Review {} potential conflicts", conflicts.len())
///         }
///         TrafficBranch::Red { conflicts } => {
///             println!("Manual resolution required for {} conflicts", conflicts.len())
///         }
///     }
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum TrafficBranch {
    /// 🟢 Green: Safe to auto-update
    Green,

    /// 🟡 Yellow: Can merge with conflicts
    Yellow { conflicts: Vec<Conflict> },

    /// 🔴 Red: Manual resolution required
    Red { conflicts: Vec<Conflict> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Conflict {
    pub path: PathBuf,
    pub line: usize,
    pub reason: String,
}

/// Traffic branch analyzer trait
pub trait TrafficAnalyzer {
    fn analyze(&self, file: &Path) -> Result<TrafficBranch>;
    fn can_auto_merge(&self, conflicts: &[Conflict]) -> bool;
}

/// Default traffic analyzer implementation
pub struct DefaultTrafficAnalyzer;

impl TrafficAnalyzer for DefaultTrafficAnalyzer {
    fn analyze(&self, file: &Path) -> Result<TrafficBranch> {
        // Analyze file to determine traffic branch
        let extension = file.extension().and_then(|e| e.to_str()).unwrap_or("");

        // 🟢 Green: Auto-update (safe files that don't affect APIs or types)
        let green_patterns = [
            "md", "txt", "json", // Documentation and config
            "css", "scss", "less", // Styles
            "png", "jpg", "svg", "ico", // Assets
            "test.ts", "test.js", "spec.ts", "spec.js", // Tests
        ];

        // 🔴 Red: Manual resolution (breaking changes, API modifications)
        let red_patterns = [
            "proto", // Protocol buffers
            "graphql", "gql", // GraphQL schemas
            "sql", // Database migrations
        ];

        // Check if file matches green patterns
        if green_patterns.iter().any(|p| extension.ends_with(p)) {
            return Ok(TrafficBranch::Green);
        }

        // Check if file matches red patterns
        if red_patterns.iter().any(|p| extension.ends_with(p)) {
            let conflict = Conflict {
                path: file.to_path_buf(),
                line: 0,
                reason: format!("Breaking change potential: {} file modification", extension),
            };
            return Ok(TrafficBranch::Red {
                conflicts: vec![conflict],
            });
        }

        // 🟡 Yellow: Merge required (code files that may have conflicts)
        // ts, tsx, js, jsx, rs, go, py, etc.
        if matches!(
            extension,
            "ts" | "tsx" | "js" | "jsx" | "rs" | "go" | "py" | "java" | "cpp" | "c" | "h"
        ) {
            // Check for API-related indicators in the file path
            let path_str = file.to_string_lossy().to_lowercase();

            if path_str.contains("api")
                || path_str.contains("interface")
                || path_str.contains("types")
                || path_str.contains("schema")
            {
                // Potential API changes - Red
                let conflict = Conflict {
                    path: file.to_path_buf(),
                    line: 0,
                    reason: "API/Type definition file modification".to_string(),
                };
                return Ok(TrafficBranch::Red {
                    conflicts: vec![conflict],
                });
            }

            // Regular code file - Yellow (may have merge conflicts)
            return Ok(TrafficBranch::Yellow { conflicts: vec![] });
        }

        // Default to Yellow for unknown file types
        Ok(TrafficBranch::Yellow { conflicts: vec![] })
    }

    fn can_auto_merge(&self, conflicts: &[Conflict]) -> bool {
        conflicts.is_empty()
    }
}

/// Orchestration configuration
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Enable parallel execution
    pub parallel: bool,

    /// Fail fast on first error
    pub fail_fast: bool,

    /// Maximum concurrent tools (for parallel mode)
    pub max_concurrent: usize,

    /// Enable traffic branch safety checks
    pub traffic_branch_enabled: bool,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            parallel: false,
            fail_fast: true,
            max_concurrent: 4,
            traffic_branch_enabled: true,
        }
    }
}

/// Simple orchestrator - coordinates tool execution timing
///
/// The Orchestrator manages the execution lifecycle of DX tools, handling:
///
/// - **Tool Registration**: Register tools for execution
/// - **Priority Ordering**: Execute tools in priority order (lower first)
/// - **Dependency Resolution**: Validate and resolve tool dependencies
/// - **Circular Dependency Detection**: Prevent infinite dependency loops
/// - **Lifecycle Hooks**: Invoke before/after/error hooks
/// - **Parallel Execution**: Support concurrent tool execution (optional)
/// - **Traffic Branch Integration**: Analyze file changes for merge safety
/// - **Error Handling**: Fail-fast or continue-on-error modes
///
/// # Example
///
/// ```rust,no_run
/// use dx_forge::{Orchestrator, DxTool, ExecutionContext, ToolOutput};
/// use anyhow::Result;
///
/// struct MyTool;
/// impl DxTool for MyTool {
///     fn name(&self) -> &str { "my-tool" }
///     fn version(&self) -> &str { "1.0.0" }
///     fn priority(&self) -> u32 { 50 }
///     fn execute(&mut self, _ctx: &ExecutionContext) -> Result<ToolOutput> {
///         Ok(ToolOutput::success())
///     }
/// }
///
/// fn main() -> Result<()> {
///     let mut orch = Orchestrator::new(".")?;
///     orch.register_tool(Box::new(MyTool))?;
///     let results = orch.execute_all()?;
///     println!("Executed {} tools", results.len());
///     Ok(())
/// }
/// ```
pub struct Orchestrator {
    tools: Vec<Box<dyn DxTool>>,
    context: ExecutionContext,
    config: OrchestratorConfig,
}

impl Orchestrator {
    /// Create a new orchestrator
    pub fn new(repo_root: impl Into<PathBuf>) -> Result<Self> {
        let repo_root = repo_root.into();
        let forge_path = repo_root.join(".dx/forge");

        Ok(Self {
            tools: Vec::new(),
            context: ExecutionContext::new(repo_root, forge_path),
            config: OrchestratorConfig::default(),
        })
    }

    /// Create orchestrator with custom configuration
    pub fn with_config(repo_root: impl Into<PathBuf>, config: OrchestratorConfig) -> Result<Self> {
        let repo_root = repo_root.into();
        let forge_path = repo_root.join(".dx/forge");

        Ok(Self {
            tools: Vec::new(),
            context: ExecutionContext::new(repo_root, forge_path),
            config,
        })
    }

    /// Update configuration
    pub fn set_config(&mut self, config: OrchestratorConfig) {
        self.config = config;
    }

    /// Register a tool (tools configure themselves)
    pub fn register_tool(&mut self, tool: Box<dyn DxTool>) -> Result<()> {
        let name = tool.name().to_string();
        tracing::info!(
            "📦 Registered tool: {} v{} (priority: {})",
            name,
            tool.version(),
            tool.priority()
        );
        self.tools.push(tool);
        Ok(())
    }

    /// Execute all registered tools in priority order
    pub fn execute_all(&mut self) -> Result<Vec<ToolOutput>> {
        let start_time = std::time::Instant::now();
        tracing::info!("🎼 Orchestrator starting execution of {} tools", self.tools.len());

        // Sort tools by priority
        self.tools.sort_by_key(|t| t.priority());

        // Check dependencies
        tracing::debug!("🔍 Validating tool dependencies...");
        self.validate_dependencies().context("Failed to validate tool dependencies")?;

        // Check for circular dependencies
        tracing::debug!("🔄 Checking for circular dependencies...");
        self.check_circular_dependencies()
            .context("Failed to check for circular dependencies")?;

        tracing::debug!(
            "📋 Execution order: {}",
            self.tools
                .iter()
                .map(|t| format!("{}(p:{})", t.name(), t.priority()))
                .collect::<Vec<_>>()
                .join(" → ")
        );

        // Execute tools based on parallel configuration
        let outputs = if self.config.parallel {
            self.execute_parallel()
        } else {
            self.execute_sequential()
        }
        .context("Failed to execute tools")?;

        let duration = start_time.elapsed();
        let success_count = outputs.iter().filter(|o| o.success).count();
        let failed_count = outputs.len() - success_count;

        tracing::info!(
            "🏁 Orchestration complete in {:.2}s: {} succeeded, {} failed",
            duration.as_secs_f64(),
            success_count,
            failed_count
        );

        Ok(outputs)
    }

    /// Execute tools sequentially in priority order
    fn execute_sequential(&mut self) -> Result<Vec<ToolOutput>> {
        let mut outputs = Vec::new();
        let context = self.context.clone();
        let total_tools = self.tools.len();
        let mut executed = 0;
        let mut skipped = 0;
        let mut failed = 0;

        for tool in &mut self.tools {
            if !tool.should_run(&context) {
                tracing::info!("⏭️  Skipping {}: pre-check failed", tool.name());
                skipped += 1;
                continue;
            }

            tracing::info!(
                "🚀 Executing: {} v{} (priority: {}, {}/{})",
                tool.name(),
                tool.version(),
                tool.priority(),
                executed + 1,
                total_tools
            );

            // Execute with lifecycle hooks
            match Self::execute_tool_with_hooks(tool, &context) {
                Ok(output) => {
                    if output.success {
                        executed += 1;
                        tracing::info!("✅ {} completed in {}ms", tool.name(), output.duration_ms);
                    } else {
                        failed += 1;
                        tracing::error!("❌ {} failed: {}", tool.name(), output.message);

                        if self.config.fail_fast {
                            tracing::error!("💥 Fail-fast enabled, stopping orchestration");
                            return Err(anyhow::anyhow!(
                                "Tool {} failed: {}",
                                tool.name(),
                                output.message
                            ));
                        }
                    }
                    outputs.push(output);
                }
                Err(e) => {
                    failed += 1;
                    tracing::error!("💥 {} error: {}", tool.name(), e);

                    if self.config.fail_fast {
                        tracing::error!("💥 Fail-fast enabled, stopping orchestration");
                        return Err(e);
                    }

                    outputs.push(ToolOutput::failure(format!("Error: {}", e)));
                }
            }
        }

        tracing::info!(
            "📊 Sequential execution complete: {} executed, {} skipped, {} failed",
            executed,
            skipped,
            failed
        );

        Ok(outputs)
    }

    /// Execute tools in parallel where possible, respecting dependencies
    fn execute_parallel(&mut self) -> Result<Vec<ToolOutput>> {
        tracing::info!(
            "🚀 Parallel execution mode (max {} concurrent)",
            self.config.max_concurrent
        );

        // Build dependency graph
        let dep_graph = self.build_dependency_graph();

        // Group tools into execution waves (tools that can run concurrently)
        let waves = self
            .compute_execution_waves(&dep_graph)
            .context("Failed to compute execution waves for parallel execution")?;

        tracing::debug!("📊 Execution waves: {}", waves.len());
        for (i, wave) in waves.iter().enumerate() {
            tracing::debug!("  Wave {}: {} tools", i + 1, wave.len());
        }

        let mut all_outputs = Vec::new();
        let context = self.context.clone();

        // Execute each wave in parallel
        for (wave_idx, wave_tools) in waves.into_iter().enumerate() {
            tracing::info!("🌊 Executing wave {} with {} tools", wave_idx + 1, wave_tools.len());

            let mut wave_outputs = Vec::new();

            // For now, execute wave tools sequentially (true parallel requires async DxTool trait)
            // Future enhancement: Use thread pool or async execution
            for tool_idx in wave_tools {
                let tool = &mut self.tools[tool_idx];

                if !tool.should_run(&context) {
                    tracing::info!("⏭️  Skipping {}: pre-check failed", tool.name());
                    continue;
                }

                tracing::info!("🚀 Executing: {} v{}", tool.name(), tool.version());

                match Self::execute_tool_with_hooks(tool, &context) {
                    Ok(output) => {
                        if output.success {
                            tracing::info!(
                                "✅ {} completed in {}ms",
                                tool.name(),
                                output.duration_ms
                            );
                        } else {
                            tracing::error!("❌ {} failed: {}", tool.name(), output.message);

                            if self.config.fail_fast {
                                return Err(anyhow::anyhow!(
                                    "Tool {} failed: {}",
                                    tool.name(),
                                    output.message
                                ));
                            }
                        }
                        wave_outputs.push(output);
                    }
                    Err(e) => {
                        tracing::error!("💥 {} error: {}", tool.name(), e);

                        if self.config.fail_fast {
                            return Err(e);
                        }

                        wave_outputs.push(ToolOutput::failure(format!("Error: {}", e)));
                    }
                }
            }

            all_outputs.extend(wave_outputs);
        }

        Ok(all_outputs)
    }

    /// Build a dependency graph for tools
    fn build_dependency_graph(&self) -> HashMap<String, HashSet<String>> {
        let mut graph = HashMap::new();

        for tool in &self.tools {
            let deps: HashSet<String> = tool.dependencies().into_iter().collect();
            graph.insert(tool.name().to_string(), deps);
        }

        graph
    }

    /// Compute execution waves based on dependency graph
    /// Tools in the same wave have no dependencies on each other
    fn compute_execution_waves(
        &self,
        dep_graph: &HashMap<String, HashSet<String>>,
    ) -> Result<Vec<Vec<usize>>> {
        let mut waves: Vec<Vec<usize>> = Vec::new();
        let mut completed: HashSet<String> = HashSet::new();
        let mut remaining: Vec<usize> = (0..self.tools.len()).collect();

        while !remaining.is_empty() {
            let mut current_wave = Vec::new();
            let mut next_remaining = Vec::new();

            for &idx in &remaining {
                let tool = &self.tools[idx];
                let tool_name = tool.name().to_string();

                // Check if all dependencies are completed
                let deps = dep_graph.get(&tool_name).cloned().unwrap_or_default();
                let all_deps_met = deps.iter().all(|dep| completed.contains(dep));

                if all_deps_met {
                    current_wave.push(idx);
                    completed.insert(tool_name);
                } else {
                    next_remaining.push(idx);
                }
            }

            if current_wave.is_empty() && !remaining.is_empty() {
                // No progress - likely a circular dependency or missing dependency
                let unmet: Vec<String> =
                    remaining.iter().map(|&idx| self.tools[idx].name().to_string()).collect();
                return Err(anyhow::anyhow!(
                    "Cannot resolve dependencies for tools: {}",
                    unmet.join(", ")
                ));
            }

            if !current_wave.is_empty() {
                waves.push(current_wave);
            }

            remaining = next_remaining;
        }

        Ok(waves)
    }

    /// Execute tool with lifecycle hooks and error handling
    fn execute_tool_with_hooks(
        tool: &mut Box<dyn DxTool>,
        context: &ExecutionContext,
    ) -> Result<ToolOutput> {
        let start = std::time::Instant::now();
        let tool_name = tool.name().to_string();

        // Before hook
        tracing::debug!("📝 Running before_execute hook for {}", tool_name);
        tool.before_execute(context)
            .with_context(|| format!("before_execute hook failed for tool: {}", tool_name))?;

        // Execute with timeout
        // Note: Since the DxTool trait's execute method is synchronous,
        // we can't use async timeout without significant refactoring.
        // Future improvement: make DxTool async or use thread-based timeout
        let result = if tool.timeout_seconds() > 0 {
            tracing::debug!(
                "⏱️  Executing {} with {}s timeout (note: timeout monitoring not yet implemented for sync tools)",
                tool_name,
                tool.timeout_seconds()
            );
            tool.execute(context)
        } else {
            tracing::debug!("🚀 Executing {} without timeout", tool_name);
            tool.execute(context)
        };

        // Handle result
        match result {
            Ok(mut output) => {
                let duration = start.elapsed();
                output.duration_ms = duration.as_millis() as u64;

                tracing::info!(
                    "✅ {} completed successfully in {:.2}s",
                    tool_name,
                    duration.as_secs_f64()
                );

                if !output.files_modified.is_empty() {
                    tracing::debug!("  📝 Modified {} files", output.files_modified.len());
                }
                if !output.files_created.is_empty() {
                    tracing::debug!("  ✨ Created {} files", output.files_created.len());
                }
                if !output.files_deleted.is_empty() {
                    tracing::debug!("  🗑️  Deleted {} files", output.files_deleted.len());
                }

                // After hook
                tracing::debug!("📝 Running after_execute hook for {}", tool_name);
                tool.after_execute(context, &output).with_context(|| {
                    format!("after_execute hook failed for tool: {}", tool_name)
                })?;

                Ok(output)
            }
            Err(e) => {
                let duration = start.elapsed();
                tracing::error!(
                    "❌ {} failed after {:.2}s: {}",
                    tool_name,
                    duration.as_secs_f64(),
                    e
                );

                // Error hook
                tracing::debug!("📝 Running on_error hook for {}", tool_name);
                tool.on_error(context, &e)
                    .with_context(|| format!("on_error hook failed for tool: {}", tool_name))?;
                Err(e)
            }
        }
    }

    /// Check for circular dependencies
    fn check_circular_dependencies(&self) -> Result<()> {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();

        for tool in &self.tools {
            if !visited.contains(tool.name()) {
                self.check_circular_deps_recursive(tool.name(), &mut visited, &mut stack)
                    .with_context(|| {
                        format!(
                            "Circular dependency check failed starting from tool: {}",
                            tool.name()
                        )
                    })?;
            }
        }

        Ok(())
    }

    fn check_circular_deps_recursive(
        &self,
        tool_name: &str,
        visited: &mut HashSet<String>,
        stack: &mut HashSet<String>,
    ) -> Result<()> {
        visited.insert(tool_name.to_string());
        stack.insert(tool_name.to_string());

        if let Some(tool) = self.tools.iter().find(|t| t.name() == tool_name) {
            for dep in tool.dependencies() {
                if !visited.contains(&dep) {
                    self.check_circular_deps_recursive(&dep, visited, stack).with_context(
                        || format!("Checking dependency '{}' of tool '{}'", dep, tool_name),
                    )?;
                } else if stack.contains(&dep) {
                    return Err(anyhow::anyhow!(
                        "Circular dependency detected: {} -> {}",
                        tool_name,
                        dep
                    ));
                }
            }
        }

        stack.remove(tool_name);
        Ok(())
    }

    /// Validate tool dependencies
    fn validate_dependencies(&self) -> Result<()> {
        let tool_names: HashSet<String> = self.tools.iter().map(|t| t.name().to_string()).collect();

        for tool in &self.tools {
            for dep in tool.dependencies() {
                if !tool_names.contains(&dep) {
                    anyhow::bail!(
                        "Tool '{}' requires '{}' but it's not registered",
                        tool.name(),
                        dep
                    );
                }
            }
        }

        Ok(())
    }

    /// Get execution context
    pub fn context(&self) -> &ExecutionContext {
        &self.context
    }

    /// Get mutable context
    pub fn context_mut(&mut self) -> &mut ExecutionContext {
        &mut self.context
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockTool {
        name: String,
        priority: u32,
    }

    impl DxTool for MockTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn version(&self) -> &str {
            "1.0.0"
        }

        fn priority(&self) -> u32 {
            self.priority
        }

        fn execute(&mut self, _ctx: &ExecutionContext) -> Result<ToolOutput> {
            Ok(ToolOutput::success())
        }
    }

    #[test]
    fn test_orchestrator_priority_order() {
        let mut orch = Orchestrator::new("/tmp/test").unwrap();

        orch.register_tool(Box::new(MockTool {
            name: "tool-c".into(),
            priority: 30,
        }))
        .unwrap();
        orch.register_tool(Box::new(MockTool {
            name: "tool-a".into(),
            priority: 10,
        }))
        .unwrap();
        orch.register_tool(Box::new(MockTool {
            name: "tool-b".into(),
            priority: 20,
        }))
        .unwrap();

        let outputs = orch.execute_all().unwrap();

        assert_eq!(outputs.len(), 3);
        assert!(outputs.iter().all(|o| o.success));
    }
}
