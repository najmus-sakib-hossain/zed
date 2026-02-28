//! Plugin Manifest Module
//!
//! Defines the plugin manifest format for DX plugins.
//! Manifests describe plugin metadata, capabilities, permissions,
//! runtime requirements, and configuration schema.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::PluginType;
use super::traits::Capability;

/// Plugin manifest version
pub const MANIFEST_VERSION: &str = "1.0";

/// Plugin manifest file name
pub const MANIFEST_FILE: &str = "plugin.yaml";

/// Plugin manifest - describes a DX plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Manifest schema version
    #[serde(default = "default_manifest_version")]
    pub manifest_version: String,

    /// Plugin identity
    pub plugin: PluginIdentity,

    /// Runtime requirements
    #[serde(default)]
    pub runtime: RuntimeRequirements,

    /// Required capabilities/permissions
    #[serde(default)]
    pub capabilities: Vec<String>,

    /// Plugin configuration schema
    #[serde(default)]
    pub config: HashMap<String, ConfigField>,

    /// Hook registrations
    #[serde(default)]
    pub hooks: Vec<HookRegistration>,

    /// Plugin dependencies
    #[serde(default)]
    pub dependencies: Vec<PluginDependency>,

    /// Build information
    #[serde(default)]
    pub build: Option<BuildInfo>,
}

fn default_manifest_version() -> String {
    MANIFEST_VERSION.to_string()
}

/// Plugin identity information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginIdentity {
    /// Plugin name (unique identifier)
    pub name: String,

    /// Human-readable display name
    #[serde(default)]
    pub display_name: Option<String>,

    /// Plugin version (semver)
    pub version: String,

    /// Plugin description
    #[serde(default)]
    pub description: Option<String>,

    /// Plugin author
    #[serde(default)]
    pub author: Option<String>,

    /// Plugin license
    #[serde(default)]
    pub license: Option<String>,

    /// Plugin homepage URL
    #[serde(default)]
    pub homepage: Option<String>,

    /// Plugin repository URL
    #[serde(default)]
    pub repository: Option<String>,

    /// Plugin type (wasm or native)
    #[serde(default = "default_plugin_type")]
    pub plugin_type: PluginType,

    /// Entry point file
    pub entry: String,

    /// Plugin icon (relative path)
    #[serde(default)]
    pub icon: Option<String>,

    /// Plugin tags for discovery
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_plugin_type() -> PluginType {
    PluginType::Wasm
}

/// Runtime requirements for the plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeRequirements {
    /// Minimum DX CLI version
    #[serde(default)]
    pub min_dx_version: Option<String>,

    /// Maximum memory (bytes)
    #[serde(default = "default_max_memory")]
    pub max_memory: u64,

    /// Maximum CPU time per invocation (ms)
    #[serde(default = "default_max_cpu_ms")]
    pub max_cpu_ms: u64,

    /// Maximum execution timeout (ms)
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,

    /// Required platform features
    #[serde(default)]
    pub features: Vec<String>,
}

fn default_max_memory() -> u64 {
    256 * 1024 * 1024 // 256 MB
}

fn default_max_cpu_ms() -> u64 {
    30_000 // 30 seconds
}

fn default_timeout_ms() -> u64 {
    60_000 // 60 seconds
}

impl Default for RuntimeRequirements {
    fn default() -> Self {
        Self {
            min_dx_version: None,
            max_memory: default_max_memory(),
            max_cpu_ms: default_max_cpu_ms(),
            timeout_ms: default_timeout_ms(),
            features: vec![],
        }
    }
}

/// Configuration field definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigField {
    /// Field type: string, number, boolean, array, object
    #[serde(rename = "type")]
    pub field_type: String,

    /// Human-readable description
    #[serde(default)]
    pub description: Option<String>,

    /// Default value
    #[serde(default)]
    pub default: Option<serde_json::Value>,

    /// Is this field required?
    #[serde(default)]
    pub required: bool,

    /// Allowed values (enum)
    #[serde(default)]
    pub allowed_values: Vec<serde_json::Value>,

    /// Is this field a secret?
    #[serde(default)]
    pub secret: bool,
}

/// Hook registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookRegistration {
    /// Hook event name (e.g., "before_chat", "after_exec")
    pub event: String,

    /// Handler function/method name
    pub handler: String,

    /// Priority (lower = earlier execution)
    #[serde(default = "default_priority")]
    pub priority: i32,

    /// Only trigger for certain conditions
    #[serde(default)]
    pub filter: Option<String>,
}

fn default_priority() -> i32 {
    100
}

/// Plugin dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    /// Plugin name
    pub name: String,

    /// Version requirement (semver range)
    pub version: String,

    /// Is this dependency optional?
    #[serde(default)]
    pub optional: bool,
}

/// Build information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildInfo {
    /// Build command
    pub command: Option<String>,

    /// Build output directory
    pub output: Option<String>,

    /// Source directory
    pub source: Option<String>,
}

impl PluginManifest {
    /// Load a manifest from a YAML file
    pub fn from_file(path: &Path) -> Result<Self, ManifestError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ManifestError::IoError(path.display().to_string(), e.to_string()))?;
        Self::from_yaml(&content)
    }

    /// Parse a manifest from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self, ManifestError> {
        serde_yaml::from_str(yaml).map_err(|e| ManifestError::ParseError(e.to_string()))
    }

    /// Serialize manifest to YAML string
    pub fn to_yaml(&self) -> Result<String, ManifestError> {
        serde_yaml::to_string(self).map_err(|e| ManifestError::SerializeError(e.to_string()))
    }

    /// Load manifest from a plugin directory
    pub fn from_dir(dir: &Path) -> Result<Self, ManifestError> {
        let manifest_path = dir.join(MANIFEST_FILE);
        if manifest_path.exists() {
            Self::from_file(&manifest_path)
        } else {
            Err(ManifestError::NotFound(dir.display().to_string()))
        }
    }

    /// Get parsed capabilities
    pub fn parsed_capabilities(&self) -> Vec<Capability> {
        self.capabilities.iter().map(|c| Capability::from_str(c)).collect()
    }

    /// Check if the plugin has dangerous capabilities
    pub fn has_dangerous_capabilities(&self) -> bool {
        self.parsed_capabilities().iter().any(|c| c.is_dangerous())
    }

    /// Get the display name (or fallback to name)
    pub fn display_name(&self) -> &str {
        self.plugin.display_name.as_deref().unwrap_or(&self.plugin.name)
    }
}

/// Manifest errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum ManifestError {
    #[error("Manifest not found in: {0}")]
    NotFound(String),

    #[error("Failed to read manifest from {0}: {1}")]
    IoError(String, String),

    #[error("Failed to parse manifest: {0}")]
    ParseError(String),

    #[error("Failed to serialize manifest: {0}")]
    SerializeError(String),

    #[error("Invalid manifest: {0}")]
    Invalid(String),
}

/// Generate a minimal example plugin manifest
pub fn example_manifest() -> PluginManifest {
    PluginManifest {
        manifest_version: MANIFEST_VERSION.to_string(),
        plugin: PluginIdentity {
            name: "example-plugin".to_string(),
            display_name: Some("Example Plugin".to_string()),
            version: "0.1.0".to_string(),
            description: Some("An example DX plugin".to_string()),
            author: Some("DX Team".to_string()),
            license: Some("MIT".to_string()),
            homepage: None,
            repository: None,
            plugin_type: PluginType::Wasm,
            entry: "plugin.wasm".to_string(),
            icon: None,
            tags: vec!["example".to_string()],
        },
        runtime: RuntimeRequirements::default(),
        capabilities: vec!["network".to_string(), "file_read".to_string()],
        config: {
            let mut m = HashMap::new();
            m.insert(
                "api_key".to_string(),
                ConfigField {
                    field_type: "string".to_string(),
                    description: Some("API key for the service".to_string()),
                    default: None,
                    required: true,
                    allowed_values: vec![],
                    secret: true,
                },
            );
            m
        },
        hooks: vec![HookRegistration {
            event: "before_chat".to_string(),
            handler: "on_before_chat".to_string(),
            priority: 100,
            filter: None,
        }],
        dependencies: vec![],
        build: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_manifest() {
        let manifest = example_manifest();
        assert_eq!(manifest.plugin.name, "example-plugin");
        assert_eq!(manifest.plugin.version, "0.1.0");
        assert_eq!(manifest.capabilities.len(), 2);
        assert_eq!(manifest.hooks.len(), 1);
    }

    #[test]
    fn test_manifest_yaml_roundtrip() {
        let manifest = example_manifest();
        let yaml = manifest.to_yaml().unwrap();
        let parsed = PluginManifest::from_yaml(&yaml).unwrap();
        assert_eq!(parsed.plugin.name, manifest.plugin.name);
        assert_eq!(parsed.plugin.version, manifest.plugin.version);
        assert_eq!(parsed.capabilities.len(), manifest.capabilities.len());
    }

    #[test]
    fn test_manifest_from_yaml() {
        let yaml = r#"
plugin:
  name: test-plugin
  version: "1.0.0"
  entry: plugin.wasm
capabilities:
  - network
  - shell
hooks:
  - event: before_chat
    handler: handle_chat
    priority: 50
"#;
        let manifest = PluginManifest::from_yaml(yaml).unwrap();
        assert_eq!(manifest.plugin.name, "test-plugin");
        assert_eq!(manifest.plugin.version, "1.0.0");
        assert!(manifest.has_dangerous_capabilities()); // shell is dangerous
        assert_eq!(manifest.hooks[0].priority, 50);
    }

    #[test]
    fn test_dangerous_capabilities() {
        let yaml = r#"
plugin:
  name: safe-plugin
  version: "1.0.0"
  entry: plugin.wasm
capabilities:
  - network
  - file_read
"#;
        let manifest = PluginManifest::from_yaml(yaml).unwrap();
        assert!(!manifest.has_dangerous_capabilities());
    }

    #[test]
    fn test_display_name_fallback() {
        let yaml = r#"
plugin:
  name: my-plugin
  version: "1.0.0"
  entry: plugin.wasm
"#;
        let manifest = PluginManifest::from_yaml(yaml).unwrap();
        assert_eq!(manifest.display_name(), "my-plugin");
    }

    #[test]
    fn test_manifest_from_dir() {
        let dir = tempfile::tempdir().unwrap();
        let manifest_path = dir.path().join(MANIFEST_FILE);

        let manifest = example_manifest();
        let yaml = manifest.to_yaml().unwrap();
        std::fs::write(&manifest_path, &yaml).unwrap();

        let loaded = PluginManifest::from_dir(dir.path()).unwrap();
        assert_eq!(loaded.plugin.name, "example-plugin");
    }

    #[test]
    fn test_manifest_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let result = PluginManifest::from_dir(dir.path());
        assert!(result.is_err());
    }
}
