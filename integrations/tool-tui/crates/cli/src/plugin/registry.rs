//! Plugin Registry
//!
//! Central registry for managing and discovering plugins.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use dashmap::DashMap;
use tokio::sync::RwLock;

use super::PluginType;
use super::native::NativeLoader;
use super::sandbox::{PluginSandbox, SandboxConfig};
use super::traits::{DxPlugin, PluginContext, PluginMetadata, PluginResult};
use super::wasm::WasmExecutor;

/// Plugin registry for managing all loaded plugins
pub struct PluginRegistry {
    /// Loaded plugins by name
    plugins: DashMap<String, Arc<RwLock<Box<dyn DxPlugin>>>>,
    /// Plugin metadata cache
    metadata: DashMap<String, PluginMetadata>,
    /// WASM executor
    wasm_executor: WasmExecutor,
    /// Native loader
    native_loader: NativeLoader,
    /// Default sandbox config
    default_sandbox: SandboxConfig,
    /// Plugin directories to search
    plugin_dirs: Vec<PathBuf>,
}

impl PluginRegistry {
    /// Create a new plugin registry
    pub fn new() -> Result<Self> {
        let wasm_executor = WasmExecutor::new()?;
        let native_loader = NativeLoader::new().allow_unsigned(); // TODO: Remove in production

        // Default plugin directories
        let mut plugin_dirs = Vec::new();

        if let Some(home) = dirs::home_dir() {
            plugin_dirs.push(home.join(".dx").join("plugins"));
        }

        Ok(Self {
            plugins: DashMap::new(),
            metadata: DashMap::new(),
            wasm_executor,
            native_loader,
            default_sandbox: SandboxConfig::default(),
            plugin_dirs,
        })
    }

    /// Set default sandbox configuration
    pub fn with_sandbox(mut self, config: SandboxConfig) -> Self {
        self.default_sandbox = config;
        self
    }

    /// Add a plugin directory
    pub fn add_plugin_dir(&mut self, dir: PathBuf) {
        self.plugin_dirs.push(dir);
    }

    /// Load plugins from all registered directories
    pub async fn load_all(&self) -> Result<usize> {
        let mut count = 0;

        for dir in &self.plugin_dirs {
            if dir.exists() {
                count += self.load_from_dir(dir).await?;
            }
        }

        Ok(count)
    }

    /// Load plugins from a directory
    pub async fn load_from_dir(&self, dir: &Path) -> Result<usize> {
        if !dir.exists() {
            return Ok(0);
        }

        let mut count = 0;
        let mut entries = tokio::fs::read_dir(dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if let Some(plugin_type) = PluginType::from_path(&path) {
                if let Err(e) = self.load_plugin(&path, plugin_type).await {
                    tracing::warn!("Failed to load plugin {:?}: {}", path, e);
                } else {
                    count += 1;
                }
            }
        }

        Ok(count)
    }

    /// Load a single plugin
    pub async fn load_plugin(&self, path: &Path, plugin_type: PluginType) -> Result<String> {
        let plugin: Box<dyn DxPlugin> = match plugin_type {
            PluginType::Wasm => {
                let wasm_plugin = self.wasm_executor.load(path).await?;
                Box::new(wasm_plugin)
            }
            PluginType::Native => {
                let native_plugin = self.native_loader.load(path)?;
                Box::new(native_plugin)
            }
        };

        let name = plugin.metadata().name.clone();
        let metadata = plugin.metadata().clone();

        self.plugins.insert(name.clone(), Arc::new(RwLock::new(plugin)));
        self.metadata.insert(name.clone(), metadata);

        Ok(name)
    }

    /// Register a plugin manually
    pub fn register(&self, plugin: Box<dyn DxPlugin>) -> String {
        let name = plugin.metadata().name.clone();
        let metadata = plugin.metadata().clone();

        self.plugins.insert(name.clone(), Arc::new(RwLock::new(plugin)));
        self.metadata.insert(name.clone(), metadata);

        name
    }

    /// Unload a plugin
    pub async fn unload(&self, name: &str) -> Result<()> {
        if let Some((_, plugin)) = self.plugins.remove(name) {
            let mut plugin = plugin.write().await;
            plugin.shutdown().await?;
        }
        self.metadata.remove(name);
        Ok(())
    }

    /// Get plugin metadata
    pub fn get_metadata(&self, name: &str) -> Option<PluginMetadata> {
        self.metadata.get(name).map(|m| m.clone())
    }

    /// List all plugin names
    pub fn list(&self) -> Vec<String> {
        self.plugins.iter().map(|e| e.key().clone()).collect()
    }

    /// List all plugin metadata
    pub fn list_metadata(&self) -> Vec<PluginMetadata> {
        self.metadata.iter().map(|e| e.value().clone()).collect()
    }

    /// Check if a plugin exists
    pub fn exists(&self, name: &str) -> bool {
        self.plugins.contains_key(name)
    }

    /// Execute a plugin
    pub async fn execute(&self, name: &str, args: &[String]) -> Result<PluginResult> {
        let plugin = self
            .plugins
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", name))?;

        let metadata = self
            .metadata
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Plugin metadata not found: {}", name))?;

        // Create sandbox for execution
        let sandbox_config = self.default_sandbox.clone();
        let mut sandbox = PluginSandbox::new(sandbox_config);

        // Check capabilities
        for cap in &metadata.capabilities {
            sandbox.check_capability(*cap)?;
        }

        // Create execution context
        let ctx = PluginContext::default()
            .with_args(args.to_vec())
            .with_capabilities(metadata.capabilities.iter().cloned());

        // Execute
        let plugin = plugin.read().await;
        let result = plugin.execute(&ctx).await?;

        Ok(result)
    }

    /// Execute a plugin with custom context
    pub async fn execute_with_context(
        &self,
        name: &str,
        ctx: &PluginContext,
    ) -> Result<PluginResult> {
        let plugin = self
            .plugins
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", name))?;

        let plugin = plugin.read().await;
        plugin.execute(ctx).await
    }

    /// Initialize all loaded plugins
    pub async fn init_all(&self) -> Result<()> {
        for entry in self.plugins.iter() {
            let mut plugin = entry.value().write().await;
            plugin.init().await?;
        }
        Ok(())
    }

    /// Shutdown all plugins
    pub async fn shutdown_all(&self) -> Result<()> {
        for entry in self.plugins.iter() {
            let mut plugin = entry.value().write().await;
            if let Err(e) = plugin.shutdown().await {
                tracing::warn!("Failed to shutdown plugin {}: {}", entry.key(), e);
            }
        }
        self.plugins.clear();
        self.metadata.clear();
        Ok(())
    }

    /// Health check all plugins
    pub async fn health_check_all(&self) -> HashMap<String, bool> {
        let mut results = HashMap::new();

        // Collect keys first to avoid holding DashMap references across await points
        let names: Vec<String> = self.plugins.iter().map(|e| e.key().clone()).collect();

        for name in names {
            if let Some(entry) = self.plugins.get(&name) {
                let plugin = entry.value().read().await;
                let healthy = plugin.health_check().await;
                results.insert(name, healthy);
            }
        }

        results
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new().expect("Failed to create default plugin registry")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_creation() {
        let registry = PluginRegistry::new();
        assert!(registry.is_ok());
    }

    #[tokio::test]
    async fn test_registry_empty_list() {
        let registry = PluginRegistry::new().unwrap();
        assert!(registry.list().is_empty());
    }

    #[tokio::test]
    async fn test_plugin_not_found() {
        let registry = PluginRegistry::new().unwrap();
        let result = registry.execute("nonexistent", &[]).await;
        assert!(result.is_err());
    }
}
