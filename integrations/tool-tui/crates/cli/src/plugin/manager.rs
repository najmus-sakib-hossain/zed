//! Plugin Manager
//!
//! High-level orchestrator for the DX plugin system. Manages plugin lifecycle,
//! coordinates between the registry, sandbox, and hook system.
//!
//! # Architecture
//!
//! ```text
//! PluginManager
//! ├── PluginRegistry     (tracks loaded plugins)
//! ├── HookSystem         (event-based hooks)
//! ├── SandboxConfig      (default resource limits)
//! └── InstallDir         (plugin installation directory)
//! ```

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;

use super::PluginType;
use super::hooks::{HookData, HookExecutionResult, HookSystem};
use super::manifest::PluginManifest;
use super::registry::PluginRegistry;
use super::sandbox::SandboxConfig;
use super::traits::{Capability, PluginResult};
use super::validation;

/// Plugin manager configuration
#[derive(Debug, Clone)]
pub struct PluginManagerConfig {
    /// Plugin installation directory
    pub install_dir: PathBuf,
    /// Default sandbox configuration
    pub default_sandbox: SandboxConfig,
    /// Auto-load plugins from install_dir on startup
    pub auto_load: bool,
    /// Require signature for native plugins
    pub require_signatures: bool,
    /// Maximum number of concurrent plugin executions
    pub max_concurrent: usize,
    /// Plugin execution timeout (ms)
    pub default_timeout_ms: u64,
}

impl Default for PluginManagerConfig {
    fn default() -> Self {
        let install_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".dx")
            .join("plugins");

        Self {
            install_dir,
            default_sandbox: SandboxConfig::default(),
            auto_load: true,
            require_signatures: false,
            max_concurrent: 10,
            default_timeout_ms: 30_000,
        }
    }
}

/// Plugin installation result
#[derive(Debug)]
pub struct InstallResult {
    /// Plugin name
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Installation path
    pub path: PathBuf,
    /// Validation warnings
    pub warnings: Vec<String>,
}

/// Plugin manager — high-level orchestrator
pub struct PluginManager {
    /// Configuration
    config: PluginManagerConfig,
    /// Plugin registry
    registry: Arc<PluginRegistry>,
    /// Hook system
    hooks: Arc<HookSystem>,
    /// Active execution count
    active_executions: Arc<std::sync::atomic::AtomicUsize>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(config: PluginManagerConfig) -> Result<Self> {
        let registry = PluginRegistry::new()?;
        let hooks = HookSystem::new();

        Ok(Self {
            config,
            registry: Arc::new(registry),
            hooks: Arc::new(hooks),
            active_executions: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        })
    }

    /// Create with default configuration
    pub fn default_manager() -> Result<Self> {
        Self::new(PluginManagerConfig::default())
    }

    /// Initialize the plugin manager and auto-load plugins
    pub async fn init(&self) -> Result<usize> {
        // Ensure plugin directory exists
        if !self.config.install_dir.exists() {
            std::fs::create_dir_all(&self.config.install_dir)?;
        }

        let mut loaded = 0;

        if self.config.auto_load {
            loaded = self.registry.load_all().await?;
            // Initialize all loaded plugins
            self.registry.init_all().await?;
            tracing::info!("Loaded {} plugins from {:?}", loaded, self.config.install_dir);
        }

        Ok(loaded)
    }

    /// Get reference to the registry
    pub fn registry(&self) -> &PluginRegistry {
        &self.registry
    }

    /// Get reference to the hook system
    pub fn hooks(&self) -> &HookSystem {
        &self.hooks
    }

    /// Install a plugin from a file path
    pub async fn install(&self, source: &Path) -> Result<InstallResult> {
        // Determine plugin type
        let plugin_type = PluginType::from_path(source).ok_or_else(|| {
            anyhow::anyhow!("Cannot determine plugin type from path: {:?}", source)
        })?;

        // Try to load and validate manifest
        let manifest_path = source.parent().unwrap_or(source).join("plugin.yaml");

        let validation_warnings = if manifest_path.exists() {
            let manifest = PluginManifest::from_file(&manifest_path)?;
            let result = validation::validate_manifest(&manifest);

            if !result.is_valid() {
                let errors: Vec<String> =
                    result.errors().iter().map(|e| format!("{}: {}", e.field, e.message)).collect();
                anyhow::bail!("Manifest validation failed:\n{}", errors.join("\n"));
            }

            result
                .warnings()
                .iter()
                .map(|w| format!("{}: {}", w.field, w.message))
                .collect()
        } else {
            Vec::new()
        };

        // Check signature for native plugins if required
        if self.config.require_signatures && plugin_type == PluginType::Native {
            // Signature verification would happen here
            tracing::warn!("Signature verification not yet fully implemented for native plugins");
        }

        // Copy plugin to install directory
        let file_name = source.file_name().ok_or_else(|| anyhow::anyhow!("Invalid plugin path"))?;
        let dest = self.config.install_dir.join(file_name);

        tokio::fs::copy(source, &dest).await?;

        // Also copy manifest if present
        if manifest_path.exists() {
            let dest_manifest = dest.with_file_name(format!(
                "{}.yaml",
                dest.file_stem().and_then(|s| s.to_str()).unwrap_or("plugin")
            ));
            tokio::fs::copy(&manifest_path, &dest_manifest).await?;
        }

        // Load the plugin
        let name = self.registry.load_plugin(&dest, plugin_type).await?;

        let metadata = self
            .registry
            .get_metadata(&name)
            .ok_or_else(|| anyhow::anyhow!("Plugin loaded but metadata missing"))?;

        Ok(InstallResult {
            name,
            version: metadata.version,
            path: dest,
            warnings: validation_warnings,
        })
    }

    /// Uninstall a plugin
    pub async fn uninstall(&self, name: &str) -> Result<()> {
        // Get metadata before unloading
        let metadata = self.registry.get_metadata(name);

        // Unload from registry
        self.registry.unload(name).await?;

        // Unregister hooks
        self.hooks.unregister_plugin(name).await;

        // Remove files
        if let Some(metadata) = metadata {
            if metadata.path.exists() {
                tokio::fs::remove_file(&metadata.path).await?;
            }
            // Also remove manifest
            let manifest_path = metadata.path.with_extension("yaml");
            if manifest_path.exists() {
                tokio::fs::remove_file(&manifest_path).await.ok();
            }
        }

        tracing::info!("Uninstalled plugin: {}", name);
        Ok(())
    }

    /// Execute a plugin by name
    pub async fn execute(&self, name: &str, args: &[String]) -> Result<PluginResult> {
        // Check concurrent execution limit
        let current = self.active_executions.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        if current >= self.config.max_concurrent {
            self.active_executions.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            anyhow::bail!(
                "Maximum concurrent plugin executions ({}) reached",
                self.config.max_concurrent
            );
        }

        // Execute with timeout
        let timeout = std::time::Duration::from_millis(self.config.default_timeout_ms);

        let result = tokio::time::timeout(timeout, self.registry.execute(name, args)).await;

        self.active_executions.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);

        match result {
            Ok(inner) => inner,
            Err(_) => anyhow::bail!(
                "Plugin '{}' execution timed out after {}ms",
                name,
                self.config.default_timeout_ms
            ),
        }
    }

    /// Fire a hook event
    pub async fn fire_hook(&self, hook_data: &HookData) -> HookExecutionResult {
        self.hooks.execute(hook_data).await
    }

    /// List all installed plugins with their health status
    pub async fn list_plugins_with_health(&self) -> Vec<PluginInfo> {
        let metadata_list = self.registry.list_metadata();
        let health = self.registry.health_check_all().await;

        metadata_list
            .into_iter()
            .map(|m| {
                let healthy = health.get(&m.name).copied().unwrap_or(false);
                PluginInfo {
                    name: m.name.clone(),
                    version: m.version.clone(),
                    description: m.description.clone(),
                    plugin_type: m.plugin_type,
                    capabilities: m.capabilities.clone(),
                    healthy,
                }
            })
            .collect()
    }

    /// Get plugin count
    pub fn plugin_count(&self) -> usize {
        self.registry.list().len()
    }

    /// Shutdown all plugins
    pub async fn shutdown(&self) -> Result<()> {
        self.registry.shutdown_all().await?;
        tracing::info!("Plugin manager shut down");
        Ok(())
    }
}

/// Plugin info for listing
#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub plugin_type: PluginType,
    pub capabilities: Vec<Capability>,
    pub healthy: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = PluginManager::default_manager();
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_manager_plugin_count() {
        let manager = PluginManager::default_manager().unwrap();
        assert_eq!(manager.plugin_count(), 0);
    }

    #[tokio::test]
    async fn test_manager_list_empty() {
        let manager = PluginManager::default_manager().unwrap();
        let plugins = manager.list_plugins_with_health().await;
        assert!(plugins.is_empty());
    }

    #[tokio::test]
    async fn test_execute_nonexistent_plugin() {
        let manager = PluginManager::default_manager().unwrap();
        let result = manager.execute("nonexistent", &[]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fire_hook() {
        let manager = PluginManager::default_manager().unwrap();
        let hook_data = HookData::new("test_event");
        let result = manager.fire_hook(&hook_data).await;
        assert_eq!(result.handlers_executed, 0); // No handlers registered
    }

    #[tokio::test]
    async fn test_manager_config() {
        let config = PluginManagerConfig {
            max_concurrent: 5,
            default_timeout_ms: 10_000,
            ..Default::default()
        };
        let manager = PluginManager::new(config).unwrap();
        assert_eq!(manager.plugin_count(), 0);
    }
}
