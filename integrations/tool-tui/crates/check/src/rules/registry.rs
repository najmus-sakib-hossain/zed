//! Rule Registry
//!
//! Manages all available rules and their configurations.

use super::builtin;
use super::{Rule, RuleId, Severity};
use crate::config::{RuleConfigs, RuleSeverity};
use crate::project::Framework;
use std::collections::HashMap;
use std::path::Path;

/// Registry of all available lint rules
///
/// Provides HashMap-based storage for fast O(1) rule lookup by ID or name,
/// and efficient category-based filtering for category-specific analysis.
pub struct RuleRegistry {
    /// Rules indexed by ID for O(1) lookup
    rules_by_id: HashMap<u16, Box<dyn Rule>>,
    /// Rules indexed by name for O(1) lookup
    rules_by_name: HashMap<String, u16>,
    /// Rules indexed by category for category-specific analysis
    rules_by_category: HashMap<super::Category, Vec<u16>>,
    /// Enabled rules with their configured severity
    enabled: HashMap<u16, Severity>,
}

impl RuleRegistry {
    /// Create a new empty registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            rules_by_id: HashMap::new(),
            rules_by_name: HashMap::new(),
            rules_by_category: HashMap::new(),
            enabled: HashMap::new(),
        }
    }

    /// Create a registry with all built-in rules
    #[must_use]
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();

        for rule in builtin::all_rules() {
            registry.register(rule);
        }

        registry
    }

    /// Create a registry configured from `RuleConfigs`
    #[must_use]
    pub fn from_config(config: &RuleConfigs) -> Self {
        let mut registry = Self::with_builtins();

        // Enable recommended rules by default
        if config.recommended {
            for rule in builtin::recommended_rules() {
                let id = rule.meta().id.0;
                registry.enabled.insert(id, rule.meta().default_severity);
            }
        }

        // Apply individual rule configurations
        for (name, rule_config) in &config.rules {
            if let Some(&id) = registry.rules_by_name.get(name) {
                match rule_config.severity() {
                    RuleSeverity::Off => {
                        registry.enabled.remove(&id);
                    }
                    RuleSeverity::Warn => {
                        registry.enabled.insert(id, Severity::Warn);
                    }
                    RuleSeverity::Error => {
                        registry.enabled.insert(id, Severity::Error);
                    }
                }
            }
        }

        registry
    }

    /// Register a rule
    pub fn register(&mut self, rule: Box<dyn Rule>) {
        let meta = rule.meta();
        let id = meta.id.0;
        let name = meta.name.to_string();
        let category = meta.category;

        self.rules_by_name.insert(name, id);
        self.rules_by_category.entry(category).or_default().push(id);
        self.rules_by_id.insert(id, rule);
    }

    /// Enable a rule with a specific severity
    pub fn enable(&mut self, name: &str, severity: Severity) {
        if let Some(&id) = self.rules_by_name.get(name) {
            self.enabled.insert(id, severity);
        }
    }

    /// Disable a rule
    pub fn disable(&mut self, name: &str) {
        if let Some(&id) = self.rules_by_name.get(name) {
            self.enabled.remove(&id);
        }
    }

    /// Get all enabled rules
    pub fn enabled_rules(&self) -> impl Iterator<Item = (&Box<dyn Rule>, Severity)> {
        self.enabled.iter().filter_map(move |(id, severity)| {
            self.rules_by_id.get(id).map(|rule| (rule, *severity))
        })
    }

    /// Get a rule by name
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Box<dyn Rule>> {
        self.rules_by_name.get(name).and_then(|id| self.rules_by_id.get(id))
    }

    /// Get a rule by ID
    #[must_use]
    pub fn get_by_id(&self, id: RuleId) -> Option<&Box<dyn Rule>> {
        self.rules_by_id.get(&id.0)
    }

    /// Get all rules in a specific category
    #[must_use]
    pub fn get_by_category(&self, category: super::Category) -> Vec<&Box<dyn Rule>> {
        self.rules_by_category
            .get(&category)
            .map(|ids| ids.iter().filter_map(|id| self.rules_by_id.get(id)).collect())
            .unwrap_or_default()
    }

    /// Deregister a rule by name
    pub fn deregister(&mut self, name: &str) -> bool {
        if let Some(&id) = self.rules_by_name.get(name) {
            // Remove from name index
            self.rules_by_name.remove(name);

            // Remove from enabled set
            self.enabled.remove(&id);

            // Remove from category index
            if let Some(rule) = self.rules_by_id.get(&id) {
                let category = rule.meta().category;
                if let Some(ids) = self.rules_by_category.get_mut(&category) {
                    ids.retain(|&rule_id| rule_id != id);
                }
            }

            // Remove from ID index
            self.rules_by_id.remove(&id);

            true
        } else {
            false
        }
    }

    /// Check if a rule is enabled
    #[must_use]
    pub fn is_enabled(&self, name: &str) -> bool {
        self.rules_by_name.get(name).is_some_and(|id| self.enabled.contains_key(id))
    }

    /// Get all registered rule names
    pub fn rule_names(&self) -> impl Iterator<Item = &str> {
        self.rules_by_name.keys().map(std::string::String::as_str)
    }

    /// Get count of registered rules
    #[must_use]
    pub fn len(&self) -> usize {
        self.rules_by_id.len()
    }

    /// Check if registry is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rules_by_id.is_empty()
    }

    /// Get count of enabled rules
    #[must_use]
    pub fn enabled_count(&self) -> usize {
        self.enabled.len()
    }

    /// Load framework-specific rules from .sr files
    pub fn load_framework_rules(
        &mut self,
        framework: Framework,
        root: &Path,
    ) -> Result<usize, String> {
        let rule_file = match framework {
            Framework::React => "react-rules.sr",
            Framework::Next => "next-rules.sr",
            Framework::Vue => "vue-rules.sr",
            Framework::Nuxt => "vue-rules.sr", // Nuxt uses Vue rules
            Framework::Angular => "angular-rules.sr",
            Framework::Svelte | Framework::SvelteKit => "svelte-rules.sr",
            _ => return Ok(0), // No specific rules for other frameworks yet
        };

        // Try to load from project-local rules directory first
        let local_path = root.join(".dx/rules").join(rule_file);
        let global_path = Path::new("crates/check/rules").join(rule_file);

        let rule_path = if local_path.exists() {
            local_path
        } else if global_path.exists() {
            global_path
        } else {
            tracing::debug!(
                framework = framework.as_str(),
                file = rule_file,
                "Framework rule file not found"
            );
            return Ok(0);
        };

        // For now, just log that we would load the rules
        // In a full implementation, this would parse the .sr file and register rules
        tracing::info!(
            framework = framework.as_str(),
            path = %rule_path.display(),
            "Loading framework-specific rules"
        );

        // TODO: Parse .sr file and register rules
        // This would use the dxs_parser module to parse the rule definitions
        // and create Rule instances to register

        Ok(0)
    }

    /// Enable framework-specific rules based on framework configuration
    pub fn enable_framework_rules(&mut self, framework: Framework, enabled_rules: &[String]) {
        for rule_name in enabled_rules {
            if self.rules_by_name.contains_key(rule_name) {
                self.enable(rule_name, Severity::Warn);
                tracing::debug!(
                    framework = framework.as_str(),
                    rule = rule_name,
                    "Enabled framework-specific rule"
                );
            }
        }
    }

    /// Disable framework-specific rules based on framework configuration
    pub fn disable_framework_rules(&mut self, framework: Framework, disabled_rules: &[String]) {
        for rule_name in disabled_rules {
            if self.rules_by_name.contains_key(rule_name) {
                self.disable(rule_name);
                tracing::debug!(
                    framework = framework.as_str(),
                    rule = rule_name,
                    "Disabled framework-specific rule"
                );
            }
        }
    }
}

impl Default for RuleRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Category;

    #[test]
    fn test_registry_with_builtins() {
        let registry = RuleRegistry::with_builtins();
        assert!(!registry.is_empty());
        assert!(registry.get("no-console").is_some());
        assert!(registry.get("no-debugger").is_some());
    }

    #[test]
    fn test_enable_disable() {
        let mut registry = RuleRegistry::with_builtins();
        registry.enable("no-console", Severity::Error);
        assert!(registry.is_enabled("no-console"));

        registry.disable("no-console");
        assert!(!registry.is_enabled("no-console"));
    }

    #[test]
    fn test_get_by_category() {
        let registry = RuleRegistry::with_builtins();

        // Get all security rules
        let security_rules = registry.get_by_category(Category::Security);
        assert!(!security_rules.is_empty(), "Should have security rules");

        // Verify all returned rules are in the Security category
        for rule in security_rules {
            assert_eq!(rule.meta().category, Category::Security);
        }
    }

    #[test]
    fn test_deregister() {
        let mut registry = RuleRegistry::with_builtins();

        // Verify rule exists
        assert!(registry.get("no-console").is_some());
        let initial_count = registry.len();

        // Deregister the rule
        assert!(registry.deregister("no-console"));

        // Verify rule is removed
        assert!(registry.get("no-console").is_none());
        assert_eq!(registry.len(), initial_count - 1);

        // Deregistering again should return false
        assert!(!registry.deregister("no-console"));
    }

    #[test]
    fn test_deregister_removes_from_category() {
        let mut registry = RuleRegistry::with_builtins();

        // Get a rule and its category
        let rule = registry.get("no-console").expect("Rule should exist");
        let category = rule.meta().category;
        let initial_category_count = registry.get_by_category(category).len();

        // Deregister the rule
        registry.deregister("no-console");

        // Verify it's removed from category index
        let final_category_count = registry.get_by_category(category).len();
        assert_eq!(final_category_count, initial_category_count - 1);
    }
}
