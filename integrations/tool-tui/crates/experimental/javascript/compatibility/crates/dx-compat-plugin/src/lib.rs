//! # dx-compat-plugin
//!
//! Plugin system compatibility layer.
//!
//! Provides Bun/esbuild-compatible plugin API with:
//! - onLoad handlers for custom file loading
//! - onResolve handlers for custom module resolution
//! - Filter patterns for selective handling
//! - Namespace support for virtual modules

#![warn(missing_docs)]

mod builder;
mod error;
mod loader;

pub use builder::{
    Filter, ImportKind, OnLoadArgs, OnLoadHandler, OnLoadResult, OnResolveArgs, OnResolveHandler,
    OnResolveResult, PluginBuilder,
};
pub use error::{PluginError, PluginResult};
pub use loader::Loader;

use parking_lot::RwLock;
use std::sync::Arc;

/// Plugin definition.
pub struct Plugin {
    /// Plugin name
    pub name: String,
    /// The plugin builder with registered handlers
    pub builder: Arc<PluginBuilder>,
}

impl Plugin {
    /// Create a new plugin with a setup function.
    pub fn new<F>(name: impl Into<String>, setup: F) -> PluginResult<Self>
    where
        F: FnOnce(&PluginBuilder) -> PluginResult<()>,
    {
        let name = name.into();
        let builder = Arc::new(PluginBuilder::new(&name));
        setup(&builder)?;
        Ok(Self { name, builder })
    }

    /// Get the plugin name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Run onLoad handlers.
    pub fn run_on_load(&self, args: &OnLoadArgs) -> Option<OnLoadResult> {
        self.builder.run_on_load(args)
    }

    /// Run onResolve handlers.
    pub fn run_on_resolve(&self, args: &OnResolveArgs) -> Option<OnResolveResult> {
        self.builder.run_on_resolve(args)
    }
}

/// Plugin registry for managing multiple plugins.
pub struct PluginRegistry {
    plugins: RwLock<Vec<Arc<Plugin>>>,
}

impl PluginRegistry {
    /// Create a new plugin registry.
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(Vec::new()),
        }
    }

    /// Register a plugin.
    pub fn register(&self, plugin: Plugin) {
        self.plugins.write().push(Arc::new(plugin));
    }

    /// Run onLoad handlers from all plugins.
    pub fn run_on_load(&self, args: &OnLoadArgs) -> Option<OnLoadResult> {
        let plugins = self.plugins.read();
        for plugin in plugins.iter() {
            if let Some(result) = plugin.run_on_load(args) {
                return Some(result);
            }
        }
        None
    }

    /// Run onResolve handlers from all plugins.
    pub fn run_on_resolve(&self, args: &OnResolveArgs) -> Option<OnResolveResult> {
        let plugins = self.plugins.read();
        for plugin in plugins.iter() {
            if let Some(result) = plugin.run_on_resolve(args) {
                return Some(result);
            }
        }
        None
    }

    /// Get the number of registered plugins.
    pub fn plugin_count(&self) -> usize {
        self.plugins.read().len()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
