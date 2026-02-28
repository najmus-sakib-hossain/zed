//! Unified configuration generator.
//!
//! This module provides the main Generator type that can produce configurations
//! for any supported platform from a WorkspaceConfig.

use crate::platforms::{
    CloudGenerator, CodeSandboxGenerator, CodespacesGenerator, ContainerGenerator,
    DesktopGenerator, DockerComposeGenerator, FirebaseStudioGenerator, GitpodGenerator,
    HelixGenerator, IntelliJGenerator, NeovimGenerator, NixFlakesGenerator, Platform,
    ReplitGenerator, StackBlitzGenerator, SublimeGenerator, VsCodeGenerator, ZedGenerator,
};
use crate::{Result, WorkspaceConfig};
use std::path::{Path, PathBuf};

/// Result of generating configuration for a platform.
#[derive(Debug)]
pub struct GenerationResult {
    /// Platform that was generated.
    pub platform: Platform,
    /// Files that were created or updated.
    pub files: Vec<GeneratedFileInfo>,
    /// Whether generation was successful.
    pub success: bool,
    /// Any error message.
    pub error: Option<String>,
}

/// Information about a generated file.
#[derive(Debug, Clone)]
pub struct GeneratedFileInfo {
    /// Path relative to project root.
    pub path: String,
    /// Whether the file was newly created.
    pub is_new: bool,
}

/// Main generator for producing platform-specific configurations.
pub struct Generator<'a> {
    config: &'a WorkspaceConfig,
    output_dir: PathBuf,
}

impl<'a> Generator<'a> {
    /// Create a new generator for the given workspace configuration.
    pub fn new(config: &'a WorkspaceConfig) -> Self {
        Self {
            config,
            output_dir: config.root.clone(),
        }
    }

    /// Create a generator with a custom output directory.
    pub fn with_output_dir(config: &'a WorkspaceConfig, output_dir: impl Into<PathBuf>) -> Self {
        Self {
            config,
            output_dir: output_dir.into(),
        }
    }

    /// Generate configuration for a specific platform.
    pub fn generate(&self, platform: Platform) -> Result<GenerationResult> {
        let files = match platform {
            // Desktop editors
            Platform::VsCode => self.generate_vscode()?,
            Platform::Zed => self.generate_zed()?,
            Platform::Neovim => self.generate_neovim()?,
            Platform::IntelliJ => self.generate_intellij()?,
            Platform::Helix => self.generate_helix()?,
            Platform::SublimeText => self.generate_sublime()?,

            // Cloud IDEs
            Platform::Codespaces | Platform::DevContainer => self.generate_codespaces()?,
            Platform::Gitpod => self.generate_gitpod()?,
            Platform::CodeSandbox => self.generate_codesandbox()?,
            Platform::FirebaseStudio => self.generate_firebase_studio()?,
            Platform::StackBlitz => self.generate_stackblitz()?,
            Platform::Replit => self.generate_replit()?,

            // Container environments
            Platform::DockerCompose | Platform::Podman => self.generate_docker_compose()?,
            Platform::NixFlakes => self.generate_nix_flakes()?,

            // Not yet implemented
            _ => {
                return Err(crate::Error::unsupported_platform(platform.display_name()));
            }
        };

        Ok(GenerationResult {
            platform,
            files,
            success: true,
            error: None,
        })
    }

    /// Generate configurations for all supported platforms.
    pub fn generate_all(&self) -> Vec<GenerationResult> {
        let mut results = Vec::new();

        for platform in Platform::all() {
            match self.generate(platform) {
                Ok(result) => results.push(result),
                Err(e) => results.push(GenerationResult {
                    platform,
                    files: vec![],
                    success: false,
                    error: Some(e.to_string()),
                }),
            }
        }

        results
    }

    /// Generate configurations for desktop editors only.
    pub fn generate_desktop(&self) -> Vec<GenerationResult> {
        Platform::desktop_editors()
            .iter()
            .map(|&p| match self.generate(p) {
                Ok(r) => r,
                Err(e) => GenerationResult {
                    platform: p,
                    files: vec![],
                    success: false,
                    error: Some(e.to_string()),
                },
            })
            .collect()
    }

    /// Generate configurations for cloud IDEs only.
    pub fn generate_cloud(&self) -> Vec<GenerationResult> {
        Platform::cloud_ides()
            .iter()
            .map(|&p| match self.generate(p) {
                Ok(r) => r,
                Err(e) => GenerationResult {
                    platform: p,
                    files: vec![],
                    success: false,
                    error: Some(e.to_string()),
                },
            })
            .collect()
    }

    /// Check if configuration exists for a platform.
    pub fn exists(&self, platform: Platform) -> bool {
        match platform {
            Platform::VsCode => VsCodeGenerator::new().exists(&self.output_dir),
            Platform::Zed => ZedGenerator::new().exists(&self.output_dir),
            Platform::Neovim => NeovimGenerator::new().exists(&self.output_dir),
            Platform::IntelliJ => IntelliJGenerator::new().exists(&self.output_dir),
            Platform::Helix => HelixGenerator::new().exists(&self.output_dir),
            Platform::SublimeText => SublimeGenerator::new().exists(&self.output_dir),
            Platform::Codespaces | Platform::DevContainer => {
                CodespacesGenerator::new().exists(&self.output_dir)
            }
            Platform::Gitpod => GitpodGenerator::new().exists(&self.output_dir),
            Platform::CodeSandbox => CodeSandboxGenerator::new().exists(&self.output_dir),
            Platform::FirebaseStudio => FirebaseStudioGenerator::new().exists(&self.output_dir),
            Platform::StackBlitz => StackBlitzGenerator::new().exists(&self.output_dir),
            Platform::Replit => ReplitGenerator::new().exists(&self.output_dir),
            Platform::DockerCompose | Platform::Podman => {
                DockerComposeGenerator::new().exists(&self.output_dir)
            }
            Platform::NixFlakes => NixFlakesGenerator::new().exists(&self.output_dir),
            _ => false,
        }
    }

    /// Clean generated configuration for a platform.
    pub fn clean(&self, platform: Platform) -> Result<()> {
        match platform {
            Platform::VsCode => VsCodeGenerator::new().clean(&self.output_dir),
            Platform::Zed => ZedGenerator::new().clean(&self.output_dir),
            Platform::Neovim => NeovimGenerator::new().clean(&self.output_dir),
            Platform::IntelliJ => IntelliJGenerator::new().clean(&self.output_dir),
            Platform::Helix => HelixGenerator::new().clean(&self.output_dir),
            Platform::SublimeText => SublimeGenerator::new().clean(&self.output_dir),
            Platform::Codespaces | Platform::DevContainer => {
                CodespacesGenerator::new().clean(&self.output_dir)
            }
            Platform::Gitpod => GitpodGenerator::new().clean(&self.output_dir),
            Platform::CodeSandbox => CodeSandboxGenerator::new().clean(&self.output_dir),
            Platform::FirebaseStudio => FirebaseStudioGenerator::new().clean(&self.output_dir),
            Platform::StackBlitz => StackBlitzGenerator::new().clean(&self.output_dir),
            Platform::Replit => ReplitGenerator::new().clean(&self.output_dir),
            Platform::DockerCompose | Platform::Podman => {
                DockerComposeGenerator::new().clean(&self.output_dir)
            }
            Platform::NixFlakes => NixFlakesGenerator::new().clean(&self.output_dir),
            _ => Err(crate::Error::unsupported_platform(platform.display_name())),
        }
    }

    /// Clean all generated configurations.
    pub fn clean_all(&self) -> Vec<Result<()>> {
        Platform::all().iter().map(|&p| self.clean(p)).collect()
    }

    // Private generator methods

    fn generate_vscode(&self) -> Result<Vec<GeneratedFileInfo>> {
        let generator = VsCodeGenerator::new();
        let files = generator.generate(self.config, &self.output_dir)?;
        Ok(files
            .into_iter()
            .map(|f| GeneratedFileInfo {
                path: f.path,
                is_new: f.is_new,
            })
            .collect())
    }

    fn generate_zed(&self) -> Result<Vec<GeneratedFileInfo>> {
        let generator = ZedGenerator::new();
        let files = generator.generate(self.config, &self.output_dir)?;
        Ok(files
            .into_iter()
            .map(|f| GeneratedFileInfo {
                path: f.path,
                is_new: f.is_new,
            })
            .collect())
    }

    fn generate_neovim(&self) -> Result<Vec<GeneratedFileInfo>> {
        let generator = NeovimGenerator::new();
        let files = generator.generate(self.config, &self.output_dir)?;
        Ok(files
            .into_iter()
            .map(|f| GeneratedFileInfo {
                path: f.path,
                is_new: f.is_new,
            })
            .collect())
    }

    fn generate_intellij(&self) -> Result<Vec<GeneratedFileInfo>> {
        let generator = IntelliJGenerator::new();
        let files = generator.generate(self.config, &self.output_dir)?;
        Ok(files
            .into_iter()
            .map(|f| GeneratedFileInfo {
                path: f.path,
                is_new: f.is_new,
            })
            .collect())
    }

    fn generate_helix(&self) -> Result<Vec<GeneratedFileInfo>> {
        let generator = HelixGenerator::new();
        let files = generator.generate(self.config, &self.output_dir)?;
        Ok(files
            .into_iter()
            .map(|f| GeneratedFileInfo {
                path: f.path,
                is_new: f.is_new,
            })
            .collect())
    }

    fn generate_sublime(&self) -> Result<Vec<GeneratedFileInfo>> {
        let generator = SublimeGenerator::new();
        let files = generator.generate(self.config, &self.output_dir)?;
        Ok(files
            .into_iter()
            .map(|f| GeneratedFileInfo {
                path: f.path,
                is_new: f.is_new,
            })
            .collect())
    }

    fn generate_codespaces(&self) -> Result<Vec<GeneratedFileInfo>> {
        let generator = CodespacesGenerator::new();
        let files = generator.generate(self.config, &self.output_dir)?;
        Ok(files
            .into_iter()
            .map(|f| GeneratedFileInfo {
                path: f.path,
                is_new: f.is_new,
            })
            .collect())
    }

    fn generate_gitpod(&self) -> Result<Vec<GeneratedFileInfo>> {
        let generator = GitpodGenerator::new();
        let files = generator.generate(self.config, &self.output_dir)?;
        Ok(files
            .into_iter()
            .map(|f| GeneratedFileInfo {
                path: f.path,
                is_new: f.is_new,
            })
            .collect())
    }

    fn generate_codesandbox(&self) -> Result<Vec<GeneratedFileInfo>> {
        let generator = CodeSandboxGenerator::new();
        let files = generator.generate(self.config, &self.output_dir)?;
        Ok(files
            .into_iter()
            .map(|f| GeneratedFileInfo {
                path: f.path,
                is_new: f.is_new,
            })
            .collect())
    }

    fn generate_firebase_studio(&self) -> Result<Vec<GeneratedFileInfo>> {
        let generator = FirebaseStudioGenerator::new();
        let files = generator.generate(self.config, &self.output_dir)?;
        Ok(files
            .into_iter()
            .map(|f| GeneratedFileInfo {
                path: f.path,
                is_new: f.is_new,
            })
            .collect())
    }

    fn generate_stackblitz(&self) -> Result<Vec<GeneratedFileInfo>> {
        let generator = StackBlitzGenerator::new();
        let files = generator.generate(self.config, &self.output_dir)?;
        Ok(files
            .into_iter()
            .map(|f| GeneratedFileInfo {
                path: f.path,
                is_new: f.is_new,
            })
            .collect())
    }

    fn generate_replit(&self) -> Result<Vec<GeneratedFileInfo>> {
        let generator = ReplitGenerator::new();
        let files = generator.generate(self.config, &self.output_dir)?;
        Ok(files
            .into_iter()
            .map(|f| GeneratedFileInfo {
                path: f.path,
                is_new: f.is_new,
            })
            .collect())
    }

    fn generate_docker_compose(&self) -> Result<Vec<GeneratedFileInfo>> {
        let generator = DockerComposeGenerator::new();
        let files = generator.generate(self.config, &self.output_dir)?;
        Ok(files
            .into_iter()
            .map(|f| GeneratedFileInfo {
                path: f.path,
                is_new: f.is_new,
            })
            .collect())
    }

    fn generate_nix_flakes(&self) -> Result<Vec<GeneratedFileInfo>> {
        let generator = NixFlakesGenerator::new();
        let files = generator.generate(self.config, &self.output_dir)?;
        Ok(files
            .into_iter()
            .map(|f| GeneratedFileInfo {
                path: f.path,
                is_new: f.is_new,
            })
            .collect())
    }
}

/// Detect current development environment.
pub fn detect_current_environment(project_dir: &Path) -> Option<Platform> {
    // Check environment variables for cloud IDE detection
    if std::env::var("GITPOD_WORKSPACE_ID").is_ok() {
        return Some(Platform::Gitpod);
    }

    if std::env::var("CODESPACES").is_ok() {
        return Some(Platform::Codespaces);
    }

    if std::env::var("CODESANDBOX_SSE").is_ok() {
        return Some(Platform::CodeSandbox);
    }

    if std::env::var("REPL_ID").is_ok() {
        return Some(Platform::Replit);
    }

    if std::env::var("IDX_CHANNEL").is_ok() {
        return Some(Platform::FirebaseStudio);
    }

    // Check for existing configuration files
    if project_dir.join(".vscode").exists() {
        return Some(Platform::VsCode);
    }

    if project_dir.join(".zed").exists() {
        return Some(Platform::Zed);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_generator_new() {
        let config = WorkspaceConfig::new("test");
        let generator = Generator::new(&config);
        assert_eq!(generator.config.name, "test");
    }

    #[test]
    fn test_generate_vscode() {
        let dir = tempdir().unwrap();
        let mut config = WorkspaceConfig::new("test");
        config.root = dir.path().to_path_buf();

        let generator = Generator::new(&config);
        let result = generator.generate(Platform::VsCode).unwrap();

        assert!(result.success);
        assert!(result.files.iter().any(|f| f.path.contains("settings.json")));
    }
}
