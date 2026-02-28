//! Scoring Plugin System
//!
//! Provides a trait-based plugin architecture for adding scoring modules
//! without modifying core code. Supports compile-time feature gates.

use crate::scoring_impl::{Category, Severity, Violation};
use std::any::Any;
use std::path::Path;

/// Plugin trait for scoring modules
///
/// Implement this trait to create a custom scoring plugin.
/// Plugins analyze code and produce violations that affect the 0-500 score.
pub trait ScoringPlugin: Send + Sync {
    /// Returns the plugin identifier (e.g., "security", "patterns")
    fn id(&self) -> &'static str;

    /// Returns the display name
    fn name(&self) -> &'static str;

    /// Returns the primary scoring category
    fn category(&self) -> Category;

    /// Analyze a file and return violations
    ///
    /// # Arguments
    /// * `path` - Path to the file
    /// * `content` - File content as bytes
    /// * `ast` - Optional parsed AST (if available)
    fn analyze(&self, path: &Path, content: &[u8], ast: Option<&dyn Any>) -> Vec<Violation>;

    /// List all rules this plugin provides
    fn rules(&self) -> &[RuleDefinition];

    /// Check if the plugin supports a file extension
    fn supports_extension(&self, ext: &str) -> bool {
        // Default: support all extensions
        let _ = ext;
        true
    }

    /// Get plugin version
    fn version(&self) -> &'static str {
        "1.0.0"
    }

    /// Get plugin description
    fn description(&self) -> &'static str {
        ""
    }
}

/// Definition of a rule provided by a plugin
#[derive(Debug, Clone)]
pub struct RuleDefinition {
    /// Rule ID (e.g., "security/sql-injection")
    pub id: String,
    /// Short description
    pub name: String,
    /// Long description
    pub description: String,
    /// Default severity
    pub severity: Severity,
    /// Category
    pub category: Category,
    /// Is this rule enabled by default?
    pub default_enabled: bool,
    /// Tags for filtering
    pub tags: Vec<String>,
}

impl RuleDefinition {
    /// Create a new rule definition
    pub fn new(id: impl Into<String>, name: impl Into<String>, category: Category) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            severity: Severity::Medium,
            category,
            default_enabled: true,
            tags: vec![],
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set severity
    #[must_use]
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Add tags
    #[must_use]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Set default enabled
    #[must_use]
    pub fn with_default_enabled(mut self, enabled: bool) -> Self {
        self.default_enabled = enabled;
        self
    }
}

/// Plugin registry for managing scoring plugins
pub struct PluginRegistry {
    plugins: Vec<Box<dyn ScoringPlugin>>,
    enabled_rules: std::collections::HashSet<String>,
    disabled_rules: std::collections::HashSet<String>,
}

impl PluginRegistry {
    /// Create a new empty registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            enabled_rules: std::collections::HashSet::new(),
            disabled_rules: std::collections::HashSet::new(),
        }
    }

    /// Create registry with all built-in plugins
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register_builtin_plugins();
        registry
    }

    /// Register a plugin
    pub fn register(&mut self, plugin: Box<dyn ScoringPlugin>) {
        self.plugins.push(plugin);
    }

    /// Register all built-in plugins based on features
    fn register_builtin_plugins(&mut self) {
        // Always register the built-in plugins
        self.register(Box::new(super::plugins::SecurityPlugin::new()));
        self.register(Box::new(super::plugins::PatternsPlugin::new()));
        self.register(Box::new(super::plugins::StructurePlugin::new()));
    }

    /// Get all registered plugins
    #[must_use]
    pub fn plugins(&self) -> &[Box<dyn ScoringPlugin>] {
        &self.plugins
    }

    /// Get plugins for a specific category
    #[must_use]
    pub fn plugins_for_category(&self, category: Category) -> Vec<&dyn ScoringPlugin> {
        self.plugins
            .iter()
            .filter(|p| p.category() == category)
            .map(std::convert::AsRef::as_ref)
            .collect()
    }

    /// Enable a specific rule
    pub fn enable_rule(&mut self, rule_id: &str) {
        self.disabled_rules.remove(rule_id);
        self.enabled_rules.insert(rule_id.to_string());
    }

    /// Disable a specific rule
    pub fn disable_rule(&mut self, rule_id: &str) {
        self.enabled_rules.remove(rule_id);
        self.disabled_rules.insert(rule_id.to_string());
    }

    /// Check if a rule is enabled
    #[must_use]
    pub fn is_rule_enabled(&self, rule_id: &str) -> bool {
        if self.disabled_rules.contains(rule_id) {
            return false;
        }
        if !self.enabled_rules.is_empty() {
            return self.enabled_rules.contains(rule_id);
        }
        true
    }

    /// Analyze a file with all applicable plugins
    #[must_use]
    pub fn analyze_file(
        &self,
        path: &Path,
        content: &[u8],
        ast: Option<&dyn Any>,
    ) -> Vec<Violation> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let mut violations = Vec::new();

        for plugin in &self.plugins {
            if !plugin.supports_extension(ext) {
                continue;
            }

            let plugin_violations = plugin.analyze(path, content, ast);

            for v in plugin_violations {
                if self.is_rule_enabled(&v.rule_id) {
                    violations.push(v);
                }
            }
        }

        violations
    }

    /// Get all rule definitions from all plugins
    #[must_use]
    pub fn all_rules(&self) -> Vec<RuleDefinition> {
        self.plugins.iter().flat_map(|p| p.rules().to_vec()).collect()
    }

    /// List all plugin IDs
    #[must_use]
    pub fn list_plugins(&self) -> Vec<&'static str> {
        self.plugins.iter().map(|p| p.id()).collect()
    }

    /// Get plugin by ID
    #[must_use]
    pub fn get_plugin(&self, id: &str) -> Option<&dyn ScoringPlugin> {
        self.plugins.iter().find(|p| p.id() == id).map(std::convert::AsRef::as_ref)
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Plugin loader for dynamic loading
///
/// NOTE: Dynamic .sr rule file loading is planned for a future release.
/// Currently, all plugins must be registered at compile time.
pub struct PluginLoader {
    search_paths: Vec<std::path::PathBuf>,
    loaded: Vec<String>,
}

impl PluginLoader {
    /// Create a new plugin loader
    #[must_use]
    pub fn new() -> Self {
        Self {
            search_paths: vec![
                std::path::PathBuf::from("plugins"),
                dirs::home_dir()
                    .map(|h| h.join(".dx-check").join("plugins"))
                    .unwrap_or_default(),
            ],
            loaded: Vec::new(),
        }
    }

    /// Add a search path
    pub fn with_path(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.search_paths.push(path.into());
        self
    }

    /// Load plugins from .sr rule files (stub - full implementation coming soon)
    ///
    /// Returns the number of plugins loaded.
    pub fn load_from_sr_files(&mut self, _registry: &mut PluginRegistry) -> Result<usize, String> {
        // Dynamic .sr rule file loading is planned for v2.0
        // For now, all plugins are compiled in via feature flags
        Ok(0)
    }

    /// Get list of loaded plugin paths
    #[must_use]
    pub fn loaded_plugins(&self) -> &[String] {
        &self.loaded
    }

    /// Get search paths
    #[must_use]
    pub fn search_paths(&self) -> &[std::path::PathBuf] {
        &self.search_paths
    }
}

impl Default for PluginLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_definition() {
        let rule = RuleDefinition::new("test/rule", "Test Rule", Category::Security)
            .with_severity(Severity::High)
            .with_description("A test rule");

        assert_eq!(rule.id, "test/rule");
        assert_eq!(rule.severity, Severity::High);
        assert_eq!(rule.category, Category::Security);
    }

    #[test]
    fn test_plugin_registry() {
        let registry = PluginRegistry::new();
        assert!(registry.plugins().is_empty());
    }

    #[test]
    fn test_rule_enablement() {
        let mut registry = PluginRegistry::new();

        assert!(registry.is_rule_enabled("any/rule"));

        registry.disable_rule("test/rule");
        assert!(!registry.is_rule_enabled("test/rule"));

        registry.enable_rule("test/rule");
        assert!(registry.is_rule_enabled("test/rule"));
    }

    #[test]
    fn test_plugin_loader() {
        let loader = PluginLoader::new();
        assert!(!loader.search_paths().is_empty());
    }
}
