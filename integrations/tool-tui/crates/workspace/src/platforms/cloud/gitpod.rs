//! Gitpod configuration generator.
//!
//! Generates:
//! - .gitpod.yml
//! - .gitpod.Dockerfile (optional)

use super::{CloudGenerator, GeneratedFile};
use crate::{Result, WorkspaceConfig};
use std::fs;
use std::path::Path;

/// Gitpod configuration generator.
#[derive(Debug, Default)]
pub struct GitpodGenerator {
    /// Whether to generate a custom Dockerfile.
    pub custom_dockerfile: bool,
}

impl GitpodGenerator {
    /// Create a new Gitpod generator.
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

    /// Generate .gitpod.yml content.
    fn generate_config(&self, config: &WorkspaceConfig) -> String {
        let mut lines = Vec::new();

        // Image
        if self.custom_dockerfile {
            lines.push("image:".to_string());
            lines.push("  file: .gitpod.Dockerfile".to_string());
        } else if config.detected_features.is_cargo_project {
            lines.push("image: gitpod/workspace-rust".to_string());
        } else {
            lines.push("image: gitpod/workspace-full".to_string());
        }

        lines.push(String::new());

        // Tasks
        lines.push("tasks:".to_string());

        // Init task for dx projects
        if config.detected_features.is_cargo_project {
            lines.push("  - name: Setup".to_string());
            lines.push("    init: |".to_string());
            lines.push("      cargo fetch".to_string());
            lines.push("      cargo build".to_string());
            lines.push("    command: |".to_string());
            lines.push("      echo 'dx workspace ready!'".to_string());
        }

        // Add configured tasks
        for task in &config.tasks.tasks {
            lines.push(format!("  - name: {}", task.label));
            if task.is_background {
                let cmd = format!("{} {}", task.command, task.args.join(" "));
                lines.push(format!("    command: {}", cmd));
            } else {
                lines.push("    init: |".to_string());
                lines.push(format!("      {} {}", task.command, task.args.join(" ")));
            }
        }

        lines.push(String::new());

        // Ports
        if config.detected_features.has_dx_server || config.detected_features.has_dx_www {
            lines.push("ports:".to_string());
            lines.push("  - port: 3000".to_string());
            lines.push("    onOpen: open-preview".to_string());
            lines.push("  - port: 8080".to_string());
            lines.push("    onOpen: ignore".to_string());
        }

        lines.push(String::new());

        // VS Code extensions
        if !config.extensions.core.is_empty() || !config.extensions.recommended.is_empty() {
            lines.push("vscode:".to_string());
            lines.push("  extensions:".to_string());

            for ext in &config.extensions.core {
                lines.push(format!("    - {}", ext.id));
            }
            for ext in &config.extensions.recommended {
                lines.push(format!("    - {}", ext.id));
            }
        }

        // GitHub integration
        lines.push(String::new());
        lines.push("github:".to_string());
        lines.push("  prebuilds:".to_string());
        lines.push("    master: true".to_string());
        lines.push("    branches: true".to_string());
        lines.push("    pullRequests: true".to_string());
        lines.push("    pullRequestsFromForks: false".to_string());
        lines.push("    addCheck: true".to_string());
        lines.push("    addComment: false".to_string());
        lines.push("    addBadge: true".to_string());

        lines.join("\n")
    }

    /// Generate .gitpod.Dockerfile content.
    fn generate_dockerfile(&self, config: &WorkspaceConfig) -> String {
        let mut lines = Vec::new();

        if config.detected_features.is_cargo_project {
            lines.push("FROM gitpod/workspace-rust".to_string());
        } else {
            lines.push("FROM gitpod/workspace-full".to_string());
        }

        lines.push(String::new());

        // Install dx CLI
        lines.push("# Install dx CLI".to_string());
        lines.push("RUN cargo install dx-cli || true".to_string());

        lines.push(String::new());

        // Install additional tools for dx development
        if config.detected_features.has_dx_client {
            lines.push("# Install WASM target".to_string());
            lines.push("RUN rustup target add wasm32-unknown-unknown".to_string());
        }

        lines.join("\n")
    }
}

impl CloudGenerator for GitpodGenerator {
    fn generate(&self, config: &WorkspaceConfig, output_dir: &Path) -> Result<Vec<GeneratedFile>> {
        let mut files = Vec::new();

        // Generate .gitpod.yml
        let gitpod_yml = self.generate_config(config);
        files.push(GeneratedFile::new(".gitpod.yml", gitpod_yml.clone()));

        let yml_path = output_dir.join(".gitpod.yml");
        fs::write(&yml_path, &gitpod_yml).map_err(|e| crate::Error::io(&yml_path, e))?;

        // Generate Dockerfile if requested
        if self.custom_dockerfile {
            let dockerfile = self.generate_dockerfile(config);
            files.push(GeneratedFile::new(".gitpod.Dockerfile", dockerfile.clone()));

            let docker_path = output_dir.join(".gitpod.Dockerfile");
            fs::write(&docker_path, &dockerfile).map_err(|e| crate::Error::io(&docker_path, e))?;
        }

        Ok(files)
    }

    fn exists(&self, project_dir: &Path) -> bool {
        project_dir.join(".gitpod.yml").exists()
    }

    fn clean(&self, project_dir: &Path) -> Result<()> {
        let yml_path = project_dir.join(".gitpod.yml");
        if yml_path.exists() {
            fs::remove_file(&yml_path).map_err(|e| crate::Error::io(&yml_path, e))?;
        }

        let docker_path = project_dir.join(".gitpod.Dockerfile");
        if docker_path.exists() {
            fs::remove_file(&docker_path).map_err(|e| crate::Error::io(&docker_path, e))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ExtensionRecommendations, TaskConfig};

    #[test]
    fn test_generate_config() {
        let mut config = WorkspaceConfig::new("test");
        config.detected_features.is_cargo_project = true;
        config.tasks = TaskConfig::dx_defaults();
        config.extensions = ExtensionRecommendations::dx_defaults();

        let generator = GitpodGenerator::new();
        let content = generator.generate_config(&config);

        assert!(content.contains("gitpod/workspace-rust"));
        assert!(content.contains("tasks:"));
        assert!(content.contains("vscode:"));
        assert!(content.contains("extensions:"));
    }

    #[test]
    fn test_generate_dockerfile() {
        let mut config = WorkspaceConfig::new("test");
        config.detected_features.is_cargo_project = true;
        config.detected_features.has_dx_client = true;

        let generator = GitpodGenerator::new().with_dockerfile();
        let content = generator.generate_dockerfile(&config);

        assert!(content.contains("FROM gitpod/workspace-rust"));
        assert!(content.contains("wasm32-unknown-unknown"));
    }
}
