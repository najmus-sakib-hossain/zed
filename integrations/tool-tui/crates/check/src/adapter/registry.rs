//! Tool Registry for Managing Adapters
//!
//! The registry manages tool adapter discovery, caching, and selection.

use super::traits::ToolAdapter;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

/// Configuration for tool preferences
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolConfig {
    /// Custom tool paths (tool name -> path)
    #[serde(default)]
    pub tool_paths: HashMap<String, PathBuf>,

    /// Preference order for tools per extension (extension -> [tool names])
    #[serde(default)]
    pub preference_order: HashMap<String, Vec<String>>,

    /// Disabled tools (won't be used even if available)
    #[serde(default)]
    pub disabled_tools: HashSet<String>,

    /// Auto-discover tools in PATH
    #[serde(default = "default_true")]
    pub auto_discover: bool,

    /// Search paths in addition to PATH
    #[serde(default)]
    pub search_paths: Vec<PathBuf>,
}

fn default_true() -> bool {
    true
}

impl ToolConfig {
    /// Create a new default configuration
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a custom tool path
    pub fn with_tool_path(mut self, tool: impl Into<String>, path: PathBuf) -> Self {
        self.tool_paths.insert(tool.into(), path);
        self
    }

    /// Set preference order for an extension
    pub fn with_preference(mut self, ext: impl Into<String>, tools: Vec<String>) -> Self {
        self.preference_order.insert(ext.into(), tools);
        self
    }

    /// Disable a tool
    pub fn with_disabled(mut self, tool: impl Into<String>) -> Self {
        self.disabled_tools.insert(tool.into());
        self
    }
}

/// Cache entry for tool availability
#[derive(Debug, Clone)]
struct AvailabilityEntry {
    available: bool,
    version: Option<String>,
    path: Option<PathBuf>,
}

/// Registry for managing tool adapters
pub struct ToolRegistry {
    /// Registered adapters (tool name -> adapter)
    adapters: HashMap<&'static str, Arc<dyn ToolAdapter>>,

    /// Extension to tool mapping (extension -> [tool names in priority order])
    extension_map: HashMap<&'static str, Vec<&'static str>>,

    /// Cached availability checks (owned String keys for flexible lookups)
    availability_cache: RwLock<HashMap<String, AvailabilityEntry>>,

    /// Configuration
    config: ToolConfig,
}

impl ToolRegistry {
    /// Create a new registry with configuration
    #[must_use]
    pub fn new(config: ToolConfig) -> Self {
        Self {
            adapters: HashMap::new(),
            extension_map: HashMap::new(),
            availability_cache: RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Create a registry with default configuration and all built-in adapters
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut registry = Self::new(ToolConfig::default());
        registry.register_builtin_adapters();
        registry
    }

    /// Register a tool adapter
    pub fn register(&mut self, adapter: Arc<dyn ToolAdapter>) {
        let name = adapter.name();

        // Skip if disabled
        if self.config.disabled_tools.contains(name) {
            return;
        }

        // Register for all supported extensions
        for &ext in adapter.extensions() {
            self.extension_map.entry(ext).or_default().push(name);
        }

        self.adapters.insert(name, adapter);
    }

    /// Register all built-in adapters
    fn register_builtin_adapters(&mut self) {
        use super::tools::{
            ClangFormatAdapter, ClangTidyAdapter, ClippyAdapter, ESLintAdapter, GofmtAdapter,
            GolangciLintAdapter, KtlintAdapter, PhpCsAdapter, PrettierAdapter, RubocopAdapter,
            RuffAdapter, RustfmtAdapter,
        };

        // Rust tools
        self.register(Arc::new(RustfmtAdapter::new()));
        self.register(Arc::new(ClippyAdapter::new()));

        // Python tools
        self.register(Arc::new(RuffAdapter::new()));

        // Go tools
        self.register(Arc::new(GofmtAdapter::new()));
        self.register(Arc::new(GolangciLintAdapter::new()));

        // JavaScript/TypeScript tools
        self.register(Arc::new(PrettierAdapter::new()));
        self.register(Arc::new(ESLintAdapter::new()));

        // C/C++ tools
        self.register(Arc::new(ClangFormatAdapter::new()));
        self.register(Arc::new(ClangTidyAdapter::new()));

        // Other languages
        self.register(Arc::new(KtlintAdapter::new()));
        self.register(Arc::new(PhpCsAdapter::new()));
        self.register(Arc::new(RubocopAdapter::new()));
    }

    /// Get the best adapter for a file extension
    ///
    /// Returns the first available adapter based on:
    /// 1. User-configured preference order
    /// 2. Default priority order
    /// 3. First available adapter
    pub fn get_adapter_for_extension(&self, ext: &str) -> Option<&dyn ToolAdapter> {
        // Check user preference first
        if let Some(prefs) = self.config.preference_order.get(ext) {
            for tool_name in prefs {
                if let Some(adapter) = self.adapters.get(tool_name.as_str())
                    && self.is_tool_available(adapter.name())
                {
                    return Some(adapter.as_ref());
                }
            }
        }

        // Fall back to default order
        if let Some(tools) = self.extension_map.get(ext) {
            for &tool_name in tools {
                if let Some(adapter) = self.adapters.get(tool_name)
                    && self.is_tool_available(tool_name)
                {
                    return Some(adapter.as_ref());
                }
            }
        }

        None
    }

    /// Get an adapter by name
    pub fn get_adapter_by_name(&self, name: &str) -> Option<&dyn ToolAdapter> {
        self.adapters.get(name).map(std::convert::AsRef::as_ref)
    }

    /// Check if a tool is available (with caching)
    pub fn is_tool_available(&self, name: &str) -> bool {
        // Check cache first
        if let Some(entry) = self.availability_cache.read().get(name) {
            return entry.available;
        }

        // Check actual availability

        if let Some(adapter) = self.adapters.get(name) {
            let is_available = adapter.is_available();

            // Cache the result
            let entry = AvailabilityEntry {
                available: is_available,
                version: if is_available {
                    adapter.version()
                } else {
                    None
                },
                path: if is_available {
                    adapter.executable_path()
                } else {
                    None
                },
            };
            self.availability_cache.write().insert(name.to_string(), entry);

            is_available
        } else {
            false
        }
    }

    /// Get tool version (cached)
    pub fn get_tool_version(&self, name: &str) -> Option<String> {
        // Ensure availability is checked (populates cache)
        self.is_tool_available(name);

        self.availability_cache.read().get(name).and_then(|e| e.version.clone())
    }

    /// Get tool path (cached)
    pub fn get_tool_path(&self, name: &str) -> Option<PathBuf> {
        // Check custom config first
        if let Some(path) = self.config.tool_paths.get(name) {
            return Some(path.clone());
        }

        // Ensure availability is checked (populates cache)
        self.is_tool_available(name);

        self.availability_cache.read().get(name).and_then(|e| e.path.clone())
    }

    /// List all registered tools
    pub fn list_tools(&self) -> Vec<&'static str> {
        self.adapters.keys().copied().collect()
    }

    /// List all available tools
    pub fn list_available_tools(&self) -> Vec<&'static str> {
        self.adapters
            .keys()
            .filter(|&&name| self.is_tool_available(name))
            .copied()
            .collect()
    }

    /// List all supported extensions
    pub fn list_extensions(&self) -> Vec<&'static str> {
        self.extension_map.keys().copied().collect()
    }

    /// Refresh availability cache for all tools
    pub fn refresh_availability(&self) {
        self.availability_cache.write().clear();
        for name in self.adapters.keys() {
            self.is_tool_available(name);
        }
    }

    /// Get tool info for display
    pub fn get_tool_info(&self, name: &str) -> Option<ToolInfo> {
        self.adapters.get(name).map(|adapter| {
            let available = self.is_tool_available(name);
            ToolInfo {
                name: adapter.name(),
                extensions: adapter.extensions().to_vec(),
                capabilities: adapter.capabilities(),
                available,
                version: if available {
                    self.get_tool_version(name)
                } else {
                    None
                },
                path: if available {
                    self.get_tool_path(name)
                } else {
                    None
                },
                install_instructions: adapter.install_instructions(),
            }
        })
    }

    /// Get all tool infos
    pub fn get_all_tool_info(&self) -> Vec<ToolInfo> {
        self.adapters.keys().filter_map(|&name| self.get_tool_info(name)).collect()
    }
}

/// Information about a tool for display
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: &'static str,
    pub extensions: Vec<&'static str>,
    pub capabilities: super::traits::ToolCapabilities,
    pub available: bool,
    pub version: Option<String>,
    pub path: Option<PathBuf>,
    pub install_instructions: &'static str,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = ToolRegistry::new(ToolConfig::default());
        assert!(registry.list_tools().is_empty());
    }

    #[test]
    fn test_tool_config() {
        let config = ToolConfig::new()
            .with_tool_path("rustfmt", PathBuf::from("/custom/rustfmt"))
            .with_preference("rs", vec!["rustfmt".to_string()])
            .with_disabled("clippy");

        assert!(config.tool_paths.contains_key("rustfmt"));
        assert!(config.preference_order.contains_key("rs"));
        assert!(config.disabled_tools.contains("clippy"));
    }
}
