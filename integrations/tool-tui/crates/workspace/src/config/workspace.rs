//! Main workspace configuration type.

use super::{DebugConfig, EditorConfig, ExtensionRecommendations, TaskConfig};
use crate::platforms::Platform;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The main workspace configuration that serves as the single source of truth.
///
/// This configuration is stored in dx's binary format and can generate
/// platform-specific configurations for any supported editor or IDE.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Schema version for forward compatibility.
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,

    /// Project name (derived from directory or Cargo.toml).
    pub name: String,

    /// Project description.
    #[serde(default)]
    pub description: String,

    /// Root directory of the project.
    #[serde(skip)]
    pub root: PathBuf,

    /// Editor experience configuration.
    #[serde(default)]
    pub editor: EditorConfig,

    /// Debug and launch configurations.
    #[serde(default)]
    pub debug: DebugConfig,

    /// Task automation configurations.
    #[serde(default)]
    pub tasks: TaskConfig,

    /// Extension recommendations.
    #[serde(default)]
    pub extensions: ExtensionRecommendations,

    /// Project structure intelligence.
    #[serde(default)]
    pub project: ProjectStructureConfig,

    /// Platform-specific overrides.
    #[serde(default)]
    pub platform_overrides: HashMap<Platform, PlatformOverride>,

    /// Detected dx features in the project.
    #[serde(default)]
    pub detected_features: DetectedFeatures,

    /// Custom metadata for extensibility.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

fn default_schema_version() -> u32 {
    1
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            schema_version: default_schema_version(),
            name: String::new(),
            description: String::new(),
            root: PathBuf::new(),
            editor: EditorConfig::default(),
            debug: DebugConfig::default(),
            tasks: TaskConfig::default(),
            extensions: ExtensionRecommendations::default(),
            project: ProjectStructureConfig::default(),
            platform_overrides: HashMap::new(),
            detected_features: DetectedFeatures::default(),
            metadata: HashMap::new(),
        }
    }
}

impl WorkspaceConfig {
    /// Create a new workspace configuration with defaults.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Detect workspace configuration from a project directory.
    ///
    /// This scans the project structure and creates intelligent defaults
    /// based on detected dx features.
    pub fn detect(path: impl AsRef<Path>) -> crate::Result<Self> {
        let detector = crate::ProjectDetector::new(path.as_ref());
        detector.detect()
    }

    /// Load workspace configuration from a dx-workspace binary file.
    pub fn load(path: impl AsRef<Path>) -> crate::Result<Self> {
        let path = path.as_ref();
        let bytes = std::fs::read(path).map_err(|e| crate::Error::io(path, e))?;

        // Try JSON first (for human-readable format during development)
        if bytes.starts_with(b"{") {
            let config: Self =
                serde_json::from_slice(&bytes).map_err(|e| crate::Error::json_parse(path, e))?;
            return Ok(config);
        }

        // TODO: Implement binary format loading with dx-serializer
        Err(crate::Error::invalid_config("Binary format not yet implemented"))
    }

    /// Save workspace configuration to a file.
    pub fn save(&self, path: impl AsRef<Path>) -> crate::Result<()> {
        let path = path.as_ref();

        // For now, save as pretty-printed JSON
        let json =
            serde_json::to_string_pretty(self).map_err(|e| crate::Error::json_parse(path, e))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| crate::Error::io(parent, e))?;
        }

        std::fs::write(path, json).map_err(|e| crate::Error::io(path, e))?;
        Ok(())
    }

    /// Get the Blake3 hash of this configuration for caching.
    pub fn content_hash(&self) -> String {
        let json = serde_json::to_string(self).unwrap_or_default();
        let hash = blake3::hash(json.as_bytes());
        hash.to_hex().to_string()
    }

    /// Validate the workspace configuration.
    pub fn validate(&self) -> crate::Result<()> {
        if self.name.is_empty() {
            return Err(crate::Error::validation("Project name cannot be empty"));
        }

        // Validate editor config
        self.editor.validate()?;

        // Validate debug config
        self.debug.validate()?;

        // Validate tasks
        self.tasks.validate()?;

        Ok(())
    }
}

/// Project structure configuration for file associations and search patterns.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectStructureConfig {
    /// File associations mapping extensions to languages.
    #[serde(default)]
    pub file_associations: HashMap<String, String>,

    /// Patterns to exclude from search and file watchers.
    #[serde(default)]
    pub search_exclude: Vec<String>,

    /// Patterns to exclude from file watchers only.
    #[serde(default)]
    pub watcher_exclude: Vec<String>,

    /// File nesting rules for generated artifacts.
    #[serde(default)]
    pub file_nesting: FileNestingConfig,

    /// Custom icon associations.
    #[serde(default)]
    pub icon_associations: HashMap<String, String>,
}

/// File nesting configuration for IDE file explorers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileNestingConfig {
    /// Enable file nesting.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Expand nested files by default.
    #[serde(default)]
    pub expand: bool,

    /// Nesting patterns mapping parent files to child patterns.
    #[serde(default)]
    pub patterns: HashMap<String, Vec<String>>,
}

fn default_true() -> bool {
    true
}

/// Platform-specific configuration overrides.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlatformOverride {
    /// Override editor settings for this platform.
    #[serde(default)]
    pub editor: Option<EditorConfig>,

    /// Override debug settings for this platform.
    #[serde(default)]
    pub debug: Option<DebugConfig>,

    /// Override task settings for this platform.
    #[serde(default)]
    pub tasks: Option<TaskConfig>,

    /// Override extension recommendations for this platform.
    #[serde(default)]
    pub extensions: Option<ExtensionRecommendations>,

    /// Additional platform-specific settings.
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Features detected in the dx project.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DetectedFeatures {
    /// dx-www components detected.
    pub has_dx_www: bool,

    /// dx-style usage detected.
    pub has_dx_style: bool,

    /// dx-server presence detected.
    pub has_dx_server: bool,

    /// dx-client WASM runtime detected.
    pub has_dx_client: bool,

    /// dx-forge build pipeline detected.
    pub has_dx_forge: bool,

    /// dx-form form handling detected.
    pub has_dx_form: bool,

    /// dx-query data fetching detected.
    pub has_dx_query: bool,

    /// dx-state state management detected.
    pub has_dx_state: bool,

    /// dx-debug debugging tools detected.
    pub has_dx_debug: bool,

    /// dx-i18n internationalization detected.
    pub has_dx_i18n: bool,

    /// dx-db database integration detected.
    pub has_dx_db: bool,

    /// Uses TypeScript.
    pub uses_typescript: bool,

    /// Is a Rust/Cargo project.
    pub is_cargo_project: bool,

    /// Is a Rust project (alias for is_cargo_project).
    pub is_rust: bool,

    /// Is a Cargo workspace with multiple members.
    pub is_workspace: bool,

    /// Has Git repository.
    pub has_git: bool,

    /// Has existing VS Code configuration.
    pub has_vscode_config: bool,

    /// Has existing Gitpod configuration.
    pub has_gitpod_config: bool,

    /// Has existing devcontainer configuration.
    pub has_devcontainer_config: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_config_defaults() {
        let config = WorkspaceConfig::new("test-project");
        assert_eq!(config.name, "test-project");
        assert_eq!(config.schema_version, 1);
    }

    #[test]
    fn test_workspace_config_serialization() {
        let config = WorkspaceConfig::new("test-project");
        let json = serde_json::to_string(&config).unwrap();
        let parsed: WorkspaceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, config.name);
    }

    #[test]
    fn test_content_hash() {
        let config1 = WorkspaceConfig::new("test");
        let config2 = WorkspaceConfig::new("test");
        assert_eq!(config1.content_hash(), config2.content_hash());

        let config3 = WorkspaceConfig::new("other");
        assert_ne!(config1.content_hash(), config3.content_hash());
    }
}
