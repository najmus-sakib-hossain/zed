//! # Plugin System
//!
//! Load and manage WASM plugins for dynamic integrations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;

use crate::{wasm_runtime::WasmRuntime, Result};

/// Plugin manifest in DX Serializer format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin name
    pub name: String,

    /// Version
    pub version: String,

    /// Description
    pub description: String,

    /// Original language (python, javascript, rust, etc.)
    pub language: String,

    /// Plugin type (integration, skill, tool)
    pub plugin_type: String,

    /// Exported functions
    pub exports: Vec<String>,

    /// Required capabilities
    pub capabilities: Vec<String>,

    /// Author
    pub author: Option<String>,
}

impl PluginManifest {
    /// Parse from DX format
    pub fn from_dx(dx: &str) -> Result<Self> {
        let mut manifest = Self {
            name: String::new(),
            version: "0.0.1".to_string(),
            description: String::new(),
            language: "wasm".to_string(),
            plugin_type: "integration".to_string(),
            exports: vec![],
            capabilities: vec![],
            author: None,
        };

        for part in dx.split_whitespace() {
            if let Some((key, value)) = part.split_once('=') {
                match key {
                    "name" => manifest.name = value.to_string(),
                    "version" => manifest.version = value.to_string(),
                    "language" => manifest.language = value.to_string(),
                    "type" => manifest.plugin_type = value.to_string(),
                    _ => {}
                }
            }
        }

        Ok(manifest)
    }

    /// Convert to DX format
    pub fn to_dx(&self) -> String {
        format!(
            "name={} version={} language={} type={} exports[{}]={}",
            self.name,
            self.version,
            self.language,
            self.plugin_type,
            self.exports.len(),
            self.exports.join(" ")
        )
    }
}

/// A loaded plugin
pub struct Plugin {
    manifest: PluginManifest,
    wasm_path: PathBuf,
    loaded: bool,
}

impl Plugin {
    pub fn new(manifest: PluginManifest, wasm_path: PathBuf) -> Self {
        Self {
            manifest,
            wasm_path,
            loaded: false,
        }
    }

    pub fn name(&self) -> &str {
        &self.manifest.name
    }

    pub fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    pub fn set_loaded(&mut self, loaded: bool) {
        self.loaded = loaded;
    }
}

/// Plugin loader and manager
pub struct PluginLoader {
    plugins: HashMap<String, Plugin>,
    plugins_path: PathBuf,
    wasm_runtime: Arc<WasmRuntime>,
}

impl PluginLoader {
    pub async fn new(plugins_path: &Path, wasm_runtime: Arc<WasmRuntime>) -> Result<Self> {
        std::fs::create_dir_all(plugins_path)?;

        Ok(Self {
            plugins: HashMap::new(),
            plugins_path: plugins_path.to_path_buf(),
            wasm_runtime,
        })
    }

    /// Load all plugins from the plugins directory
    pub async fn load_all(&mut self) -> Result<usize> {
        let mut count = 0;

        if let Ok(entries) = std::fs::read_dir(&self.plugins_path) {
            for entry in entries.flatten() {
                let path = entry.path();

                // Load .wasm files with accompanying .sr manifest
                if path.extension().is_some_and(|e| e == "wasm") {
                    let manifest_path = path.with_extension("sr");
                    if manifest_path.exists() {
                        if let Ok(plugin) = self.load_from_path(&path, &manifest_path).await {
                            let name = plugin.name().to_string();
                            self.plugins.insert(name, plugin);
                            count += 1;
                        }
                    }
                }
            }
        }

        info!("Loaded {} plugins", count);
        Ok(count)
    }

    /// Load a plugin from file paths
    async fn load_from_path(&self, wasm_path: &Path, manifest_path: &Path) -> Result<Plugin> {
        let manifest_content = std::fs::read_to_string(manifest_path)?;
        let manifest = PluginManifest::from_dx(&manifest_content)?;

        // Load the WASM module
        self.wasm_runtime
            .load_module_from_file(&manifest.name, wasm_path)
            .await?;

        let mut plugin = Plugin::new(manifest, wasm_path.to_path_buf());
        plugin.set_loaded(true);

        info!("Loaded plugin: {}", plugin.name());
        Ok(plugin)
    }

    /// Load a plugin from bytes (for dynamically created plugins)
    pub async fn load_from_bytes(
        &mut self,
        name: &str,
        wasm_bytes: &[u8],
        manifest_dx: &str,
    ) -> Result<()> {
        let manifest = PluginManifest::from_dx(manifest_dx)?;

        // Save the WASM file
        let wasm_path = self.plugins_path.join(format!("{}.wasm", name));
        std::fs::write(&wasm_path, wasm_bytes)?;

        // Save the manifest
        let manifest_path = self.plugins_path.join(format!("{}.sr", name));
        std::fs::write(&manifest_path, manifest_dx)?;

        // Load the module
        self.wasm_runtime.load_module(name, wasm_bytes).await?;

        let mut plugin = Plugin::new(manifest, wasm_path);
        plugin.set_loaded(true);

        self.plugins.insert(name.to_string(), plugin);

        info!("Plugin {} loaded from bytes", name);
        Ok(())
    }

    /// Get a plugin by name
    pub fn get(&self, name: &str) -> Option<&Plugin> {
        self.plugins.get(name)
    }

    /// List all loaded plugins
    pub fn list(&self) -> Vec<&str> {
        self.plugins.keys().map(|s| s.as_str()).collect()
    }

    /// Unload a plugin
    pub fn unload(&mut self, name: &str) -> Result<()> {
        self.plugins.remove(name);
        info!("Plugin {} unloaded", name);
        Ok(())
    }
}
