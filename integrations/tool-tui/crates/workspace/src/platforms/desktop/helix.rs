//! Helix editor configuration generator.

use super::{DesktopGenerator, GeneratedFile};
use crate::{Result, WorkspaceConfig};
use std::fs;
use std::path::Path;

/// Helix editor configuration generator.
#[derive(Debug, Default)]
pub struct HelixGenerator;

impl HelixGenerator {
    /// Create a new Helix generator.
    pub fn new() -> Self {
        Self
    }

    /// Generate config.toml content.
    fn generate_config(&self, config: &WorkspaceConfig) -> String {
        let mut lines = Vec::new();

        lines.push("# dx-workspace generated Helix configuration".to_string());
        lines.push(String::new());

        lines.push("[editor]".to_string());
        lines.push(format!("line-number = \"{}\"", "relative"));
        lines.push("mouse = true".to_string());
        lines.push("auto-format = true".to_string());
        lines.push("auto-save = false".to_string());

        lines.push(String::new());
        lines.push("[editor.cursor-shape]".to_string());
        lines.push("insert = \"bar\"".to_string());
        lines.push("normal = \"block\"".to_string());
        lines.push("select = \"underline\"".to_string());

        lines.push(String::new());
        lines.push("[editor.indent-guides]".to_string());
        lines.push("render = true".to_string());

        lines.push(String::new());
        lines.push("[editor.statusline]".to_string());
        lines.push("left = [\"mode\", \"spinner\", \"file-name\"]".to_string());
        lines.push("right = [\"diagnostics\", \"position\", \"file-encoding\"]".to_string());

        if let Some(ref theme) = config.editor.theme {
            lines.push(String::new());
            lines.push(format!("theme = \"{}\"", theme));
        }

        lines.join("\n")
    }

    /// Generate languages.toml content.
    fn generate_languages(&self, config: &WorkspaceConfig) -> String {
        let mut lines = Vec::new();

        lines.push("# dx-workspace generated Helix languages configuration".to_string());
        lines.push(String::new());

        // Rust configuration
        if config.detected_features.is_cargo_project {
            lines.push("[[language]]".to_string());
            lines.push("name = \"rust\"".to_string());
            lines.push("auto-format = true".to_string());
            lines.push(String::new());

            lines.push("[language.config.rust-analyzer]".to_string());
            lines.push("[language.config.rust-analyzer.cargo]".to_string());
            lines.push("features = \"all\"".to_string());
            lines.push(String::new());

            lines.push("[language.config.rust-analyzer.checkOnSave]".to_string());
            lines.push("command = \"clippy\"".to_string());
        }

        // TypeScript configuration
        if config.detected_features.uses_typescript {
            if !lines.is_empty() {
                lines.push(String::new());
            }
            lines.push("[[language]]".to_string());
            lines.push("name = \"typescript\"".to_string());
            lines.push("auto-format = true".to_string());
            lines.push(String::new());

            lines.push("[[language]]".to_string());
            lines.push("name = \"tsx\"".to_string());
            lines.push("auto-format = true".to_string());
        }

        lines.join("\n")
    }
}

impl DesktopGenerator for HelixGenerator {
    fn generate(&self, config: &WorkspaceConfig, output_dir: &Path) -> Result<Vec<GeneratedFile>> {
        let mut files = Vec::new();

        let helix_dir = output_dir.join(".helix");
        fs::create_dir_all(&helix_dir).map_err(|e| crate::Error::io(&helix_dir, e))?;

        // Generate config.toml
        let config_content = self.generate_config(config);
        files.push(GeneratedFile::new(".helix/config.toml", config_content.clone()));

        let config_path = helix_dir.join("config.toml");
        fs::write(&config_path, &config_content).map_err(|e| crate::Error::io(&config_path, e))?;

        // Generate languages.toml
        let languages_content = self.generate_languages(config);
        files.push(GeneratedFile::new(".helix/languages.toml", languages_content.clone()));

        let languages_path = helix_dir.join("languages.toml");
        fs::write(&languages_path, &languages_content)
            .map_err(|e| crate::Error::io(&languages_path, e))?;

        Ok(files)
    }

    fn exists(&self, project_dir: &Path) -> bool {
        project_dir.join(".helix").exists()
    }

    fn clean(&self, project_dir: &Path) -> Result<()> {
        let helix_dir = project_dir.join(".helix");
        if helix_dir.exists() {
            fs::remove_dir_all(&helix_dir).map_err(|e| crate::Error::io(&helix_dir, e))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_config() {
        let config = WorkspaceConfig::new("test");
        let generator = HelixGenerator::new();
        let content = generator.generate_config(&config);

        assert!(content.contains("[editor]"));
        assert!(content.contains("auto-format = true"));
    }

    #[test]
    fn test_generate_languages() {
        let mut config = WorkspaceConfig::new("test");
        config.detected_features.is_cargo_project = true;

        let generator = HelixGenerator::new();
        let content = generator.generate_languages(&config);

        assert!(content.contains("name = \"rust\""));
        assert!(content.contains("clippy"));
    }
}
