//! Project detection and analysis.
//!
//! This module detects project characteristics to generate intelligent defaults.

use crate::Result;
use crate::config::{
    DebugConfig, DetectedFeatures, EditorConfig, ExtensionRecommendations, ProjectStructureConfig,
    TaskConfig, WorkspaceConfig,
};
use std::path::{Path, PathBuf};

/// Detects project characteristics for intelligent configuration defaults.
pub struct ProjectDetector {
    root: PathBuf,
}

impl ProjectDetector {
    /// Create a new project detector for the given directory.
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    /// Detect project characteristics and create a workspace configuration.
    pub fn detect(&self) -> Result<WorkspaceConfig> {
        let name = self.detect_name();
        let features = self.detect_features();

        let mut config = WorkspaceConfig::new(&name);
        config.root = self.root.clone();
        config.detected_features = features.clone();

        // Apply intelligent defaults based on detected features
        config.editor = self.create_editor_config(&features);
        config.debug = self.create_debug_config(&features);
        config.tasks = self.create_task_config(&features);
        config.extensions = self.create_extension_config(&features);
        config.project = self.create_project_structure_config(&features);

        Ok(config)
    }

    /// Detect project name from Cargo.toml or directory name.
    fn detect_name(&self) -> String {
        // Try Cargo.toml first
        let cargo_toml = self.root.join("Cargo.toml");
        if cargo_toml.exists()
            && let Ok(content) = std::fs::read_to_string(&cargo_toml)
            && let Ok(parsed) = content.parse::<toml::Table>()
        {
            if let Some(package) = parsed.get("package").and_then(|p| p.as_table())
                && let Some(name) = package.get("name").and_then(|n| n.as_str())
            {
                return name.to_string();
            }
            // Check workspace name
            if let Some(workspace) = parsed.get("workspace").and_then(|w| w.as_table())
                && let Some(package) = workspace.get("package").and_then(|p| p.as_table())
                && let Some(name) = package.get("name").and_then(|n| n.as_str())
            {
                return name.to_string();
            }
        }

        // Try package.json
        let package_json = self.root.join("package.json");
        if package_json.exists()
            && let Ok(content) = std::fs::read_to_string(&package_json)
            && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content)
            && let Some(name) = parsed.get("name").and_then(|n| n.as_str())
        {
            return name.to_string();
        }

        // Fall back to directory name
        self.root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("dx-project")
            .to_string()
    }

    /// Detect project features.
    fn detect_features(&self) -> DetectedFeatures {
        let mut features = DetectedFeatures::default();

        // Check for Cargo.toml
        let cargo_toml = self.root.join("Cargo.toml");
        if cargo_toml.exists() {
            features.is_cargo_project = true;
            features.is_rust = true;

            // Parse Cargo.toml for dx dependencies and workspace info
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                features.has_dx_www = content.contains("dx-www");
                features.has_dx_style = content.contains("dx-style");
                features.has_dx_server = content.contains("dx-server");
                features.has_dx_client = content.contains("dx-client");
                features.has_dx_forge = content.contains("dx-forge");
                features.has_dx_form = content.contains("dx-form");
                features.has_dx_query = content.contains("dx-query");
                features.has_dx_state = content.contains("dx-state");
                features.has_dx_debug = content.contains("dx-debug");
                features.has_dx_i18n = content.contains("dx-i18n");
                features.has_dx_db = content.contains("dx-db");
                features.is_workspace = content.contains("[workspace]");
            }
        }

        // Check for Git
        features.has_git = self.root.join(".git").exists() || self.root.join(".gitignore").exists();

        // Check for TypeScript
        features.uses_typescript = self.root.join("tsconfig.json").exists()
            || self.root.join("package.json").exists()
                && std::fs::read_to_string(self.root.join("package.json"))
                    .map(|c| c.contains("typescript"))
                    .unwrap_or(false);

        // Check for existing configurations
        features.has_vscode_config = self.root.join(".vscode").exists();
        features.has_gitpod_config = self.root.join(".gitpod.yml").exists();
        features.has_devcontainer_config = self.root.join(".devcontainer").exists();

        // Check crates directory for dx features (workspace projects)
        let crates_dir = self.root.join("crates");
        if crates_dir.exists() {
            if crates_dir.join("dx-www").exists() {
                features.has_dx_www = true;
            }
            if crates_dir.join("dx-style").exists() {
                features.has_dx_style = true;
            }
            if crates_dir.join("dx-server").exists() {
                features.has_dx_server = true;
            }
            if crates_dir.join("dx-client").exists() {
                features.has_dx_client = true;
            }
            if crates_dir.join("dx-forge").exists() {
                features.has_dx_forge = true;
            }
        }

        features
    }

    /// Create editor configuration based on detected features.
    fn create_editor_config(&self, _features: &DetectedFeatures) -> EditorConfig {
        EditorConfig {
            tab_size: 4,
            insert_spaces: true,
            font_family: Some("JetBrains Mono, Fira Code, monospace".to_string()),
            font_size: Some(14),
            line_height: Some(1.5),
            breadcrumbs_enabled: true,
            ..Default::default()
        }
    }

    /// Create debug configuration based on detected features.
    fn create_debug_config(&self, features: &DetectedFeatures) -> DebugConfig {
        if features.is_cargo_project {
            DebugConfig::dx_defaults()
        } else {
            DebugConfig::default()
        }
    }

    /// Create task configuration based on detected features.
    fn create_task_config(&self, features: &DetectedFeatures) -> TaskConfig {
        if features.is_cargo_project {
            TaskConfig::dx_defaults()
        } else {
            TaskConfig::default()
        }
    }

    /// Create extension recommendations based on detected features.
    fn create_extension_config(&self, features: &DetectedFeatures) -> ExtensionRecommendations {
        if features.is_cargo_project {
            ExtensionRecommendations::dx_defaults()
        } else {
            ExtensionRecommendations::default()
        }
    }

    /// Create project structure configuration.
    fn create_project_structure_config(
        &self,
        features: &DetectedFeatures,
    ) -> ProjectStructureConfig {
        let mut config = ProjectStructureConfig::default();

        // File associations
        config.file_associations.insert("*.dx".to_string(), "rust".to_string());
        config.file_associations.insert("*.dxb".to_string(), "binary".to_string());

        // Search exclusions
        config.search_exclude = vec![
            "**/target/**".to_string(),
            "**/node_modules/**".to_string(),
            "**/.git/**".to_string(),
            "**/dist/**".to_string(),
            "**/*.wasm".to_string(),
        ];

        // Watcher exclusions (more aggressive)
        config.watcher_exclude = vec!["**/target/**".to_string(), "**/node_modules/**".to_string()];

        // File nesting for Rust projects
        if features.is_cargo_project {
            config.file_nesting.enabled = true;
            config.file_nesting.patterns.insert(
                "Cargo.toml".to_string(),
                vec!["Cargo.lock".to_string(), ".cargo".to_string()],
            );
            config
                .file_nesting
                .patterns
                .insert("*.rs".to_string(), vec!["$(capture).generated.rs".to_string()]);
        }

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_detect_cargo_project() {
        let dir = tempdir().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_toml,
            r#"
[package]
name = "test-project"
version = "0.1.0"

[dependencies]
dx-www = "0.1"
dx-style = "0.1"
"#,
        )
        .unwrap();

        let detector = ProjectDetector::new(dir.path());
        let config = detector.detect().unwrap();

        assert_eq!(config.name, "test-project");
        assert!(config.detected_features.is_cargo_project);
        assert!(config.detected_features.has_dx_www);
        assert!(config.detected_features.has_dx_style);
    }

    #[test]
    fn test_detect_name_fallback() {
        let dir = tempdir().unwrap();
        let detector = ProjectDetector::new(dir.path());
        let config = detector.detect().unwrap();

        // Should use directory name as fallback
        assert!(!config.name.is_empty());
    }
}
