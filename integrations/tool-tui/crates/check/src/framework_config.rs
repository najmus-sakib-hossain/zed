//! Framework-Specific Configuration
//!
//! This module provides support for framework-specific configuration options
//! and loading framework-specific rules.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::project::Framework;

/// Framework-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkConfig {
    /// Framework identifier
    pub framework: Framework,
    /// Framework version (if detected)
    pub version: Option<String>,
    /// Framework-specific settings
    pub settings: HashMap<String, FrameworkSetting>,
    /// Enabled rules for this framework
    pub enabled_rules: Vec<String>,
    /// Disabled rules for this framework
    pub disabled_rules: Vec<String>,
    /// Rule overrides
    pub rule_overrides: HashMap<String, RuleOverride>,
}

/// Framework setting value
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FrameworkSetting {
    Boolean(bool),
    Number(i64),
    String(String),
    Array(Vec<String>),
    Object(HashMap<String, String>),
}

/// Rule override configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleOverride {
    /// Rule identifier
    pub rule_id: String,
    /// Severity override
    pub severity: Option<SeverityOverride>,
    /// Custom options for the rule
    pub options: Option<HashMap<String, serde_json::Value>>,
}

/// Severity override
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SeverityOverride {
    Error,
    Warning,
    Info,
    Off,
}

/// Framework configuration manager
pub struct FrameworkConfigManager {
    /// Loaded framework configurations
    configs: HashMap<Framework, FrameworkConfig>,
    /// Configuration file paths
    config_paths: HashMap<Framework, Vec<PathBuf>>,
    /// Default configurations for each framework
    default_configs: HashMap<Framework, FrameworkConfig>,
}

impl FrameworkConfigManager {
    /// Create a new framework configuration manager
    #[must_use]
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
            config_paths: HashMap::new(),
            default_configs: Self::init_default_configs(),
        }
    }

    /// Initialize default configurations for each framework
    fn init_default_configs() -> HashMap<Framework, FrameworkConfig> {
        let mut defaults = HashMap::new();

        // React default configuration
        defaults.insert(
            Framework::React,
            FrameworkConfig {
                framework: Framework::React,
                version: None,
                settings: Self::react_default_settings(),
                enabled_rules: vec![
                    "react-hooks/rules-of-hooks".to_string(),
                    "react-hooks/exhaustive-deps".to_string(),
                    "react/no-unescaped-entities".to_string(),
                    "react/display-name".to_string(),
                ],
                disabled_rules: vec![],
                rule_overrides: HashMap::new(),
            },
        );

        // Next.js default configuration
        defaults.insert(
            Framework::Next,
            FrameworkConfig {
                framework: Framework::Next,
                version: None,
                settings: Self::next_default_settings(),
                enabled_rules: vec![
                    "next/no-img-element".to_string(),
                    "next/no-html-link-for-pages".to_string(),
                    "next/no-sync-scripts".to_string(),
                ],
                disabled_rules: vec![],
                rule_overrides: HashMap::new(),
            },
        );

        // Vue default configuration
        defaults.insert(
            Framework::Vue,
            FrameworkConfig {
                framework: Framework::Vue,
                version: None,
                settings: Self::vue_default_settings(),
                enabled_rules: vec![
                    "vue/no-v-html".to_string(),
                    "vue/require-v-for-key".to_string(),
                    "vue/no-mutating-props".to_string(),
                ],
                disabled_rules: vec![],
                rule_overrides: HashMap::new(),
            },
        );

        // Angular default configuration
        defaults.insert(
            Framework::Angular,
            FrameworkConfig {
                framework: Framework::Angular,
                version: None,
                settings: Self::angular_default_settings(),
                enabled_rules: vec![
                    "angular/contextual-lifecycle".to_string(),
                    "angular/no-output-native".to_string(),
                    "angular/no-output-on-prefix".to_string(),
                ],
                disabled_rules: vec![],
                rule_overrides: HashMap::new(),
            },
        );

        defaults
    }

    /// React default settings
    fn react_default_settings() -> HashMap<String, FrameworkSetting> {
        let mut settings = HashMap::new();
        settings
            .insert("jsx-runtime".to_string(), FrameworkSetting::String("automatic".to_string()));
        settings.insert("strict-mode".to_string(), FrameworkSetting::Boolean(true));
        settings.insert("allow-js".to_string(), FrameworkSetting::Boolean(true));
        settings
    }

    /// Next.js default settings
    fn next_default_settings() -> HashMap<String, FrameworkSetting> {
        let mut settings = HashMap::new();
        settings.insert("react-strict-mode".to_string(), FrameworkSetting::Boolean(true));
        settings.insert("swc-minify".to_string(), FrameworkSetting::Boolean(true));
        settings.insert("experimental-app-dir".to_string(), FrameworkSetting::Boolean(false));
        settings
    }

    /// Vue default settings
    fn vue_default_settings() -> HashMap<String, FrameworkSetting> {
        let mut settings = HashMap::new();
        settings.insert(
            "compiler-options".to_string(),
            FrameworkSetting::Object({
                let mut opts = HashMap::new();
                opts.insert(
                    "isCustomElement".to_string(),
                    "tag => tag.startsWith('x-')".to_string(),
                );
                opts
            }),
        );
        settings
    }

    /// Angular default settings
    fn angular_default_settings() -> HashMap<String, FrameworkSetting> {
        let mut settings = HashMap::new();
        settings.insert("strict-templates".to_string(), FrameworkSetting::Boolean(true));
        settings.insert("strict-injection-parameters".to_string(), FrameworkSetting::Boolean(true));
        settings
    }

    /// Load framework configuration from a directory
    pub fn load_config(
        &mut self,
        root: &Path,
        framework: Framework,
    ) -> Result<FrameworkConfig, ConfigError> {
        // Start with default configuration
        let mut config =
            self.default_configs
                .get(&framework)
                .cloned()
                .unwrap_or_else(|| FrameworkConfig {
                    framework,
                    version: None,
                    settings: HashMap::new(),
                    enabled_rules: vec![],
                    disabled_rules: vec![],
                    rule_overrides: HashMap::new(),
                });

        // Try to load framework-specific config files
        let config_files = self.find_config_files(root, framework);

        for config_file in &config_files {
            if let Ok(loaded_config) = self.load_config_file(config_file) {
                // Merge configurations
                config = self.merge_configs(config, loaded_config);
            }
        }

        // Store the loaded configuration
        self.configs.insert(framework, config.clone());
        self.config_paths.insert(framework, config_files);

        Ok(config)
    }

    /// Find framework-specific configuration files
    fn find_config_files(&self, root: &Path, framework: Framework) -> Vec<PathBuf> {
        let mut files = Vec::new();

        // Framework-specific config file names
        let config_names = match framework {
            Framework::React => vec![".dx-react.json", ".dx-react.config.js"],
            Framework::Next => vec![".dx-next.json", ".dx-next.config.js", "next.config.js"],
            Framework::Vue => vec![".dx-vue.json", ".dx-vue.config.js", "vue.config.js"],
            Framework::Angular => vec![".dx-angular.json", ".dx-angular.config.js", "angular.json"],
            Framework::Svelte => vec![
                ".dx-svelte.json",
                ".dx-svelte.config.js",
                "svelte.config.js",
            ],
            Framework::SvelteKit => vec![
                ".dx-sveltekit.json",
                ".dx-sveltekit.config.js",
                "svelte.config.js",
            ],
            Framework::Solid => vec![".dx-solid.json", ".dx-solid.config.js"],
            Framework::Qwik => vec![".dx-qwik.json", ".dx-qwik.config.js"],
            Framework::Remix => vec![".dx-remix.json", ".dx-remix.config.js", "remix.config.js"],
            Framework::Astro => vec![".dx-astro.json", ".dx-astro.config.js", "astro.config.mjs"],
            Framework::Nuxt => vec![".dx-nuxt.json", ".dx-nuxt.config.js", "nuxt.config.ts"],
            Framework::Express => vec![".dx-express.json", ".dx-express.config.js"],
            Framework::Fastify => vec![".dx-fastify.json", ".dx-fastify.config.js"],
            Framework::Hono => vec![".dx-hono.json", ".dx-hono.config.js"],
            Framework::Nest => vec![".dx-nest.json", ".dx-nest.config.js", "nest-cli.json"],
        };

        for config_name in config_names {
            let config_path = root.join(config_name);
            if config_path.exists() {
                files.push(config_path);
            }
        }

        files
    }

    /// Load configuration from a file
    fn load_config_file(&self, path: &Path) -> Result<FrameworkConfig, ConfigError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| ConfigError::IoError(e.to_string()))?;

        // Try JSON first
        if path.extension().is_some_and(|ext| ext == "json") {
            return serde_json::from_str(&content)
                .map_err(|e| ConfigError::ParseError(e.to_string()));
        }

        // For JS config files, we'd need a JS runtime
        // For now, return an error
        Err(ConfigError::UnsupportedFormat(
            "JavaScript config files require a JS runtime".to_string(),
        ))
    }

    /// Merge two configurations
    fn merge_configs(
        &self,
        base: FrameworkConfig,
        override_config: FrameworkConfig,
    ) -> FrameworkConfig {
        let mut merged = base;

        // Merge settings
        for (key, value) in override_config.settings {
            merged.settings.insert(key, value);
        }

        // Merge enabled rules (deduplicate)
        for rule in override_config.enabled_rules {
            if !merged.enabled_rules.contains(&rule) {
                merged.enabled_rules.push(rule);
            }
        }

        // Merge disabled rules (deduplicate)
        for rule in override_config.disabled_rules {
            if !merged.disabled_rules.contains(&rule) {
                merged.disabled_rules.push(rule);
            }
        }

        // Merge rule overrides
        for (rule_id, override_setting) in override_config.rule_overrides {
            merged.rule_overrides.insert(rule_id, override_setting);
        }

        // Update version if provided
        if override_config.version.is_some() {
            merged.version = override_config.version;
        }

        merged
    }

    /// Get configuration for a framework
    #[must_use]
    pub fn get_config(&self, framework: Framework) -> Option<&FrameworkConfig> {
        self.configs.get(&framework)
    }

    /// Get all loaded configurations
    #[must_use]
    pub fn get_all_configs(&self) -> &HashMap<Framework, FrameworkConfig> {
        &self.configs
    }

    /// Get configuration paths for a framework
    #[must_use]
    pub fn get_config_paths(&self, framework: Framework) -> Option<&Vec<PathBuf>> {
        self.config_paths.get(&framework)
    }

    /// Clear all loaded configurations
    pub fn clear(&mut self) {
        self.configs.clear();
        self.config_paths.clear();
    }
}

impl Default for FrameworkConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration error
#[derive(Debug, Clone)]
pub enum ConfigError {
    /// IO error
    IoError(String),
    /// Parse error
    ParseError(String),
    /// Unsupported format
    UnsupportedFormat(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::IoError(msg) => write!(f, "IO error: {msg}"),
            ConfigError::ParseError(msg) => write!(f, "Parse error: {msg}"),
            ConfigError::UnsupportedFormat(msg) => write!(f, "Unsupported format: {msg}"),
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_config_manager_new() {
        let manager = FrameworkConfigManager::new();
        assert!(!manager.default_configs.is_empty());
    }

    #[test]
    fn test_framework_config_manager_default() {
        let manager = FrameworkConfigManager::default();
        assert!(!manager.default_configs.is_empty());
    }

    #[test]
    fn test_get_config_not_loaded() {
        let manager = FrameworkConfigManager::new();
        let config = manager.get_config(Framework::React);
        assert!(config.is_none());
    }

    #[test]
    fn test_clear() {
        let mut manager = FrameworkConfigManager::new();
        manager.clear();
        assert!(manager.configs.is_empty());
        assert!(manager.config_paths.is_empty());
    }

    #[test]
    fn test_merge_configs() {
        let manager = FrameworkConfigManager::new();

        let base = FrameworkConfig {
            framework: Framework::React,
            version: None,
            settings: {
                let mut s = HashMap::new();
                s.insert("setting1".to_string(), FrameworkSetting::Boolean(true));
                s
            },
            enabled_rules: vec!["rule1".to_string()],
            disabled_rules: vec![],
            rule_overrides: HashMap::new(),
        };

        let override_config = FrameworkConfig {
            framework: Framework::React,
            version: Some("1.0.0".to_string()),
            settings: {
                let mut s = HashMap::new();
                s.insert("setting2".to_string(), FrameworkSetting::Boolean(false));
                s
            },
            enabled_rules: vec!["rule2".to_string()],
            disabled_rules: vec!["rule3".to_string()],
            rule_overrides: HashMap::new(),
        };

        let merged = manager.merge_configs(base, override_config);

        assert_eq!(merged.version, Some("1.0.0".to_string()));
        assert_eq!(merged.settings.len(), 2);
        assert_eq!(merged.enabled_rules.len(), 2);
        assert_eq!(merged.disabled_rules.len(), 1);
    }
}
