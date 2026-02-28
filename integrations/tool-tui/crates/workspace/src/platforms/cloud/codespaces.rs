//! GitHub Codespaces / Dev Container configuration generator.
//!
//! Generates:
//! - .devcontainer/devcontainer.json
//! - .devcontainer/Dockerfile (optional)

use super::{CloudGenerator, GeneratedFile};
use crate::{Result, WorkspaceConfig};
use serde_json::{Map, Value, json};
use std::fs;
use std::path::Path;

/// GitHub Codespaces configuration generator.
#[derive(Debug, Default)]
pub struct CodespacesGenerator {
    /// Whether to generate a custom Dockerfile.
    pub custom_dockerfile: bool,
}

impl CodespacesGenerator {
    /// Create a new Codespaces generator.
    pub fn new() -> Self {
        Self {
            custom_dockerfile: false,
        }
    }

    /// Enable custom Dockerfile generation.
    pub fn with_dockerfile(mut self) -> Self {
        self.custom_dockerfile = true;
        self
    }

    /// Generate devcontainer.json content.
    fn generate_config(&self, config: &WorkspaceConfig) -> Value {
        let mut devcontainer = Map::new();

        devcontainer.insert("name".to_string(), json!(config.name));

        // Image or Dockerfile
        if self.custom_dockerfile {
            devcontainer.insert(
                "build".to_string(),
                json!({
                    "dockerfile": "Dockerfile",
                    "context": ".."
                }),
            );
        } else if config.detected_features.is_cargo_project {
            devcontainer
                .insert("image".to_string(), json!("mcr.microsoft.com/devcontainers/rust:latest"));
        } else {
            devcontainer
                .insert("image".to_string(), json!("mcr.microsoft.com/devcontainers/base:ubuntu"));
        }

        // Features
        let mut features = Map::new();

        if config.detected_features.is_cargo_project {
            features.insert(
                "ghcr.io/devcontainers/features/rust:1".to_string(),
                json!({
                    "version": "latest",
                    "profile": "default"
                }),
            );
        }

        if config.detected_features.has_dx_client {
            features
                .insert("ghcr.io/aspect-build/devcontainer-features/wasm:1".to_string(), json!({}));
        }

        if !features.is_empty() {
            devcontainer.insert("features".to_string(), json!(features));
        }

        // Customizations
        let mut customizations = Map::new();
        let mut vscode = Map::new();

        // Extensions
        let extensions: Vec<String> = config
            .extensions
            .core
            .iter()
            .chain(config.extensions.recommended.iter())
            .map(|e| e.id.clone())
            .collect();

        if !extensions.is_empty() {
            vscode.insert("extensions".to_string(), json!(extensions));
        }

        // Settings from workspace config
        let mut settings = Map::new();
        settings.insert("editor.tabSize".to_string(), json!(config.editor.tab_size));
        settings.insert("editor.insertSpaces".to_string(), json!(config.editor.insert_spaces));

        if config.detected_features.is_cargo_project {
            settings.insert("rust-analyzer.cargo.features".to_string(), json!("all"));
            settings.insert("rust-analyzer.checkOnSave.command".to_string(), json!("clippy"));
        }

        vscode.insert("settings".to_string(), json!(settings));
        customizations.insert("vscode".to_string(), json!(vscode));
        devcontainer.insert("customizations".to_string(), json!(customizations));

        // Forward ports
        if config.detected_features.has_dx_server || config.detected_features.has_dx_www {
            devcontainer.insert("forwardPorts".to_string(), json!([3000, 8080]));
        }

        // Post-create command
        if config.detected_features.is_cargo_project {
            devcontainer
                .insert("postCreateCommand".to_string(), json!("cargo fetch && cargo build"));
        }

        // On create commands
        devcontainer.insert("updateContentCommand".to_string(), json!("cargo build"));

        // Remote user
        devcontainer.insert("remoteUser".to_string(), json!("vscode"));

        json!(devcontainer)
    }

    /// Generate Dockerfile content.
    fn generate_dockerfile(&self, config: &WorkspaceConfig) -> String {
        let mut lines = Vec::new();

        lines.push("# dx-workspace generated Dockerfile for Codespaces".to_string());
        lines.push(String::new());

        if config.detected_features.is_cargo_project {
            lines.push("FROM mcr.microsoft.com/devcontainers/rust:latest".to_string());
        } else {
            lines.push("FROM mcr.microsoft.com/devcontainers/base:ubuntu".to_string());
        }

        lines.push(String::new());

        // Install dx CLI
        lines.push("# Install dx CLI".to_string());
        lines.push("RUN cargo install dx-cli || true".to_string());

        lines.push(String::new());

        // Install WASM target if needed
        if config.detected_features.has_dx_client {
            lines.push("# Install WASM target for dx-client".to_string());
            lines.push("RUN rustup target add wasm32-unknown-unknown".to_string());
            lines.push(String::new());
        }

        // Install additional tools
        lines.push("# Additional tools".to_string());
        lines.push("RUN apt-get update && apt-get install -y \\".to_string());
        lines.push("    pkg-config \\".to_string());
        lines.push("    libssl-dev \\".to_string());
        lines.push("    && rm -rf /var/lib/apt/lists/*".to_string());

        lines.join("\n")
    }
}

impl CloudGenerator for CodespacesGenerator {
    fn generate(&self, config: &WorkspaceConfig, output_dir: &Path) -> Result<Vec<GeneratedFile>> {
        let mut files = Vec::new();

        let devcontainer_dir = output_dir.join(".devcontainer");
        fs::create_dir_all(&devcontainer_dir)
            .map_err(|e| crate::Error::io(&devcontainer_dir, e))?;

        // Generate devcontainer.json
        let devcontainer_json = self.generate_config(config);
        let json_content = serde_json::to_string_pretty(&devcontainer_json).unwrap_or_default();
        files.push(GeneratedFile::new(".devcontainer/devcontainer.json", json_content.clone()));

        let json_path = devcontainer_dir.join("devcontainer.json");
        fs::write(&json_path, &json_content).map_err(|e| crate::Error::io(&json_path, e))?;

        // Generate Dockerfile if requested
        if self.custom_dockerfile {
            let dockerfile = self.generate_dockerfile(config);
            files.push(GeneratedFile::new(".devcontainer/Dockerfile", dockerfile.clone()));

            let docker_path = devcontainer_dir.join("Dockerfile");
            fs::write(&docker_path, &dockerfile).map_err(|e| crate::Error::io(&docker_path, e))?;
        }

        Ok(files)
    }

    fn exists(&self, project_dir: &Path) -> bool {
        project_dir.join(".devcontainer").exists()
    }

    fn clean(&self, project_dir: &Path) -> Result<()> {
        let devcontainer_dir = project_dir.join(".devcontainer");
        if devcontainer_dir.exists() {
            fs::remove_dir_all(&devcontainer_dir)
                .map_err(|e| crate::Error::io(&devcontainer_dir, e))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ExtensionRecommendations;

    #[test]
    fn test_generate_config() {
        let mut config = WorkspaceConfig::new("test-project");
        config.detected_features.is_cargo_project = true;
        config.extensions = ExtensionRecommendations::dx_defaults();

        let generator = CodespacesGenerator::new();
        let devcontainer = generator.generate_config(&config);

        assert_eq!(devcontainer["name"], "test-project");
        assert!(devcontainer["image"].as_str().unwrap().contains("rust"));
        assert!(devcontainer["customizations"]["vscode"]["extensions"].is_array());
    }

    #[test]
    fn test_generate_dockerfile() {
        let mut config = WorkspaceConfig::new("test");
        config.detected_features.is_cargo_project = true;
        config.detected_features.has_dx_client = true;

        let generator = CodespacesGenerator::new().with_dockerfile();
        let content = generator.generate_dockerfile(&config);

        assert!(content.contains("rust"));
        assert!(content.contains("wasm32-unknown-unknown"));
    }
}
