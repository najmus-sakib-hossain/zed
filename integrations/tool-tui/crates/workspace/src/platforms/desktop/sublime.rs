//! Sublime Text configuration generator.

use super::{DesktopGenerator, GeneratedFile};
use crate::{Result, WorkspaceConfig};
use serde_json::{Map, Value, json};
use std::fs;
use std::path::Path;

/// Sublime Text configuration generator.
#[derive(Debug, Default)]
pub struct SublimeGenerator;

impl SublimeGenerator {
    /// Create a new Sublime Text generator.
    pub fn new() -> Self {
        Self
    }

    /// Generate sublime-project content.
    fn generate_project(&self, config: &WorkspaceConfig) -> Value {
        let mut project = Map::new();

        // Folders
        project.insert(
            "folders".to_string(),
            json!([
                {
                    "path": ".",
                    "folder_exclude_patterns": ["target", "node_modules", ".git"],
                    "file_exclude_patterns": ["*.lock"]
                }
            ]),
        );

        // Settings
        let mut settings = Map::new();
        settings.insert("tab_size".to_string(), json!(config.editor.tab_size));
        settings.insert("translate_tabs_to_spaces".to_string(), json!(config.editor.insert_spaces));

        if let Some(ref font) = config.editor.font_family {
            settings.insert("font_face".to_string(), json!(font));
        }
        if let Some(size) = config.editor.font_size {
            settings.insert("font_size".to_string(), json!(size));
        }
        if let Some(ref theme) = config.editor.theme {
            settings.insert("color_scheme".to_string(), json!(theme));
        }

        // Rust settings
        if config.detected_features.is_cargo_project {
            settings.insert(
                "LSP".to_string(),
                json!({
                    "rust-analyzer": {
                        "enabled": true,
                        "settings": {
                            "cargo": {
                                "features": "all"
                            },
                            "checkOnSave": {
                                "command": "clippy"
                            }
                        }
                    }
                }),
            );
        }

        project.insert("settings".to_string(), json!(settings));

        // Build systems
        let mut build_systems = Vec::new();

        for task in &config.tasks.tasks {
            build_systems.push(json!({
                "name": task.label,
                "shell_cmd": format!("{} {}", task.command, task.args.join(" ")),
                "working_dir": "${project_path}",
                "file_regex": "^(.+):(\\d+):(\\d+): (error|warning): (.*)$"
            }));
        }

        if !build_systems.is_empty() {
            project.insert("build_systems".to_string(), json!(build_systems));
        }

        json!(project)
    }
}

impl DesktopGenerator for SublimeGenerator {
    fn generate(&self, config: &WorkspaceConfig, output_dir: &Path) -> Result<Vec<GeneratedFile>> {
        let mut files = Vec::new();

        let project = self.generate_project(config);
        let filename = format!("{}.sublime-project", config.name);
        let content = serde_json::to_string_pretty(&project).unwrap_or_default();

        files.push(GeneratedFile::new(&filename, content.clone()));

        let path = output_dir.join(&filename);
        fs::write(&path, &content).map_err(|e| crate::Error::io(&path, e))?;

        Ok(files)
    }

    fn exists(&self, project_dir: &Path) -> bool {
        // Check for any .sublime-project file
        if let Ok(entries) = fs::read_dir(project_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().is_some_and(|ext| ext == "sublime-project") {
                    return true;
                }
            }
        }
        false
    }

    fn clean(&self, project_dir: &Path) -> Result<()> {
        // Remove dx-generated sublime project files
        if let Ok(entries) = fs::read_dir(project_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "sublime-project") {
                    fs::remove_file(&path).map_err(|e| crate::Error::io(&path, e))?;
                }
                if path.extension().is_some_and(|ext| ext == "sublime-workspace") {
                    fs::remove_file(&path).map_err(|e| crate::Error::io(&path, e))?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TaskConfig;

    #[test]
    fn test_generate_project() {
        let mut config = WorkspaceConfig::new("test");
        config.tasks = TaskConfig::dx_defaults();
        config.detected_features.is_cargo_project = true;

        let generator = SublimeGenerator::new();
        let project = generator.generate_project(&config);

        assert!(project["folders"].is_array());
        assert!(project["settings"]["LSP"]["rust-analyzer"]["enabled"].as_bool().unwrap());
        assert!(project["build_systems"].is_array());
    }
}
