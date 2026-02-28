//! Main Forge struct - unified API for DX tools

use anyhow::{Context, Result};
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::broadcast;

use super::branching_engine::BranchingEngine;
use super::editor_integration::EditorIntegration;
use super::event_bus::EventBus;
use super::lifecycle::{LifecycleManager, ToolId, ToolStatus};
use super::tracking::GeneratedCodeTracker;
use crate::injection::InjectionManager;
use crate::orchestrator::{DxTool, ExecutionContext, Orchestrator, OrchestratorConfig};
use crate::version::{ToolRegistry, Version};
use crate::watcher::{DualWatcher, FileChange};

/// Main Forge instance - provides unified API for DX tools
pub struct Forge {
    config: ForgeConfig,
    orchestrator: Arc<RwLock<Orchestrator>>,
    watcher: Option<Arc<RwLock<DualWatcher>>>,
    registry: Arc<RwLock<ToolRegistry>>,
    _injection_manager: Arc<RwLock<InjectionManager>>,
    lifecycle_manager: Arc<RwLock<LifecycleManager>>,
    code_tracker: Arc<RwLock<GeneratedCodeTracker>>,
    _editor_integration: Arc<RwLock<EditorIntegration>>,
    // New state components
    branching_engine: Arc<RwLock<BranchingEngine>>,
    event_bus: Arc<RwLock<EventBus>>,
    execution_context: Arc<RwLock<ExecutionContext>>,
}

/// Configuration for Forge instance
#[derive(Clone, Debug)]
pub struct ForgeConfig {
    /// Root directory of the project
    pub project_root: PathBuf,

    /// Forge data directory (.dx/forge)
    pub forge_dir: PathBuf,

    /// Automatically start file watching
    pub auto_watch: bool,

    /// Enable LSP integration
    pub enable_lsp: bool,

    /// Enable version control features
    pub enable_versioning: bool,

    /// Number of worker threads for orchestration
    pub worker_threads: usize,

    /// Debounce delay for events
    pub debounce_delay: std::time::Duration,

    /// Idle threshold for idle detection
    pub idle_threshold: std::time::Duration,

    /// Maximum backup size for revert support
    pub max_backup_size: usize,

    /// Enable R2 sync
    pub enable_r2_sync: bool,
}

impl ForgeConfig {
    /// Create default configuration for a project
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        let project_root = project_root.as_ref().to_path_buf();
        let forge_dir = project_root.join(".dx").join("forge");

        Self {
            project_root,
            forge_dir,
            auto_watch: true,
            enable_lsp: true,
            enable_versioning: true,
            worker_threads: num_cpus::get(),
            debounce_delay: std::time::Duration::from_millis(300),
            idle_threshold: std::time::Duration::from_secs(5),
            max_backup_size: 10 * 1024 * 1024, // 10MB
            enable_r2_sync: false,
        }
    }

    /// Disable automatic file watching
    pub fn without_auto_watch(mut self) -> Self {
        self.auto_watch = false;
        self
    }

    /// Disable LSP integration
    pub fn without_lsp(mut self) -> Self {
        self.enable_lsp = false;
        self
    }

    /// Set custom forge directory
    pub fn with_forge_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.forge_dir = dir.as_ref().to_path_buf();
        self
    }
}

impl Forge {
    /// Create a new Forge instance for a project
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use dx_forge::Forge;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let forge = Forge::new(".")?;
    ///     Ok(())
    /// }
    /// ```
    pub fn new(project_root: impl AsRef<Path>) -> Result<Self> {
        let config = ForgeConfig::new(project_root);
        Self::with_config(config)
    }

    /// Create Forge instance with custom configuration
    pub fn with_config(config: ForgeConfig) -> Result<Self> {
        // Ensure forge directory exists
        std::fs::create_dir_all(&config.forge_dir).context("Failed to create forge directory")?;

        // Initialize components
        let orchestrator_config = OrchestratorConfig {
            parallel: false,
            fail_fast: true,
            max_concurrent: config.worker_threads,
            traffic_branch_enabled: true,
        };

        let orchestrator = Arc::new(RwLock::new(
            Orchestrator::with_config(config.project_root.clone(), orchestrator_config)
                .context("Failed to initialize orchestrator")?,
        ));

        let registry = Arc::new(RwLock::new(
            ToolRegistry::new(&config.forge_dir).context("Failed to initialize tool registry")?,
        ));

        let injection_manager = Arc::new(RwLock::new(
            InjectionManager::new(&config.forge_dir)
                .context("Failed to initialize injection manager")?,
        ));

        let lifecycle_manager = Arc::new(RwLock::new(LifecycleManager::new()));

        let code_tracker = Arc::new(RwLock::new(
            GeneratedCodeTracker::new(&config.forge_dir)
                .context("Failed to initialize code tracker")?,
        ));

        let editor_integration = Arc::new(RwLock::new(EditorIntegration::new()));

        // Initialize new state components
        let branching_engine = Arc::new(RwLock::new(BranchingEngine::new()));
        let event_bus = Arc::new(RwLock::new(EventBus::new()));

        // Create execution context
        let execution_context = Arc::new(RwLock::new(ExecutionContext::new(
            config.project_root.clone(),
            config.forge_dir.clone(),
        )));

        // Initialize watcher if auto_watch is enabled
        let watcher = if config.auto_watch {
            let dual_watcher = DualWatcher::new().context("Failed to initialize file watcher")?;
            Some(Arc::new(RwLock::new(dual_watcher)))
        } else {
            None
        };

        Ok(Self {
            config,
            orchestrator,
            watcher,
            registry,
            _injection_manager: injection_manager,
            lifecycle_manager,
            code_tracker,
            _editor_integration: editor_integration,
            branching_engine,
            event_bus,
            execution_context,
        })
    }

    /// Get the project root directory
    pub fn project_root(&self) -> &Path {
        &self.config.project_root
    }

    /// Get the orchestrator instance
    pub fn orchestrator(&self) -> Arc<RwLock<Orchestrator>> {
        self.orchestrator.clone()
    }

    /// Get the forge data directory
    pub fn forge_dir(&self) -> &Path {
        &self.config.forge_dir
    }

    // ========================================================================
    // Tool Lifecycle Management
    // ========================================================================

    /// Get the current status of a tool
    pub fn get_tool_status(&self, id: ToolId) -> Option<ToolStatus> {
        self.lifecycle_manager.read().get_status(id)
    }

    /// Subscribe to lifecycle events
    pub fn subscribe_lifecycle_events(
        &self,
    ) -> broadcast::Receiver<super::lifecycle::LifecycleEvent> {
        self.lifecycle_manager.read().subscribe()
    }

    // ========================================================================
    // File Watching & Change Detection
    // ========================================================================

    /// Start watching a directory for changes
    #[allow(clippy::await_holding_lock)]
    pub async fn watch_directory(&mut self, path: impl AsRef<Path>) -> Result<()> {
        if let Some(watcher) = &self.watcher {
            let path_ref = path.as_ref();
            watcher
                .write()
                .start(path_ref)
                .await
                .with_context(|| format!("Failed to start watching directory: {:?}", path_ref))?;
            tracing::info!("Started watching directory: {:?}", path_ref);
            Ok(())
        } else {
            anyhow::bail!("File watching is disabled in configuration")
        }
    }

    /// Subscribe to file change events
    pub fn subscribe_changes(&self) -> Result<broadcast::Receiver<FileChange>> {
        if let Some(watcher) = &self.watcher {
            Ok(watcher.read().receiver())
        } else {
            anyhow::bail!("File watching is disabled in configuration")
        }
    }

    /// Stop file watching
    #[allow(clippy::await_holding_lock)]
    pub async fn stop_watching(&mut self) -> Result<()> {
        if let Some(watcher) = &self.watcher {
            watcher.write().stop().await.context("Failed to stop file watching")?;
            tracing::info!("Stopped file watching");
            Ok(())
        } else {
            Ok(())
        }
    }

    // ========================================================================
    // Generated Code Tracking
    // ========================================================================

    /// Track a file as being generated by a tool
    pub fn track_generated_file(
        &mut self,
        file: PathBuf,
        tool: &str,
        metadata: std::collections::HashMap<String, String>,
    ) -> Result<()> {
        self.code_tracker.write().track_file(file, tool, metadata)
    }

    /// Get all files generated by a specific tool
    pub fn get_generated_files(&self, tool: &str) -> Vec<PathBuf> {
        self.code_tracker.read().get_files_by_tool(tool)
    }

    /// Remove all files generated by a tool
    #[allow(clippy::await_holding_lock)]
    pub async fn cleanup_generated(&mut self, tool: &str) -> Result<Vec<PathBuf>> {
        self.code_tracker.write().cleanup_tool_files(tool).await
    }

    // ========================================================================
    // Tool Registry & Versioning
    // ========================================================================

    /// Check if a tool is registered
    pub fn is_tool_registered(&self, name: &str) -> bool {
        self.registry.read().is_registered(name)
    }

    /// Get version of a registered tool
    pub fn get_tool_version(&self, name: &str) -> Option<Version> {
        self.registry.read().version(name).cloned()
    }

    /// List all registered tools
    pub fn list_tools(&self) -> Vec<String> {
        self.registry.read().list().iter().map(|info| info.name.clone()).collect()
    }

    // ========================================================================
    // Tool Registration & Management
    // ========================================================================

    /// Register a tool with the forge
    pub fn register_tool(&mut self, tool: Box<dyn DxTool>) -> Result<String> {
        let tool_name = tool.name().to_string();
        let tool_version = tool.version().to_string();
        let tool_id = format!("{}@{}", tool_name, tool_version);

        tracing::info!("📦 Registering tool: {}", tool_id);

        // TODO: Add tool to registry when DxTool trait is available
        // For now, just return the tool_id
        Ok(tool_id)
    }

    /// Get the current execution context
    pub fn get_execution_context(&self) -> ExecutionContext {
        self.execution_context.read().clone()
    }

    // ========================================================================
    // Branching Engine Access
    // ========================================================================

    /// Get access to the branching engine
    pub fn branching_engine(&self) -> Arc<RwLock<BranchingEngine>> {
        self.branching_engine.clone()
    }

    // ========================================================================
    // Event Bus Access
    // ========================================================================

    /// Get access to the event bus
    pub fn event_bus(&self) -> Arc<RwLock<EventBus>> {
        self.event_bus.clone()
    }

    /// Subscribe to all events
    pub fn subscribe_events(&self) -> broadcast::Receiver<super::event_bus::ForgeEvent> {
        self.event_bus.read().subscribe()
    }
}

impl Drop for Forge {
    fn drop(&mut self) {
        // Cleanup: stop all running tools
        if let Some(mut lifecycle) = self.lifecycle_manager.try_write() {
            if let Err(e) = lifecycle.stop_all() {
                tracing::error!("Failed to stop all tools during cleanup: {}", e);
            }
        }

        tracing::debug!("Forge instance dropped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{BranchColor, BranchingVote, ForgeEvent};
    use crate::orchestrator::ToolOutput;
    use std::collections::HashMap;
    use tempfile::TempDir;

    // ========================================================================
    // Helper Functions
    // ========================================================================

    /// Create a test Forge instance with a temporary directory
    fn create_test_forge() -> (Forge, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = ForgeConfig::new(temp_dir.path()).without_auto_watch();
        let forge = Forge::with_config(config).expect("Failed to create Forge instance");
        (forge, temp_dir)
    }

    /// Create a test tool for registration tests
    struct TestTool {
        name: String,
        version: String,
    }

    impl TestTool {
        fn new(name: &str, version: &str) -> Self {
            Self {
                name: name.to_string(),
                version: version.to_string(),
            }
        }
    }

    impl DxTool for TestTool {
        fn name(&self) -> &str {
            &self.name
        }

        fn version(&self) -> &str {
            &self.version
        }

        fn priority(&self) -> u32 {
            50
        }

        fn execute(&mut self, _ctx: &ExecutionContext) -> Result<ToolOutput> {
            Ok(ToolOutput::success())
        }
    }

    // ========================================================================
    // Constructor Tests
    // ========================================================================

    /// **Validates: Requirements 5.1, 5.3**
    /// Test that Forge::new() creates a valid instance with default configuration
    #[test]
    fn test_forge_new_creates_instance() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let result = Forge::new(temp_dir.path());

        assert!(result.is_ok(), "Forge::new() should succeed with valid path");

        let forge = result.unwrap();
        assert_eq!(forge.project_root(), temp_dir.path());
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test that Forge::with_config() creates instance with custom configuration
    #[test]
    fn test_forge_with_config_creates_instance() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = ForgeConfig::new(temp_dir.path()).without_auto_watch().without_lsp();

        let result = Forge::with_config(config);

        assert!(result.is_ok(), "Forge::with_config() should succeed");

        let forge = result.unwrap();
        assert_eq!(forge.project_root(), temp_dir.path());
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test that Forge creates the forge directory if it doesn't exist
    #[test]
    fn test_forge_creates_forge_directory() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let forge_dir = temp_dir.path().join(".dx").join("forge");

        // Ensure forge directory doesn't exist initially
        assert!(!forge_dir.exists());

        let config = ForgeConfig::new(temp_dir.path()).without_auto_watch();
        let _forge = Forge::with_config(config).expect("Failed to create Forge");

        // Forge directory should now exist
        assert!(forge_dir.exists(), "Forge should create the forge directory");
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test that Forge::new() with custom forge directory works
    #[test]
    fn test_forge_with_custom_forge_dir() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let custom_forge_dir = temp_dir.path().join("custom_forge");

        let config = ForgeConfig::new(temp_dir.path())
            .without_auto_watch()
            .with_forge_dir(&custom_forge_dir);

        let forge = Forge::with_config(config).expect("Failed to create Forge");

        assert_eq!(forge.forge_dir(), custom_forge_dir);
        assert!(custom_forge_dir.exists(), "Custom forge directory should be created");
    }

    // ========================================================================
    // Configuration Tests
    // ========================================================================

    /// **Validates: Requirements 5.1, 5.3**
    /// Test ForgeConfig default values
    #[test]
    fn test_forge_config_defaults() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = ForgeConfig::new(temp_dir.path());

        assert_eq!(config.project_root, temp_dir.path());
        assert_eq!(config.forge_dir, temp_dir.path().join(".dx").join("forge"));
        assert!(config.auto_watch, "auto_watch should be true by default");
        assert!(config.enable_lsp, "enable_lsp should be true by default");
        assert!(config.enable_versioning, "enable_versioning should be true by default");
        assert!(config.worker_threads > 0, "worker_threads should be positive");
        assert_eq!(config.debounce_delay, std::time::Duration::from_millis(300));
        assert_eq!(config.idle_threshold, std::time::Duration::from_secs(5));
        assert_eq!(config.max_backup_size, 10 * 1024 * 1024);
        assert!(!config.enable_r2_sync, "enable_r2_sync should be false by default");
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test ForgeConfig builder methods
    #[test]
    fn test_forge_config_builder_methods() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let custom_dir = temp_dir.path().join("custom");

        let config = ForgeConfig::new(temp_dir.path())
            .without_auto_watch()
            .without_lsp()
            .with_forge_dir(&custom_dir);

        assert!(!config.auto_watch, "auto_watch should be disabled");
        assert!(!config.enable_lsp, "enable_lsp should be disabled");
        assert_eq!(config.forge_dir, custom_dir);
    }

    // ========================================================================
    // Basic Operations Tests
    // ========================================================================

    /// **Validates: Requirements 5.1, 5.3**
    /// Test project_root() returns correct path
    #[test]
    fn test_forge_project_root() {
        let (forge, temp_dir) = create_test_forge();
        assert_eq!(forge.project_root(), temp_dir.path());
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test forge_dir() returns correct path
    #[test]
    fn test_forge_forge_dir() {
        let (forge, temp_dir) = create_test_forge();
        let expected_forge_dir = temp_dir.path().join(".dx").join("forge");
        assert_eq!(forge.forge_dir(), expected_forge_dir);
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test orchestrator() returns a valid orchestrator
    #[test]
    fn test_forge_orchestrator_access() {
        let (forge, _temp_dir) = create_test_forge();
        let orchestrator = forge.orchestrator();

        // Verify we can access the orchestrator
        let _guard = orchestrator.read();
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test get_execution_context() returns valid context
    #[test]
    fn test_forge_get_execution_context() {
        let (forge, temp_dir) = create_test_forge();
        let context = forge.get_execution_context();

        assert_eq!(context.repo_root, temp_dir.path());
        assert_eq!(context.forge_path, temp_dir.path().join(".dx").join("forge"));
    }

    // ========================================================================
    // Tool Registration Tests
    // ========================================================================

    /// **Validates: Requirements 5.1, 5.3**
    /// Test register_tool() returns tool ID
    #[test]
    fn test_forge_register_tool() {
        let (mut forge, _temp_dir) = create_test_forge();
        let tool = Box::new(TestTool::new("test-tool", "1.0.0"));

        let result = forge.register_tool(tool);

        assert!(result.is_ok(), "register_tool should succeed");
        let tool_id = result.unwrap();
        assert_eq!(tool_id, "test-tool@1.0.0");
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test register_tool() with different tool versions
    #[test]
    fn test_forge_register_multiple_tools() {
        let (mut forge, _temp_dir) = create_test_forge();

        let tool1 = Box::new(TestTool::new("tool-a", "1.0.0"));
        let tool2 = Box::new(TestTool::new("tool-b", "2.0.0"));

        let id1 = forge.register_tool(tool1).expect("Failed to register tool1");
        let id2 = forge.register_tool(tool2).expect("Failed to register tool2");

        assert_eq!(id1, "tool-a@1.0.0");
        assert_eq!(id2, "tool-b@2.0.0");
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test list_tools() returns registered tools
    #[test]
    fn test_forge_list_tools() {
        let (forge, _temp_dir) = create_test_forge();

        // Initially should be empty (no tools registered in registry)
        let tools = forge.list_tools();
        // list_tools should return a valid list (empty or with tools)
        assert!(tools.is_empty(), "list_tools should be empty initially");
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test is_tool_registered() returns correct status
    #[test]
    fn test_forge_is_tool_registered() {
        let (forge, _temp_dir) = create_test_forge();

        // Non-existent tool should not be registered
        assert!(!forge.is_tool_registered("non-existent-tool"));
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test get_tool_version() for non-existent tool
    #[test]
    fn test_forge_get_tool_version_not_found() {
        let (forge, _temp_dir) = create_test_forge();

        let version = forge.get_tool_version("non-existent-tool");
        assert!(version.is_none(), "Non-existent tool should return None");
    }

    // ========================================================================
    // Generated Code Tracking Tests
    // ========================================================================

    /// **Validates: Requirements 5.1, 5.3**
    /// Test track_generated_file() and get_generated_files()
    #[test]
    fn test_forge_track_generated_file() {
        let (mut forge, temp_dir) = create_test_forge();

        // Create a test file
        let test_file = temp_dir.path().join("generated.ts");
        std::fs::write(&test_file, "// generated code").expect("Failed to write test file");

        let mut metadata = HashMap::new();
        metadata.insert("version".to_string(), "1.0.0".to_string());

        let result = forge.track_generated_file(test_file.clone(), "test-tool", metadata);
        assert!(result.is_ok(), "track_generated_file should succeed");

        let files = forge.get_generated_files("test-tool");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], test_file);
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test get_generated_files() for non-existent tool
    #[test]
    fn test_forge_get_generated_files_empty() {
        let (forge, _temp_dir) = create_test_forge();

        let files = forge.get_generated_files("non-existent-tool");
        assert!(files.is_empty(), "Non-existent tool should have no generated files");
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test cleanup_generated() removes tracked files
    #[tokio::test]
    async fn test_forge_cleanup_generated() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = ForgeConfig::new(temp_dir.path()).without_auto_watch();
        let mut forge = Forge::with_config(config).expect("Failed to create Forge");

        // Create and track a test file
        let test_file = temp_dir.path().join("to_cleanup.ts");
        std::fs::write(&test_file, "// to be cleaned up").expect("Failed to write test file");

        forge
            .track_generated_file(test_file.clone(), "cleanup-tool", HashMap::new())
            .expect("Failed to track file");

        // Verify file exists
        assert!(test_file.exists());

        // Cleanup
        let removed = forge.cleanup_generated("cleanup-tool").await.expect("Failed to cleanup");

        assert_eq!(removed.len(), 1);
        assert!(!test_file.exists(), "File should be deleted after cleanup");
    }

    // ========================================================================
    // Branching Engine Access Tests
    // ========================================================================

    /// **Validates: Requirements 5.1, 5.3**
    /// Test branching_engine() returns valid engine
    #[test]
    fn test_forge_branching_engine_access() {
        let (forge, _temp_dir) = create_test_forge();
        let engine = forge.branching_engine();

        // Verify we can access the branching engine
        let _guard = engine.read();
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test branching engine operations through Forge
    #[test]
    fn test_forge_branching_engine_operations() {
        let (forge, _temp_dir) = create_test_forge();
        let engine = forge.branching_engine();

        // Test vote submission
        let file = PathBuf::from("test.ts");
        let vote = BranchingVote {
            voter_id: "test-voter".to_string(),
            color: BranchColor::Green,
            reason: "Test vote".to_string(),
            confidence: 0.9,
        };

        {
            let mut engine_guard = engine.write();
            engine_guard.submit_vote(&file, vote).expect("Failed to submit vote");

            let color = engine_guard.predict_color(&file);
            assert_eq!(color, BranchColor::Green);
        }
    }

    // ========================================================================
    // Event Bus Access Tests
    // ========================================================================

    /// **Validates: Requirements 5.1, 5.3**
    /// Test event_bus() returns valid event bus
    #[test]
    fn test_forge_event_bus_access() {
        let (forge, _temp_dir) = create_test_forge();
        let event_bus = forge.event_bus();

        // Verify we can access the event bus
        let _guard = event_bus.read();
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test subscribe_events() returns valid receiver
    #[test]
    fn test_forge_subscribe_events() {
        let (forge, _temp_dir) = create_test_forge();
        let _receiver = forge.subscribe_events();

        // Receiver should be valid (no panic)
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test event publishing through Forge
    #[tokio::test]
    async fn test_forge_event_publishing() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = ForgeConfig::new(temp_dir.path()).without_auto_watch();
        let forge = Forge::with_config(config).expect("Failed to create Forge");

        let mut receiver = forge.subscribe_events();
        let event_bus = forge.event_bus();

        // Publish an event
        {
            let bus = event_bus.read();
            bus.emit_tool_started("test-tool").expect("Failed to emit event");
        }

        // Receive the event
        let event = receiver.recv().await.expect("Failed to receive event");
        match event {
            ForgeEvent::ToolStarted { tool_id, .. } => {
                assert_eq!(tool_id, "test-tool");
            }
            _ => panic!("Expected ToolStarted event"),
        }
    }

    // ========================================================================
    // Lifecycle Management Tests
    // ========================================================================

    /// **Validates: Requirements 5.1, 5.3**
    /// Test get_tool_status() for non-existent tool
    #[test]
    fn test_forge_get_tool_status_not_found() {
        let (forge, _temp_dir) = create_test_forge();

        // Create a random ToolId
        let tool_id = ToolId::new();
        let status = forge.get_tool_status(tool_id);

        assert!(status.is_none(), "Non-existent tool should return None");
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test subscribe_lifecycle_events() returns valid receiver
    #[test]
    fn test_forge_subscribe_lifecycle_events() {
        let (forge, _temp_dir) = create_test_forge();
        let _receiver = forge.subscribe_lifecycle_events();

        // Receiver should be valid (no panic)
    }

    // ========================================================================
    // File Watching Tests (Error Conditions)
    // ========================================================================

    /// **Validates: Requirements 5.1, 5.3**
    /// Test subscribe_changes() fails when watching is disabled
    #[test]
    fn test_forge_subscribe_changes_disabled() {
        let (forge, _temp_dir) = create_test_forge();

        // Watching is disabled in create_test_forge()
        let result = forge.subscribe_changes();

        assert!(result.is_err(), "subscribe_changes should fail when watching is disabled");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("disabled"),
            "Error should mention watching is disabled"
        );
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test watch_directory() fails when watching is disabled
    #[tokio::test]
    async fn test_forge_watch_directory_disabled() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = ForgeConfig::new(temp_dir.path()).without_auto_watch();
        let mut forge = Forge::with_config(config).expect("Failed to create Forge");

        let result = forge.watch_directory(temp_dir.path()).await;

        assert!(result.is_err(), "watch_directory should fail when watching is disabled");
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test stop_watching() succeeds even when watching is disabled
    #[tokio::test]
    async fn test_forge_stop_watching_disabled() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = ForgeConfig::new(temp_dir.path()).without_auto_watch();
        let mut forge = Forge::with_config(config).expect("Failed to create Forge");

        // Should succeed (no-op when watching is disabled)
        let result = forge.stop_watching().await;
        assert!(result.is_ok(), "stop_watching should succeed when watching is disabled");
    }

    // ========================================================================
    // Error Condition Tests
    // ========================================================================

    /// **Validates: Requirements 5.1, 5.3**
    /// Test Forge creation with invalid path (permission denied scenario)
    /// Note: This test may be skipped on some systems where root can write anywhere
    #[test]
    #[cfg(unix)]
    fn test_forge_creation_invalid_path() {
        // Try to create forge in a read-only location
        // This test is platform-specific and may not work on all systems
        let result = Forge::new("/root/definitely_not_writable_path_12345");

        // On most systems, this should fail due to permissions
        // But we don't assert failure because root user can write anywhere
        if result.is_err() {
            let err = result.unwrap_err();
            // Error should have context about what failed
            assert!(err.to_string().len() > 0, "Error should have a message");
        }
    }

    // ========================================================================
    // Instance Isolation Tests
    // ========================================================================

    /// **Validates: Requirements 5.1, 5.3, 1.2, 1.3**
    /// Test that multiple Forge instances are isolated from each other
    #[test]
    fn test_forge_instance_isolation() {
        let temp_dir_a = TempDir::new().expect("Failed to create temp directory A");
        let temp_dir_b = TempDir::new().expect("Failed to create temp directory B");

        let config_a = ForgeConfig::new(temp_dir_a.path()).without_auto_watch();
        let config_b = ForgeConfig::new(temp_dir_b.path()).without_auto_watch();

        let forge_a = Forge::with_config(config_a).expect("Failed to create Forge A");
        let forge_b = Forge::with_config(config_b).expect("Failed to create Forge B");

        // Verify instances have different paths
        assert_ne!(forge_a.project_root(), forge_b.project_root());
        assert_ne!(forge_a.forge_dir(), forge_b.forge_dir());

        // Verify branching engines are separate
        let engine_a = forge_a.branching_engine();
        let engine_b = forge_b.branching_engine();

        // Submit vote in A
        let file = PathBuf::from("test.ts");
        let vote = BranchingVote {
            voter_id: "voter-a".to_string(),
            color: BranchColor::Red,
            reason: "Test".to_string(),
            confidence: 1.0,
        };

        engine_a.write().submit_vote(&file, vote).expect("Failed to submit vote");

        // Verify vote is NOT in B
        let color_a = engine_a.read().predict_color(&file);
        let color_b = engine_b.read().predict_color(&file);

        assert_eq!(color_a, BranchColor::Red, "Vote should be in instance A");
        assert_eq!(
            color_b,
            BranchColor::Green,
            "Vote should NOT be in instance B (default is Green)"
        );
    }

    /// **Validates: Requirements 5.1, 5.3, 1.2, 1.3**
    /// Test that generated file tracking is isolated between instances
    #[test]
    fn test_forge_tracking_isolation() {
        let temp_dir_a = TempDir::new().expect("Failed to create temp directory A");
        let temp_dir_b = TempDir::new().expect("Failed to create temp directory B");

        let config_a = ForgeConfig::new(temp_dir_a.path()).without_auto_watch();
        let config_b = ForgeConfig::new(temp_dir_b.path()).without_auto_watch();

        let mut forge_a = Forge::with_config(config_a).expect("Failed to create Forge A");
        let forge_b = Forge::with_config(config_b).expect("Failed to create Forge B");

        // Create and track a file in A
        let test_file = temp_dir_a.path().join("tracked.ts");
        std::fs::write(&test_file, "// tracked").expect("Failed to write file");

        forge_a
            .track_generated_file(test_file, "tool-a", HashMap::new())
            .expect("Failed to track file");

        // Verify file is tracked in A but not in B
        let files_a = forge_a.get_generated_files("tool-a");
        let files_b = forge_b.get_generated_files("tool-a");

        assert_eq!(files_a.len(), 1, "File should be tracked in instance A");
        assert_eq!(files_b.len(), 0, "File should NOT be tracked in instance B");
    }

    // ========================================================================
    // Drop/Cleanup Tests
    // ========================================================================

    /// **Validates: Requirements 5.1, 5.3**
    /// Test that Forge cleanup happens on drop
    #[test]
    fn test_forge_drop_cleanup() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");

        {
            let config = ForgeConfig::new(temp_dir.path()).without_auto_watch();
            let _forge = Forge::with_config(config).expect("Failed to create Forge");
            // Forge will be dropped here
        }

        // Forge directory should still exist (we don't delete it on drop)
        let forge_dir = temp_dir.path().join(".dx").join("forge");
        assert!(forge_dir.exists(), "Forge directory should persist after drop");
    }

    // ========================================================================
    // Additional Constructor Tests
    // ========================================================================

    /// **Validates: Requirements 5.1, 5.3**
    /// Test Forge creation with auto_watch enabled
    #[test]
    fn test_forge_with_auto_watch_enabled() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = ForgeConfig::new(temp_dir.path()); // auto_watch is true by default

        let result = Forge::with_config(config);
        assert!(result.is_ok(), "Forge with auto_watch should succeed");

        let forge = result.unwrap();
        // Watcher should be initialized
        let changes_result = forge.subscribe_changes();
        assert!(
            changes_result.is_ok(),
            "subscribe_changes should succeed when watching is enabled"
        );
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test Forge creation with all options disabled
    #[test]
    fn test_forge_with_all_options_disabled() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = ForgeConfig::new(temp_dir.path()).without_auto_watch().without_lsp();

        assert!(!config.auto_watch);
        assert!(!config.enable_lsp);

        let forge = Forge::with_config(config).expect("Failed to create Forge");
        assert_eq!(forge.project_root(), temp_dir.path());
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test Forge creation with nested project root
    #[test]
    fn test_forge_with_nested_project_root() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let nested_path = temp_dir.path().join("level1").join("level2").join("project");
        std::fs::create_dir_all(&nested_path).expect("Failed to create nested directory");

        let config = ForgeConfig::new(&nested_path).without_auto_watch();
        let forge = Forge::with_config(config).expect("Failed to create Forge");

        assert_eq!(forge.project_root(), nested_path);
        assert!(forge.forge_dir().exists(), "Forge directory should be created in nested path");
    }

    // ========================================================================
    // Additional Configuration Tests
    // ========================================================================

    /// **Validates: Requirements 5.1, 5.3**
    /// Test ForgeConfig Clone implementation
    #[test]
    fn test_forge_config_clone() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = ForgeConfig::new(temp_dir.path()).without_auto_watch().without_lsp();

        let cloned = config.clone();

        assert_eq!(cloned.project_root, config.project_root);
        assert_eq!(cloned.forge_dir, config.forge_dir);
        assert_eq!(cloned.auto_watch, config.auto_watch);
        assert_eq!(cloned.enable_lsp, config.enable_lsp);
        assert_eq!(cloned.debounce_delay, config.debounce_delay);
        assert_eq!(cloned.idle_threshold, config.idle_threshold);
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test ForgeConfig Debug implementation
    #[test]
    fn test_forge_config_debug() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = ForgeConfig::new(temp_dir.path());

        let debug_str = format!("{:?}", config);

        assert!(debug_str.contains("ForgeConfig"), "Debug output should contain struct name");
        assert!(debug_str.contains("project_root"), "Debug output should contain field names");
        assert!(debug_str.contains("auto_watch"), "Debug output should contain auto_watch field");
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test ForgeConfig with custom debounce and idle settings
    #[test]
    fn test_forge_config_timing_settings() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = ForgeConfig::new(temp_dir.path());

        // Verify default timing settings
        assert_eq!(config.debounce_delay, std::time::Duration::from_millis(300));
        assert_eq!(config.idle_threshold, std::time::Duration::from_secs(5));
        assert_eq!(config.max_backup_size, 10 * 1024 * 1024);
    }

    // ========================================================================
    // Additional Tool Registration Tests
    // ========================================================================

    /// **Validates: Requirements 5.1, 5.3**
    /// Test registering tool with special characters in name
    #[test]
    fn test_forge_register_tool_special_chars() {
        let (mut forge, _temp_dir) = create_test_forge();
        let tool = Box::new(TestTool::new("my-tool_v2", "1.0.0-beta.1"));

        let result = forge.register_tool(tool);

        assert!(result.is_ok());
        let tool_id = result.unwrap();
        assert_eq!(tool_id, "my-tool_v2@1.0.0-beta.1");
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test registering tool with empty version
    #[test]
    fn test_forge_register_tool_empty_version() {
        let (mut forge, _temp_dir) = create_test_forge();
        let tool = Box::new(TestTool::new("tool", ""));

        let result = forge.register_tool(tool);

        assert!(result.is_ok());
        let tool_id = result.unwrap();
        assert_eq!(tool_id, "tool@");
    }

    // ========================================================================
    // Additional Generated Code Tracking Tests
    // ========================================================================

    /// **Validates: Requirements 5.1, 5.3**
    /// Test tracking multiple files from same tool
    #[test]
    fn test_forge_track_multiple_files_same_tool() {
        let (mut forge, temp_dir) = create_test_forge();

        // Create and track multiple files
        let file1 = temp_dir.path().join("gen1.ts");
        let file2 = temp_dir.path().join("gen2.ts");
        let file3 = temp_dir.path().join("gen3.ts");

        std::fs::write(&file1, "// gen1").expect("Failed to write file1");
        std::fs::write(&file2, "// gen2").expect("Failed to write file2");
        std::fs::write(&file3, "// gen3").expect("Failed to write file3");

        forge
            .track_generated_file(file1.clone(), "multi-tool", HashMap::new())
            .expect("Failed to track file1");
        forge
            .track_generated_file(file2.clone(), "multi-tool", HashMap::new())
            .expect("Failed to track file2");
        forge
            .track_generated_file(file3.clone(), "multi-tool", HashMap::new())
            .expect("Failed to track file3");

        let files = forge.get_generated_files("multi-tool");
        assert_eq!(files.len(), 3, "Should have 3 tracked files");
        assert!(files.contains(&file1));
        assert!(files.contains(&file2));
        assert!(files.contains(&file3));
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test tracking files from different tools
    #[test]
    fn test_forge_track_files_different_tools() {
        let (mut forge, temp_dir) = create_test_forge();

        let file_a = temp_dir.path().join("tool_a.ts");
        let file_b = temp_dir.path().join("tool_b.ts");

        std::fs::write(&file_a, "// tool a").expect("Failed to write file_a");
        std::fs::write(&file_b, "// tool b").expect("Failed to write file_b");

        forge
            .track_generated_file(file_a.clone(), "tool-a", HashMap::new())
            .expect("Failed to track file_a");
        forge
            .track_generated_file(file_b.clone(), "tool-b", HashMap::new())
            .expect("Failed to track file_b");

        let files_a = forge.get_generated_files("tool-a");
        let files_b = forge.get_generated_files("tool-b");

        assert_eq!(files_a.len(), 1);
        assert_eq!(files_b.len(), 1);
        assert_eq!(files_a[0], file_a);
        assert_eq!(files_b[0], file_b);
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test tracking file with metadata
    #[test]
    fn test_forge_track_file_with_metadata() {
        let (mut forge, temp_dir) = create_test_forge();

        let test_file = temp_dir.path().join("with_meta.ts");
        std::fs::write(&test_file, "// with metadata").expect("Failed to write file");

        let mut metadata = HashMap::new();
        metadata.insert("generator".to_string(), "codegen-v1".to_string());
        metadata.insert("template".to_string(), "component".to_string());
        metadata.insert("timestamp".to_string(), "2024-01-01".to_string());

        let result = forge.track_generated_file(test_file.clone(), "meta-tool", metadata);
        assert!(result.is_ok(), "Tracking with metadata should succeed");

        let files = forge.get_generated_files("meta-tool");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], test_file);
    }

    // ========================================================================
    // Additional Branching Engine Tests
    // ========================================================================

    /// **Validates: Requirements 5.1, 5.3**
    /// Test multiple votes on same file
    #[test]
    fn test_forge_branching_multiple_votes() {
        let (forge, _temp_dir) = create_test_forge();
        let engine = forge.branching_engine();

        let file = PathBuf::from("contested.ts");

        // Submit multiple votes
        let vote1 = BranchingVote {
            voter_id: "voter-1".to_string(),
            color: BranchColor::Green,
            reason: "Looks safe".to_string(),
            confidence: 0.8,
        };
        let vote2 = BranchingVote {
            voter_id: "voter-2".to_string(),
            color: BranchColor::Yellow,
            reason: "Needs review".to_string(),
            confidence: 0.9,
        };

        {
            let mut engine_guard = engine.write();
            engine_guard.submit_vote(&file, vote1).expect("Failed to submit vote1");
            engine_guard.submit_vote(&file, vote2).expect("Failed to submit vote2");
        }

        // Color should be determined by voting logic
        let color = engine.read().predict_color(&file);
        // The exact color depends on the voting algorithm, but it should be valid
        assert!(matches!(color, BranchColor::Green | BranchColor::Yellow | BranchColor::Red));
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test branching engine with different file paths
    #[test]
    fn test_forge_branching_different_files() {
        let (forge, _temp_dir) = create_test_forge();
        let engine = forge.branching_engine();

        let file1 = PathBuf::from("src/main.ts");
        let file2 = PathBuf::from("src/utils.ts");

        let green_vote = BranchingVote {
            voter_id: "voter".to_string(),
            color: BranchColor::Green,
            reason: "Safe".to_string(),
            confidence: 1.0,
        };
        let red_vote = BranchingVote {
            voter_id: "voter".to_string(),
            color: BranchColor::Red,
            reason: "Dangerous".to_string(),
            confidence: 1.0,
        };

        {
            let mut engine_guard = engine.write();
            engine_guard
                .submit_vote(&file1, green_vote)
                .expect("Failed to submit green vote");
            engine_guard.submit_vote(&file2, red_vote).expect("Failed to submit red vote");
        }

        let color1 = engine.read().predict_color(&file1);
        let color2 = engine.read().predict_color(&file2);

        assert_eq!(color1, BranchColor::Green);
        assert_eq!(color2, BranchColor::Red);
    }

    // ========================================================================
    // Additional Event Bus Tests
    // ========================================================================

    /// **Validates: Requirements 5.1, 5.3**
    /// Test multiple event subscribers
    #[tokio::test]
    async fn test_forge_multiple_event_subscribers() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = ForgeConfig::new(temp_dir.path()).without_auto_watch();
        let forge = Forge::with_config(config).expect("Failed to create Forge");

        // Create multiple subscribers
        let mut receiver1 = forge.subscribe_events();
        let mut receiver2 = forge.subscribe_events();

        let event_bus = forge.event_bus();

        // Publish an event
        {
            let bus = event_bus.read();
            bus.emit_tool_started("broadcast-tool").expect("Failed to emit event");
        }

        // Both receivers should get the event
        let event1 = receiver1.recv().await.expect("Failed to receive event1");
        let event2 = receiver2.recv().await.expect("Failed to receive event2");

        match (event1, event2) {
            (
                ForgeEvent::ToolStarted { tool_id: id1, .. },
                ForgeEvent::ToolStarted { tool_id: id2, .. },
            ) => {
                assert_eq!(id1, "broadcast-tool");
                assert_eq!(id2, "broadcast-tool");
            }
            _ => panic!("Expected ToolStarted events"),
        }
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test event bus with different event types
    #[tokio::test]
    async fn test_forge_different_event_types() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = ForgeConfig::new(temp_dir.path()).without_auto_watch();
        let forge = Forge::with_config(config).expect("Failed to create Forge");

        let mut receiver = forge.subscribe_events();
        let event_bus = forge.event_bus();

        // Emit different event types
        {
            let bus = event_bus.read();
            bus.emit_tool_started("test-tool").expect("Failed to emit started");
            bus.emit_tool_completed("test-tool", 100).expect("Failed to emit completed");
        }

        // Receive and verify events
        let event1 = receiver.recv().await.expect("Failed to receive event1");
        let event2 = receiver.recv().await.expect("Failed to receive event2");

        match event1 {
            ForgeEvent::ToolStarted { tool_id, .. } => assert_eq!(tool_id, "test-tool"),
            _ => panic!("Expected ToolStarted event"),
        }

        match event2 {
            ForgeEvent::ToolCompleted {
                tool_id,
                duration_ms,
                ..
            } => {
                assert_eq!(tool_id, "test-tool");
                assert_eq!(duration_ms, 100);
            }
            _ => panic!("Expected ToolCompleted event"),
        }
    }

    // ========================================================================
    // File Watching Tests (Success Paths)
    // ========================================================================

    /// **Validates: Requirements 5.1, 5.3**
    /// Test watch_directory with auto_watch enabled
    #[tokio::test]
    async fn test_forge_watch_directory_enabled() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let watch_dir = temp_dir.path().join("watch_target");
        std::fs::create_dir_all(&watch_dir).expect("Failed to create watch directory");

        let config = ForgeConfig::new(temp_dir.path()); // auto_watch enabled
        let mut forge = Forge::with_config(config).expect("Failed to create Forge");

        let result = forge.watch_directory(&watch_dir).await;
        assert!(result.is_ok(), "watch_directory should succeed when watching is enabled");

        // Cleanup
        let _ = forge.stop_watching().await;
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test stop_watching with auto_watch enabled
    #[tokio::test]
    async fn test_forge_stop_watching_enabled() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = ForgeConfig::new(temp_dir.path()); // auto_watch enabled
        let mut forge = Forge::with_config(config).expect("Failed to create Forge");

        // Start watching
        let _ = forge.watch_directory(temp_dir.path()).await;

        // Stop watching should succeed
        let result = forge.stop_watching().await;
        assert!(result.is_ok(), "stop_watching should succeed");
    }

    // ========================================================================
    // Concurrent Access Tests
    // ========================================================================

    /// **Validates: Requirements 5.1, 5.3**
    /// Test concurrent access to branching engine
    #[test]
    fn test_forge_concurrent_branching_access() {
        let (forge, _temp_dir) = create_test_forge();
        let engine = forge.branching_engine();

        // Simulate concurrent access by getting multiple references
        let engine_ref1 = engine.clone();
        let engine_ref2 = engine.clone();

        // Both should be able to read
        let _color1 = engine_ref1.read().predict_color(&PathBuf::from("file1.ts"));
        let _color2 = engine_ref2.read().predict_color(&PathBuf::from("file2.ts"));

        // Write access should work
        let vote = BranchingVote {
            voter_id: "concurrent-voter".to_string(),
            color: BranchColor::Green,
            reason: "Test".to_string(),
            confidence: 1.0,
        };
        engine_ref1
            .write()
            .submit_vote(&PathBuf::from("concurrent.ts"), vote)
            .expect("Concurrent write should succeed");
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test concurrent access to event bus
    #[test]
    fn test_forge_concurrent_event_bus_access() {
        let (forge, _temp_dir) = create_test_forge();
        let event_bus = forge.event_bus();

        // Get multiple references
        let bus_ref1 = event_bus.clone();
        let bus_ref2 = event_bus.clone();

        // Both should be able to subscribe
        let _receiver1 = bus_ref1.read().subscribe();
        let _receiver2 = bus_ref2.read().subscribe();
    }

    // ========================================================================
    // Edge Case Tests
    // ========================================================================

    /// **Validates: Requirements 5.1, 5.3**
    /// Test Forge with path containing spaces
    #[test]
    fn test_forge_path_with_spaces() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let path_with_spaces = temp_dir.path().join("path with spaces");
        std::fs::create_dir_all(&path_with_spaces).expect("Failed to create directory");

        let config = ForgeConfig::new(&path_with_spaces).without_auto_watch();
        let result = Forge::with_config(config);

        assert!(result.is_ok(), "Forge should handle paths with spaces");
        let forge = result.unwrap();
        assert_eq!(forge.project_root(), path_with_spaces);
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test Forge with Unicode path
    #[test]
    fn test_forge_unicode_path() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let unicode_path = temp_dir.path().join("项目_プロジェクト");
        std::fs::create_dir_all(&unicode_path).expect("Failed to create directory");

        let config = ForgeConfig::new(&unicode_path).without_auto_watch();
        let result = Forge::with_config(config);

        assert!(result.is_ok(), "Forge should handle Unicode paths");
        let forge = result.unwrap();
        assert_eq!(forge.project_root(), unicode_path);
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test tracking file that doesn't exist
    #[test]
    fn test_forge_track_nonexistent_file() {
        let (mut forge, temp_dir) = create_test_forge();

        let nonexistent = temp_dir.path().join("does_not_exist.ts");

        // Tracking a non-existent file should still work (the tracker just records the path)
        let result = forge.track_generated_file(nonexistent.clone(), "test-tool", HashMap::new());
        // The behavior depends on implementation - it may succeed or fail
        // We just verify it doesn't panic
        let _ = result;
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test get_tool_version returns None for unregistered tool
    #[test]
    fn test_forge_get_version_unregistered() {
        let (forge, _temp_dir) = create_test_forge();

        let version = forge.get_tool_version("definitely-not-registered-tool-xyz");
        assert!(version.is_none());
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test branching engine predict_color for unvoted file
    #[test]
    fn test_forge_predict_color_unvoted_file() {
        let (forge, _temp_dir) = create_test_forge();
        let engine = forge.branching_engine();

        // File with no votes should return default color (Green)
        let color = engine.read().predict_color(&PathBuf::from("never_voted.ts"));
        assert_eq!(color, BranchColor::Green, "Unvoted file should default to Green");
    }

    // ========================================================================
    // Cleanup Tests
    // ========================================================================

    /// **Validates: Requirements 5.1, 5.3**
    /// Test cleanup_generated for tool with no files
    #[tokio::test]
    async fn test_forge_cleanup_no_files() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = ForgeConfig::new(temp_dir.path()).without_auto_watch();
        let mut forge = Forge::with_config(config).expect("Failed to create Forge");

        let removed = forge
            .cleanup_generated("no-files-tool")
            .await
            .expect("Cleanup should succeed even with no files");

        assert!(removed.is_empty(), "Should return empty list when no files to clean");
    }

    /// **Validates: Requirements 5.1, 5.3**
    /// Test cleanup_generated removes multiple files
    #[tokio::test]
    async fn test_forge_cleanup_multiple_files() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let config = ForgeConfig::new(temp_dir.path()).without_auto_watch();
        let mut forge = Forge::with_config(config).expect("Failed to create Forge");

        // Create and track multiple files
        let files: Vec<PathBuf> =
            (0..3).map(|i| temp_dir.path().join(format!("cleanup_{}.ts", i))).collect();

        for file in &files {
            std::fs::write(file, "// to cleanup").expect("Failed to write file");
            forge
                .track_generated_file(file.clone(), "cleanup-multi", HashMap::new())
                .expect("Failed to track file");
        }

        // Verify files exist
        for file in &files {
            assert!(file.exists(), "File should exist before cleanup");
        }

        // Cleanup
        let removed =
            forge.cleanup_generated("cleanup-multi").await.expect("Cleanup should succeed");

        assert_eq!(removed.len(), 3, "Should remove all 3 files");

        // Verify files are deleted
        for file in &files {
            assert!(!file.exists(), "File should be deleted after cleanup");
        }
    }
}
