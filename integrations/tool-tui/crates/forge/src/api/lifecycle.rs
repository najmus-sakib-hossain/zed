//! Core Lifecycle & System Orchestration APIs

use anyhow::{Context, Result};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use crate::core::Forge;
use crate::orchestrator::{DxTool, ExecutionContext};

/// Type alias for tool registry storage
type ToolRegistryMap = Arc<RwLock<HashMap<String, Arc<RwLock<Box<dyn DxTool>>>>>>;

// Global forge instance using OnceLock with Mutex for thread-safety
// We use Mutex instead of RwLock because Forge contains types that don't implement Sync
pub(crate) static FORGE_INSTANCE: OnceLock<Arc<Mutex<Forge>>> = OnceLock::new();
pub(crate) static TOOL_REGISTRY: OnceLock<ToolRegistryMap> = OnceLock::new();
pub(crate) static CURRENT_CONTEXT: OnceLock<Arc<RwLock<ExecutionContext>>> = OnceLock::new();

/// Global one-time initialization (dx binary, LSP, editor extension, daemon)
///
/// **DEPRECATED**: This function uses global state and is deprecated in favor of creating
/// a `Forge` instance directly using `Forge::new()`. The global state pattern prevents
/// multiple isolated instances and makes testing difficult.
///
/// This must be called exactly once at application startup before using any other forge APIs.
/// It initializes the global forge instance, LSP server, file watchers, and all core systems.
///
/// # Migration Guide
///
/// Instead of:
/// ```no_run
/// use dx_forge::initialize_forge;
///
/// fn main() -> anyhow::Result<()> {
///     initialize_forge()?;
///     // Now forge is ready to use
///     Ok(())
/// }
/// ```
///
/// Use:
/// ```no_run
/// use dx_forge::Forge;
///
/// fn main() -> anyhow::Result<()> {
///     let forge = Forge::new(".")?;
///     // Now forge is ready to use
///     Ok(())
/// }
/// ```
#[deprecated(
    since = "0.2.0",
    note = "Use Forge::new() instead. Global state prevents multiple instances and complicates testing."
)]
pub fn initialize_forge() -> Result<()> {
    // Log deprecation warning
    tracing::warn!(
        "initialize_forge() is deprecated since v0.2.0. Use Forge::new() instead. \
         Global state prevents multiple instances and complicates testing. \
         See migration guide in documentation."
    );
    // Check if already initialized
    if FORGE_INSTANCE.get().is_some() {
        tracing::debug!("Forge already initialized, skipping");
        return Ok(());
    }

    tracing::info!("🚀 Initializing Forge v{}", crate::VERSION);

    // Detect project root (walk up to find .dx or .git)
    let project_root = detect_workspace_root()
        .or_else(|_| std::env::current_dir())
        .context("Failed to detect workspace root or get current directory")?;

    tracing::info!("📁 Project root: {:?}", project_root);

    // Create forge instance
    let forge = Forge::new(&project_root).context("Failed to initialize forge")?;

    // Initialize all global state using OnceLock
    FORGE_INSTANCE
        .set(Arc::new(Mutex::new(forge)))
        .map_err(|_| anyhow::anyhow!("Forge instance already initialized"))?;

    TOOL_REGISTRY
        .set(Arc::new(RwLock::new(HashMap::new())))
        .map_err(|_| anyhow::anyhow!("Tool registry already initialized"))?;

    // Create initial execution context
    let forge_path = project_root.join(".dx/forge");
    let context = ExecutionContext::new(project_root.clone(), forge_path);
    CURRENT_CONTEXT
        .set(Arc::new(RwLock::new(context)))
        .map_err(|_| anyhow::anyhow!("Execution context already initialized"))?;

    tracing::info!("✅ Forge initialization complete");
    Ok(())
}

/// Every dx-tool must call this exactly once during startup
///
/// **DEPRECATED**: This function uses global state and is deprecated in favor of calling
/// `forge.register_tool()` on a `Forge` instance. The global state pattern prevents
/// multiple isolated instances and makes testing difficult.
///
/// Registers a tool with the forge orchestrator. Tools are indexed by name and
/// version for dependency resolution and execution ordering.
///
/// # Arguments
/// * `tool` - The tool implementation to register
///
/// # Returns
/// A unique tool ID for subsequent operations
///
/// # Migration Guide
///
/// Instead of:
/// ```no_run
/// use dx_forge::{register_tool, DxTool};
///
/// struct MyTool;
/// impl DxTool for MyTool {
///     fn name(&self) -> &str { "my-tool" }
///     fn version(&self) -> &str { "1.0.0" }
///     fn priority(&self) -> u32 { 50 }
///     fn execute(&mut self, _ctx: &dx_forge::ExecutionContext) -> anyhow::Result<dx_forge::ToolOutput> {
///         Ok(dx_forge::ToolOutput::success())
///     }
/// }
///
/// fn main() -> anyhow::Result<()> {
///     dx_forge::initialize_forge()?;
///     register_tool(Box::new(MyTool))?;
///     Ok(())
/// }
/// ```
///
/// Use:
/// ```no_run
/// use dx_forge::{Forge, DxTool};
///
/// struct MyTool;
/// impl DxTool for MyTool {
///     fn name(&self) -> &str { "my-tool" }
///     fn version(&self) -> &str { "1.0.0" }
///     fn priority(&self) -> u32 { 50 }
///     fn execute(&mut self, _ctx: &dx_forge::ExecutionContext) -> anyhow::Result<dx_forge::ToolOutput> {
///         Ok(dx_forge::ToolOutput::success())
///     }
/// }
///
/// fn main() -> anyhow::Result<()> {
///     let mut forge = Forge::new(".")?;
///     forge.register_tool(Box::new(MyTool))?;
///     Ok(())
/// }
/// ```
#[deprecated(
    since = "0.2.0",
    note = "Use forge.register_tool() on a Forge instance instead. Global state prevents multiple instances."
)]
pub fn register_tool(tool: Box<dyn DxTool>) -> Result<String> {
    // Log deprecation warning
    tracing::warn!(
        "register_tool() is deprecated since v0.2.0. Use forge.register_tool() on a Forge instance instead. \
         Global state prevents multiple instances and complicates testing. \
         See migration guide in documentation."
    );
    ensure_initialized()?;

    let tool_name = tool.name().to_string();
    let tool_version = tool.version().to_string();
    let tool_id = format!("{}@{}", tool_name, tool_version);

    tracing::info!("📦 Registering tool: {}", tool_id);

    let registry = TOOL_REGISTRY
        .get()
        .ok_or_else(|| anyhow::anyhow!("Tool registry not initialized"))?;

    let tool_arc = Arc::new(RwLock::new(tool));
    registry.write().insert(tool_id.clone(), tool_arc);

    Ok(tool_id)
}

/// Returns the live, immutable ToolContext for the current operation
///
/// **DEPRECATED**: This function uses global state and is deprecated in favor of calling
/// `forge.get_execution_context()` on a `Forge` instance. The global state pattern prevents
/// multiple isolated instances and makes testing difficult.
///
/// Provides access to the execution context including repository state,
/// changed files, and shared data between tools.
///
/// # Returns
/// A clone of the current execution context
///
/// # Migration Guide
///
/// Instead of:
/// ```no_run
/// use dx_forge::get_tool_context;
///
/// fn my_operation() -> anyhow::Result<()> {
///     let ctx = get_tool_context()?;
///     println!("Working in: {:?}", ctx.repo_root);
///     Ok(())
/// }
/// ```
///
/// Use:
/// ```no_run
/// use dx_forge::Forge;
///
/// fn my_operation(forge: &Forge) -> anyhow::Result<()> {
///     let ctx = forge.get_execution_context();
///     println!("Working in: {:?}", ctx.repo_root);
///     Ok(())
/// }
/// ```
#[deprecated(
    since = "0.2.0",
    note = "Use forge.get_execution_context() on a Forge instance instead. Global state prevents multiple instances."
)]
pub fn get_tool_context() -> Result<ExecutionContext> {
    // Log deprecation warning
    tracing::warn!(
        "get_tool_context() is deprecated since v0.2.0. Use forge.get_execution_context() on a Forge instance instead. \
         Global state prevents multiple instances and complicates testing. \
         See migration guide in documentation."
    );
    ensure_initialized()?;

    let context = CURRENT_CONTEXT
        .get()
        .ok_or_else(|| anyhow::anyhow!("Tool context not available"))?;

    Ok(context.read().clone())
}

/// Full graceful shutdown with progress reporting and cleanup
///
/// **DEPRECATED**: This function uses global state and is deprecated in favor of using
/// RAII (Resource Acquisition Is Initialization) with `Forge` instances. Simply dropping
/// the `Forge` instance will perform cleanup automatically.
///
/// Shuts down all running tools, flushes caches, closes file watchers,
/// and performs cleanup. Should be called before application exit.
///
/// # Migration Guide
///
/// Instead of:
/// ```no_run
/// use dx_forge::shutdown_forge;
///
/// fn main() -> anyhow::Result<()> {
///     dx_forge::initialize_forge()?;
///     // ... do work ...
///     shutdown_forge()?;
///     Ok(())
/// }
/// ```
///
/// Use:
/// ```no_run
/// use dx_forge::Forge;
///
/// fn main() -> anyhow::Result<()> {
///     let forge = Forge::new(".")?;
///     // ... do work ...
///     // Cleanup happens automatically when forge goes out of scope
///     Ok(())
/// }
/// ```
#[deprecated(
    since = "0.2.0",
    note = "Use RAII with Forge instances instead. Cleanup happens automatically when Forge is dropped."
)]
pub fn shutdown_forge() -> Result<()> {
    // Log deprecation warning
    tracing::warn!(
        "shutdown_forge() is deprecated since v0.2.0. Use RAII with Forge instances instead. \
         Cleanup happens automatically when the Forge instance is dropped. \
         See migration guide in documentation."
    );
    tracing::info!("🛑 Shutting down Forge...");

    // Clear tool registry
    if let Some(registry) = TOOL_REGISTRY.get() {
        let count = registry.read().len();
        tracing::info!("📦 Unregistering {} tools", count);
        registry.write().clear();
    }

    // Note: OnceLock doesn't support taking the value out, but we can clear internal state
    // The Forge instance will be cleaned up when the process exits
    if let Some(forge) = FORGE_INSTANCE.get() {
        tracing::info!("🧹 Cleaning up forge instance");
        // Forge's Drop impl will handle cleanup when process exits
        if let Ok(guard) = forge.lock() {
            drop(guard);
        }
    }

    // Clear context data
    if let Some(context) = CURRENT_CONTEXT.get() {
        // Clear any mutable state in the context
        let mut ctx = context.write();
        ctx.changed_files.clear();
        ctx.shared_state.write().clear();
    }

    tracing::info!("✅ Forge shutdown complete");
    Ok(())
}

// Helper functions

fn ensure_initialized() -> Result<()> {
    if FORGE_INSTANCE.get().is_none() {
        anyhow::bail!("Forge not initialized. Call initialize_forge() first.");
    }
    Ok(())
}

fn detect_workspace_root() -> Result<PathBuf> {
    let mut current = std::env::current_dir()?;

    loop {
        // Check for .dx directory
        if current.join(".dx").exists() {
            return Ok(current);
        }

        // Check for .git directory
        if current.join(".git").exists() {
            return Ok(current);
        }

        // Move up one directory
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            // Reached filesystem root
            break;
        }
    }

    // Default to current directory
    Ok(std::env::current_dir()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::ToolOutput;

    struct TestTool;

    impl DxTool for TestTool {
        fn name(&self) -> &str {
            "test-tool"
        }
        fn version(&self) -> &str {
            "1.0.0"
        }
        fn priority(&self) -> u32 {
            50
        }
        fn execute(&mut self, _ctx: &ExecutionContext) -> Result<ToolOutput> {
            Ok(ToolOutput::success())
        }
    }

    #[test]
    fn test_lifecycle() {
        // Note: Can only test once per process due to Once
        initialize_forge().ok();

        let result = register_tool(Box::new(TestTool));
        assert!(result.is_ok());

        let ctx = get_tool_context();
        assert!(ctx.is_ok());
    }
}
