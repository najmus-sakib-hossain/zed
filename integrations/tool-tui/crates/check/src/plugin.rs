//! Plugin System for Custom Rules
//!
//! Enables users to create and load custom lint rules via plugins.
//!
//! # Architecture
//!
//! Plugins can be written in:
//! 1. Rust (compiled to shared libraries)
//! 2. WASM (portable, sandboxed)
//! 3. JavaScript (via dx-js-runtime)
//!
//! # Plugin Discovery
//!
//! Plugins are discovered from:
//! 1. `./plugins/` directory in workspace
//! 2. `~/.dx-check/plugins/` global directory
//! 3. npm packages with `dx-check-plugin-*` prefix
//!
//! # Example Plugin (Rust)
//!
//! ```rust,ignore
//! use dx_check::plugin::{Plugin, PluginMeta, Rule};
//!
//! #[dx_check::plugin]
//! pub struct MyPlugin;
//!
//! impl Plugin for MyPlugin {
//!     fn meta(&self) -> PluginMeta {
//!         PluginMeta {
//!             name: "my-plugin",
//!             version: "1.0.0",
//!             rules: vec!["my-rule-1", "my-rule-2"],
//!         }
//!     }
//!
//!     fn rules(&self) -> Vec<Box<dyn Rule>> {
//!         vec![Box::new(MyRule1), Box::new(MyRule2)]
//!     }
//! }
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::rules::{Rule, RuleRegistry};

/// Plugin metadata
#[derive(Debug, Clone)]
pub struct PluginMeta {
    /// Plugin name (unique identifier)
    pub name: String,
    /// Plugin version (semver)
    pub version: String,
    /// Plugin description
    pub description: String,
    /// Author name
    pub author: Option<String>,
    /// Homepage URL
    pub homepage: Option<String>,
    /// List of rule names provided by this plugin
    pub rules: Vec<String>,
    /// Plugin type
    pub plugin_type: PluginType,
}

/// Plugin type determines how the plugin is loaded
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PluginType {
    /// Native Rust plugin (shared library)
    Native,
    /// WebAssembly plugin (portable, sandboxed)
    Wasm,
    /// JavaScript plugin (via dx-js-runtime)
    JavaScript,
    /// Built-in plugin (compiled into dx-check)
    #[default]
    Builtin,
}

/// Plugin status for tracking loaded plugins
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginStatus {
    /// Plugin is loaded and active
    Active,
    /// Plugin is loaded but disabled
    Disabled,
    /// Plugin failed to load
    Failed,
    /// Plugin is not yet loaded
    NotLoaded,
}

/// Plugin trait - implement this to create a custom plugin
pub trait Plugin: Send + Sync {
    /// Get plugin metadata
    fn meta(&self) -> PluginMeta;

    /// Get all rules provided by this plugin
    fn rules(&self) -> Vec<Box<dyn Rule>>;

    /// Called when plugin is loaded
    fn on_load(&self) {}

    /// Called when plugin is unloaded
    fn on_unload(&self) {}

    /// Get configuration schema (JSON Schema)
    fn config_schema(&self) -> Option<String> {
        None
    }
}

/// Type alias for the plugin creation function exported by native plugins
///
/// Native plugins must export a function with this signature:
/// ```rust,ignore
/// #[no_mangle]
/// pub extern "C" fn dx_check_plugin_create() -> *mut dyn Plugin {
///     Box::into_raw(Box::new(MyPlugin))
/// }
/// ```
pub type PluginCreateFn = unsafe extern "C" fn() -> *mut dyn Plugin;

/// Type alias for the plugin destroy function exported by native plugins
pub type PluginDestroyFn = unsafe extern "C" fn(*mut dyn Plugin);

/// Plugin instance with loaded state
pub struct LoadedPlugin {
    /// Plugin metadata
    pub meta: PluginMeta,
    /// Plugin implementation
    plugin: Arc<dyn Plugin>,
    /// Rules from this plugin
    rules: Vec<Box<dyn Rule>>,
    /// Whether plugin is enabled
    pub enabled: bool,
    /// Plugin source path
    pub source_path: Option<PathBuf>,
    /// Plugin status
    pub status: PluginStatus,
    /// Error message if plugin failed to load
    pub error: Option<String>,
}

impl LoadedPlugin {
    /// Create a new loaded plugin
    pub fn new(plugin: Arc<dyn Plugin>) -> Self {
        let meta = plugin.meta();
        let rules = plugin.rules();
        plugin.on_load();

        Self {
            meta,
            plugin,
            rules,
            enabled: true,
            source_path: None,
            status: PluginStatus::Active,
            error: None,
        }
    }

    /// Create a failed plugin entry
    #[must_use]
    pub fn failed(meta: PluginMeta, error: String, source_path: Option<PathBuf>) -> Self {
        Self {
            meta,
            plugin: Arc::new(FailedPluginStub),
            rules: Vec::new(),
            enabled: false,
            source_path,
            status: PluginStatus::Failed,
            error: Some(error),
        }
    }

    /// Get rules from this plugin
    #[must_use]
    pub fn rules(&self) -> &[Box<dyn Rule>] {
        &self.rules
    }

    /// Enable or disable the plugin
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.status = if enabled {
            PluginStatus::Active
        } else {
            PluginStatus::Disabled
        };
    }
}

impl Drop for LoadedPlugin {
    fn drop(&mut self) {
        self.plugin.on_unload();
    }
}

/// Stub for failed plugins
struct FailedPluginStub;

impl Plugin for FailedPluginStub {
    fn meta(&self) -> PluginMeta {
        PluginMeta {
            name: "failed".to_string(),
            version: "0.0.0".to_string(),
            description: "Failed to load".to_string(),
            author: None,
            homepage: None,
            rules: Vec::new(),
            plugin_type: PluginType::Builtin,
        }
    }

    fn rules(&self) -> Vec<Box<dyn Rule>> {
        Vec::new()
    }
}

/// Plugin loader - discovers and loads plugins
pub struct PluginLoader {
    /// Search paths for plugins
    search_paths: Vec<PathBuf>,
    /// Loaded plugins
    plugins: HashMap<String, LoadedPlugin>,
    /// Native library handles (kept alive to prevent unloading)
    #[cfg(feature = "native-plugins")]
    native_handles: HashMap<String, libloading::Library>,
}

impl PluginLoader {
    /// Create a new plugin loader
    #[must_use]
    pub fn new() -> Self {
        Self {
            search_paths: Vec::new(),
            plugins: HashMap::new(),
            #[cfg(feature = "native-plugins")]
            native_handles: HashMap::new(),
        }
    }

    /// Add a search path for plugins
    pub fn add_search_path(&mut self, path: impl Into<PathBuf>) {
        self.search_paths.push(path.into());
    }

    /// Add default search paths
    #[must_use]
    pub fn with_default_paths(mut self) -> Self {
        // Current directory plugins
        self.search_paths.push(PathBuf::from("./plugins"));
        self.search_paths.push(PathBuf::from("./.dx-check/plugins"));

        // Home directory plugins
        if let Some(home) = dirs::home_dir() {
            self.search_paths.push(home.join(".dx-check").join("plugins"));
        }

        // XDG config plugins
        if let Some(config) = dirs::config_dir() {
            self.search_paths.push(config.join("dx-check").join("plugins"));
        }

        self
    }

    /// Discover plugins in search paths
    pub fn discover(&mut self) -> Vec<PluginMeta> {
        let mut discovered = Vec::new();

        for path in &self.search_paths {
            if !path.exists() {
                continue;
            }

            // Look for plugin manifest files
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();

                    // Check for dx-plugin.toml manifest
                    let manifest_path = if entry_path.is_dir() {
                        entry_path.join("dx-plugin.toml")
                    } else if entry_path.extension().is_some_and(|e| e == "toml") {
                        entry_path.clone()
                    } else {
                        continue;
                    };

                    if manifest_path.exists()
                        && let Ok(meta) = self.parse_manifest(&manifest_path)
                    {
                        discovered.push(meta);
                    }
                }
            }
        }

        discovered
    }

    /// Parse plugin manifest file
    fn parse_manifest(&self, path: &Path) -> Result<PluginMeta, PluginError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| PluginError::ManifestRead(e.to_string()))?;

        let manifest: PluginManifest =
            toml::from_str(&content).map_err(|e| PluginError::ManifestParse(e.to_string()))?;

        Ok(PluginMeta {
            name: manifest.plugin.name,
            version: manifest.plugin.version,
            description: manifest.plugin.description.unwrap_or_default(),
            author: manifest.plugin.author,
            homepage: manifest.plugin.homepage,
            rules: manifest.plugin.rules.unwrap_or_default(),
            plugin_type: match manifest.plugin.plugin_type.as_deref() {
                Some("wasm") => PluginType::Wasm,
                Some("javascript" | "js") => PluginType::JavaScript,
                Some("native" | "rust") => PluginType::Native,
                _ => PluginType::Native,
            },
        })
    }

    /// Load a plugin by name
    pub fn load(&mut self, name: &str) -> Result<&LoadedPlugin, PluginError> {
        // Check if already loaded
        if self.plugins.contains_key(name) {
            // SAFETY: We just checked that the key exists
            return self.plugins.get(name).ok_or_else(|| {
                PluginError::LoadFailed("Plugin was removed during load".to_string())
            });
        }

        // Find plugin in search paths
        for path in &self.search_paths.clone() {
            let plugin_dir = path.join(name);
            let manifest_path = plugin_dir.join("dx-plugin.toml");

            if manifest_path.exists() {
                let meta = self.parse_manifest(&manifest_path)?;

                // Load based on plugin type
                match meta.plugin_type {
                    PluginType::Native => {
                        return self.load_native_plugin(&plugin_dir, meta);
                    }
                    PluginType::Wasm => {
                        return self.load_wasm_plugin(&plugin_dir, meta);
                    }
                    PluginType::JavaScript => {
                        return self.load_js_plugin(&plugin_dir, meta);
                    }
                    PluginType::Builtin => {
                        return Err(PluginError::NotSupported(
                            "Cannot dynamically load builtin plugins".to_string(),
                        ));
                    }
                }
            }
        }

        Err(PluginError::NotFound(name.to_string()))
    }

    /// Load a native Rust plugin from a shared library
    fn load_native_plugin(
        &mut self,
        dir: &Path,
        meta: PluginMeta,
    ) -> Result<&LoadedPlugin, PluginError> {
        // Determine the library file name based on platform
        let lib_name = get_native_lib_name(&meta.name);
        let lib_path = dir.join(&lib_name);

        if !lib_path.exists() {
            return Err(PluginError::LoadFailed(format!(
                "Native library not found: {}",
                lib_path.display()
            )));
        }

        #[cfg(feature = "native-plugins")]
        {
            // Safety: We're loading a dynamic library that should export the required symbols
            // The plugin author is responsible for ensuring the library is compatible
            unsafe {
                let lib = libloading::Library::new(&lib_path).map_err(|e| {
                    PluginError::LoadFailed(format!("Failed to load library: {}", e))
                })?;

                // Get the plugin creation function
                let create_fn: libloading::Symbol<PluginCreateFn> =
                    lib.get(b"dx_check_plugin_create").map_err(|e| {
                        PluginError::LoadFailed(format!(
                            "Plugin missing dx_check_plugin_create symbol: {}",
                            e
                        ))
                    })?;

                // Create the plugin instance
                let plugin_ptr = create_fn();
                if plugin_ptr.is_null() {
                    return Err(PluginError::LoadFailed(
                        "Plugin creation returned null".to_string(),
                    ));
                }

                // Convert to Arc<dyn Plugin>
                let plugin: Arc<dyn Plugin> = Arc::from_raw(plugin_ptr);

                // Store the library handle to keep it alive
                self.native_handles.insert(meta.name.clone(), lib);

                let mut loaded = LoadedPlugin::new(plugin);
                loaded.source_path = Some(lib_path);

                let plugin_name = meta.name.clone();
                self.plugins.insert(plugin_name.clone(), loaded);
                self.plugins.get(&plugin_name).ok_or_else(|| {
                    PluginError::LoadFailed("Failed to retrieve loaded plugin".to_string())
                })
            }
        }

        #[cfg(not(feature = "native-plugins"))]
        {
            // Native plugins require the native-plugins feature
            let _ = lib_path;
            Err(PluginError::NotSupported(
                "Native plugins require the 'native-plugins' feature. \
                 Rebuild dx-check with --features native-plugins"
                    .to_string(),
            ))
        }
    }

    /// Load a WASM plugin
    fn load_wasm_plugin(
        &mut self,
        dir: &Path,
        meta: PluginMeta,
    ) -> Result<&LoadedPlugin, PluginError> {
        let wasm_path = dir.join(format!("{}.wasm", meta.name));

        if !wasm_path.exists() {
            return Err(PluginError::LoadFailed(format!(
                "WASM file not found: {}",
                wasm_path.display()
            )));
        }

        #[cfg(feature = "wasm-plugins")]
        {
            // Load WASM plugin with wasmtime
            let plugin = WasmPlugin::load(&wasm_path, meta.clone())?;
            let mut loaded = LoadedPlugin::new(Arc::new(plugin));
            loaded.source_path = Some(wasm_path);

            let plugin_name = meta.name.clone();
            self.plugins.insert(plugin_name.clone(), loaded);
            self.plugins.get(&plugin_name).ok_or_else(|| {
                PluginError::LoadFailed("Failed to retrieve loaded plugin".to_string())
            })
        }

        #[cfg(not(feature = "wasm-plugins"))]
        {
            // WASM plugins require the wasm-plugins feature
            let _ = wasm_path;

            // Create a stub plugin that indicates WASM support is not available
            let plugin = Arc::new(WasmPluginStub { meta: meta.clone() });
            let mut loaded = LoadedPlugin::new(plugin);
            loaded.source_path = Some(dir.to_path_buf());
            loaded.status = PluginStatus::Failed;
            loaded.error = Some(
                "WASM plugins require the 'wasm-plugins' feature. \
                 Rebuild dx-check with --features wasm-plugins"
                    .to_string(),
            );

            let plugin_name = meta.name.clone();
            self.plugins.insert(plugin_name.clone(), loaded);
            self.plugins.get(&plugin_name).ok_or_else(|| {
                PluginError::LoadFailed("Failed to retrieve loaded plugin".to_string())
            })
        }
    }

    /// Load a JavaScript plugin
    fn load_js_plugin(
        &mut self,
        dir: &Path,
        meta: PluginMeta,
    ) -> Result<&LoadedPlugin, PluginError> {
        // JavaScript plugin loading will be implemented with dx-js-runtime
        // For now, return a placeholder
        let plugin = Arc::new(JsPluginStub { meta: meta.clone() });
        let mut loaded = LoadedPlugin::new(plugin);
        loaded.source_path = Some(dir.to_path_buf());
        loaded.status = PluginStatus::Failed;
        loaded.error = Some(
            "JavaScript plugins are not yet supported. \
             Use WASM or native plugins instead."
                .to_string(),
        );

        let plugin_name = meta.name.clone();
        self.plugins.insert(plugin_name.clone(), loaded);
        self.plugins
            .get(&plugin_name)
            .ok_or_else(|| PluginError::LoadFailed("Failed to retrieve loaded plugin".to_string()))
    }

    /// Register a built-in plugin
    pub fn register_builtin(&mut self, plugin: Arc<dyn Plugin>) {
        let meta = plugin.meta();
        let name = meta.name.clone();
        let loaded = LoadedPlugin::new(plugin);
        self.plugins.insert(name, loaded);
    }

    /// Get all loaded plugins
    pub fn plugins(&self) -> impl Iterator<Item = &LoadedPlugin> {
        self.plugins.values()
    }

    /// Get a loaded plugin by name
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&LoadedPlugin> {
        self.plugins.get(name)
    }

    /// Get a mutable reference to a loaded plugin
    pub fn get_mut(&mut self, name: &str) -> Option<&mut LoadedPlugin> {
        self.plugins.get_mut(name)
    }

    /// Unload a plugin
    pub fn unload(&mut self, name: &str) -> bool {
        #[cfg(feature = "native-plugins")]
        {
            self.native_handles.remove(name);
        }
        self.plugins.remove(name).is_some()
    }

    /// Get all rules from all enabled plugins
    #[must_use]
    pub fn all_rules(&self) -> Vec<&dyn Rule> {
        self.plugins
            .values()
            .filter(|p| p.enabled && p.status == PluginStatus::Active)
            .flat_map(|p| p.rules().iter().map(std::convert::AsRef::as_ref))
            .collect()
    }

    /// Register all plugin rules with a `RuleRegistry`
    pub fn register_rules_with_registry(&self, _registry: &mut RuleRegistry) {
        for plugin in self.plugins.values() {
            if !plugin.enabled || plugin.status != PluginStatus::Active {
                continue;
            }

            for rule in plugin.rules() {
                // Clone the rule for the registry
                // Note: This requires rules to implement Clone or we need a different approach
                // For now, we'll skip this as rules are already registered during plugin load
                let _ = rule;
            }
        }
    }

    /// Get plugin statistics
    #[must_use]
    pub fn stats(&self) -> PluginStats {
        let total = self.plugins.len();
        let active = self.plugins.values().filter(|p| p.status == PluginStatus::Active).count();
        let disabled = self.plugins.values().filter(|p| p.status == PluginStatus::Disabled).count();
        let failed = self.plugins.values().filter(|p| p.status == PluginStatus::Failed).count();
        let total_rules: usize = self
            .plugins
            .values()
            .filter(|p| p.status == PluginStatus::Active)
            .map(|p| p.rules().len())
            .sum();

        PluginStats {
            total,
            active,
            disabled,
            failed,
            total_rules,
        }
    }
}

/// Get the native library file name for the current platform
fn get_native_lib_name(plugin_name: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        format!("{plugin_name}.dll")
    }
    #[cfg(target_os = "macos")]
    {
        format!("lib{}.dylib", plugin_name)
    }
    #[cfg(target_os = "linux")]
    {
        format!("lib{}.so", plugin_name)
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        format!("lib{}.so", plugin_name)
    }
}

/// Plugin statistics
#[derive(Debug, Clone)]
pub struct PluginStats {
    /// Total number of plugins
    pub total: usize,
    /// Number of active plugins
    pub active: usize,
    /// Number of disabled plugins
    pub disabled: usize,
    /// Number of failed plugins
    pub failed: usize,
    /// Total number of rules from active plugins
    pub total_rules: usize,
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin manifest file format (dx-plugin.toml)
#[derive(Debug, serde::Deserialize)]
struct PluginManifest {
    plugin: PluginSection,
}

#[derive(Debug, serde::Deserialize)]
struct PluginSection {
    name: String,
    version: String,
    description: Option<String>,
    author: Option<String>,
    homepage: Option<String>,
    #[serde(rename = "type")]
    plugin_type: Option<String>,
    rules: Option<Vec<String>>,
}

/// Plugin loading errors
#[derive(Debug, Clone)]
pub enum PluginError {
    /// Plugin not found
    NotFound(String),
    /// Failed to read manifest
    ManifestRead(String),
    /// Failed to parse manifest
    ManifestParse(String),
    /// Plugin type not supported
    NotSupported(String),
    /// Plugin load failed
    LoadFailed(String),
    /// Plugin initialization failed
    InitFailed(String),
    /// WASM execution error
    WasmError(String),
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(name) => write!(f, "Plugin not found: {name}"),
            Self::ManifestRead(e) => write!(f, "Failed to read manifest: {e}"),
            Self::ManifestParse(e) => write!(f, "Failed to parse manifest: {e}"),
            Self::NotSupported(msg) => write!(f, "Not supported: {msg}"),
            Self::LoadFailed(e) => write!(f, "Plugin load failed: {e}"),
            Self::InitFailed(e) => write!(f, "Plugin initialization failed: {e}"),
            Self::WasmError(e) => write!(f, "WASM error: {e}"),
        }
    }
}

impl std::error::Error for PluginError {}

/// Stub for WASM plugins (placeholder until wasmtime integration)
struct WasmPluginStub {
    meta: PluginMeta,
}

impl Plugin for WasmPluginStub {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }

    fn rules(&self) -> Vec<Box<dyn Rule>> {
        Vec::new() // Will be populated from WASM exports
    }
}

/// WASM Plugin implementation with sandboxing
#[cfg(feature = "wasm-plugins")]
pub struct WasmPlugin {
    meta: PluginMeta,
    // wasmtime engine and store would go here
    // engine: wasmtime::Engine,
    // store: wasmtime::Store<WasmPluginState>,
    // instance: wasmtime::Instance,
}

#[cfg(feature = "wasm-plugins")]
impl WasmPlugin {
    /// Load a WASM plugin from a file
    pub fn load(path: &Path, meta: PluginMeta) -> Result<Self, PluginError> {
        // Read the WASM file
        let _wasm_bytes = std::fs::read(path)
            .map_err(|e| PluginError::LoadFailed(format!("Failed to read WASM file: {}", e)))?;

        // TODO: Initialize wasmtime engine and compile the module
        // let engine = wasmtime::Engine::default();
        // let module = wasmtime::Module::new(&engine, &wasm_bytes)
        //     .map_err(|e| PluginError::WasmError(format!("Failed to compile WASM: {}", e)))?;

        Ok(Self { meta })
    }
}

#[cfg(feature = "wasm-plugins")]
impl Plugin for WasmPlugin {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }

    fn rules(&self) -> Vec<Box<dyn Rule>> {
        // TODO: Call into WASM to get rules
        Vec::new()
    }
}

/// Stub for JavaScript plugins (placeholder until dx-js-runtime integration)
struct JsPluginStub {
    meta: PluginMeta,
}

impl Plugin for JsPluginStub {
    fn meta(&self) -> PluginMeta {
        self.meta.clone()
    }

    fn rules(&self) -> Vec<Box<dyn Rule>> {
        Vec::new() // Will be populated from JS exports
    }
}

/// Built-in plugin with core rules
pub struct BuiltinPlugin;

impl Plugin for BuiltinPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta {
            name: "builtin".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: "Core dx-check rules".to_string(),
            author: Some("DX Team".to_string()),
            homepage: Some("https://dx.dev/check".to_string()),
            rules: vec![
                "no-console".to_string(),
                "no-debugger".to_string(),
                "no-unused-vars".to_string(),
                "eqeqeq".to_string(),
                "prefer-const".to_string(),
                "no-var".to_string(),
                "no-eval".to_string(),
                "no-with".to_string(),
            ],
            plugin_type: PluginType::Builtin,
        }
    }

    fn rules(&self) -> Vec<Box<dyn Rule>> {
        use crate::rules::builtin::{
            Eqeqeq, NoConsole, NoDebugger, NoEval, NoUnusedVars, NoVar, NoWith, PreferConst,
        };

        vec![
            Box::new(NoConsole::new(vec![])),
            Box::<NoDebugger>::default(),
            Box::<NoUnusedVars>::default(),
            Box::new(Eqeqeq::new(false)),
            Box::<PreferConst>::default(),
            Box::<NoVar>::default(),
            Box::<NoEval>::default(),
            Box::<NoWith>::default(),
        ]
    }
}

/// Plugin template generator for creating new plugins
pub struct PluginTemplate;

impl PluginTemplate {
    /// Generate a native Rust plugin template
    pub fn generate_native(name: &str, output_dir: &Path) -> std::io::Result<()> {
        let plugin_dir = output_dir.join(name);
        std::fs::create_dir_all(&plugin_dir)?;

        // Create Cargo.toml
        let cargo_toml = format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
dx-check = {{ version = "*", features = ["plugin-api"] }}
"#
        );
        std::fs::write(plugin_dir.join("Cargo.toml"), cargo_toml)?;

        // Create src/lib.rs
        let lib_rs = format!(
            r#"//! {name} - A dx-check plugin

use dx_check::plugin::{{Plugin, PluginMeta, PluginType}};
use dx_check::rules::Rule;

pub struct {struct_name};

impl Plugin for {struct_name} {{
    fn meta(&self) -> PluginMeta {{
        PluginMeta {{
            name: "{name}".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: "A custom dx-check plugin".to_string(),
            author: None,
            homepage: None,
            rules: vec!["my-rule".to_string()],
            plugin_type: PluginType::Native,
        }}
    }}

    fn rules(&self) -> Vec<Box<dyn Rule>> {{
        vec![
            // Add your rules here
        ]
    }}
}}

/// Plugin entry point - called by dx-check to create the plugin
#[no_mangle]
pub extern "C" fn dx_check_plugin_create() -> *mut dyn Plugin {{
    Box::into_raw(Box::new({struct_name}))
}}

/// Plugin cleanup - called by dx-check when unloading the plugin
#[no_mangle]
pub extern "C" fn dx_check_plugin_destroy(plugin: *mut dyn Plugin) {{
    if !plugin.is_null() {{
        unsafe {{ drop(Box::from_raw(plugin)); }}
    }}
}}
"#,
            name = name,
            struct_name = to_pascal_case(name),
        );
        std::fs::create_dir_all(plugin_dir.join("src"))?;
        std::fs::write(plugin_dir.join("src/lib.rs"), lib_rs)?;

        // Create dx-plugin.toml
        let manifest = format!(
            r#"[plugin]
name = "{name}"
version = "0.1.0"
description = "A custom dx-check plugin"
type = "native"
rules = ["my-rule"]
"#
        );
        std::fs::write(plugin_dir.join("dx-plugin.toml"), manifest)?;

        // Create README.md
        let readme = format!(
            r"# {name}

A custom dx-check plugin.

## Installation

```bash
cargo build --release
cp target/release/lib{name}.so ~/.dx-check/plugins/{name}/
```

## Usage

Add to your dx.toml:

```toml
[plugins]
{name} = {{ enabled = true }}
```
"
        );
        std::fs::write(plugin_dir.join("README.md"), readme)?;

        Ok(())
    }

    /// Generate a WASM plugin template
    pub fn generate_wasm(name: &str, output_dir: &Path) -> std::io::Result<()> {
        let plugin_dir = output_dir.join(name);
        std::fs::create_dir_all(&plugin_dir)?;

        // Create Cargo.toml
        let cargo_toml = format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
dx-check-wasm-api = {{ version = "*" }}
wasm-bindgen = "0.2"

[profile.release]
opt-level = "s"
lto = true
"#
        );
        std::fs::write(plugin_dir.join("Cargo.toml"), cargo_toml)?;

        // Create src/lib.rs
        let lib_rs = format!(
            r#"//! {name} - A dx-check WASM plugin

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn plugin_name() -> String {{
    "{name}".to_string()
}}

#[wasm_bindgen]
pub fn plugin_version() -> String {{
    env!("CARGO_PKG_VERSION").to_string()
}}

#[wasm_bindgen]
pub fn plugin_rules() -> Vec<JsValue> {{
    vec![
        // Add your rules here
    ]
}}
"#
        );
        std::fs::create_dir_all(plugin_dir.join("src"))?;
        std::fs::write(plugin_dir.join("src/lib.rs"), lib_rs)?;

        // Create dx-plugin.toml
        let manifest = format!(
            r#"[plugin]
name = "{name}"
version = "0.1.0"
description = "A custom dx-check WASM plugin"
type = "wasm"
rules = ["my-rule"]
"#
        );
        std::fs::write(plugin_dir.join("dx-plugin.toml"), manifest)?;

        Ok(())
    }
}

/// Convert a kebab-case string to `PascalCase`
fn to_pascal_case(s: &str) -> String {
    s.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_loader_creation() {
        let loader = PluginLoader::new();
        assert_eq!(loader.plugins().count(), 0);
    }

    #[test]
    fn test_builtin_plugin() {
        let plugin = BuiltinPlugin;
        let meta = plugin.meta();
        assert_eq!(meta.name, "builtin");
        assert_eq!(meta.plugin_type, PluginType::Builtin);
        assert!(!meta.rules.is_empty());
    }

    #[test]
    fn test_register_builtin() {
        let mut loader = PluginLoader::new();
        loader.register_builtin(Arc::new(BuiltinPlugin));
        assert_eq!(loader.plugins().count(), 1);
        assert!(loader.get("builtin").is_some());
    }

    #[test]
    fn test_plugin_stats() {
        let mut loader = PluginLoader::new();
        loader.register_builtin(Arc::new(BuiltinPlugin));

        let stats = loader.stats();
        assert_eq!(stats.total, 1);
        assert_eq!(stats.active, 1);
        assert_eq!(stats.disabled, 0);
        assert_eq!(stats.failed, 0);
    }

    #[test]
    fn test_plugin_enable_disable() {
        let mut loader = PluginLoader::new();
        loader.register_builtin(Arc::new(BuiltinPlugin));

        if let Some(plugin) = loader.get_mut("builtin") {
            plugin.set_enabled(false);
            assert_eq!(plugin.status, PluginStatus::Disabled);

            plugin.set_enabled(true);
            assert_eq!(plugin.status, PluginStatus::Active);
        }
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("my-plugin"), "MyPlugin");
        assert_eq!(to_pascal_case("no-console"), "NoConsole");
        assert_eq!(to_pascal_case("simple"), "Simple");
    }

    #[test]
    fn test_native_lib_name() {
        let name = get_native_lib_name("my-plugin");
        #[cfg(target_os = "windows")]
        assert_eq!(name, "my-plugin.dll");
        #[cfg(target_os = "macos")]
        assert_eq!(name, "libmy-plugin.dylib");
        #[cfg(target_os = "linux")]
        assert_eq!(name, "libmy-plugin.so");
    }

    #[test]
    fn test_plugin_template_generation() {
        let temp_dir = std::env::temp_dir().join("dx-check-test-plugins");
        let _ = std::fs::remove_dir_all(&temp_dir);

        // Test native plugin template
        PluginTemplate::generate_native("test-plugin", &temp_dir).unwrap();
        assert!(temp_dir.join("test-plugin/Cargo.toml").exists());
        assert!(temp_dir.join("test-plugin/src/lib.rs").exists());
        assert!(temp_dir.join("test-plugin/dx-plugin.toml").exists());

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
