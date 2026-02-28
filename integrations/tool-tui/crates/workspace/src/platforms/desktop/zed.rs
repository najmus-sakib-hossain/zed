//! Zed editor configuration generator.

use super::{DesktopGenerator, GeneratedFile};
use crate::{Result, WorkspaceConfig};
use serde_json::{Map, Value, json};
use std::fs;
use std::path::Path;

/// Zed editor configuration generator.
#[derive(Debug, Default)]
pub struct ZedGenerator;

impl ZedGenerator {
    /// Create a new Zed generator.
    pub fn new() -> Self {
        Self
    }

    /// Generate Zed settings.json content.
    fn generate_settings(&self, config: &WorkspaceConfig) -> Value {
        let editor = &config.editor;
        let mut settings = Map::new();

        // Tab settings
        settings.insert("tab_size".to_string(), json!(editor.tab_size));
        settings.insert("hard_tabs".to_string(), json!(!editor.insert_spaces));

        // Font settings
        if let Some(ref font) = editor.font_family {
            settings.insert("buffer_font_family".to_string(), json!(font));
        }
        if let Some(size) = editor.font_size {
            settings.insert("buffer_font_size".to_string(), json!(size));
        }
        if let Some(height) = editor.line_height {
            settings.insert("buffer_line_height".to_string(), json!(height));
        }

        // Theme
        if let Some(ref theme) = editor.theme {
            settings.insert("theme".to_string(), json!(theme));
        }

        // Vim mode
        if editor.keybinding_style == crate::config::KeybindingStyle::Vim {
            settings.insert("vim_mode".to_string(), json!(true));
        }

        // Rust-specific settings for dx projects
        if config.detected_features.is_cargo_project {
            settings.insert(
                "lsp".to_string(),
                json!({
                    "rust-analyzer": {
                        "initialization_options": {
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

        // Format on save
        settings.insert("format_on_save".to_string(), json!("on"));

        // File types for dx
        settings.insert(
            "file_types".to_string(),
            json!({
                "Rust": ["rs"],
                "TOML": ["toml"],
                "TSX": ["tsx"],
                "TypeScript": ["ts"]
            }),
        );

        json!(settings)
    }

    /// Generate Zed tasks.json content.
    fn generate_tasks(&self, config: &WorkspaceConfig) -> Value {
        let tasks: Vec<Value> = config
            .tasks
            .tasks
            .iter()
            .map(|task| {
                json!({
                    "label": task.label,
                    "command": task.command,
                    "args": task.args,
                    "cwd": task.cwd,
                    "use_new_terminal": !task.is_background,
                    "allow_concurrent_runs": task.is_background
                })
            })
            .collect();

        json!(tasks)
    }
}

impl DesktopGenerator for ZedGenerator {
    fn generate(&self, config: &WorkspaceConfig, output_dir: &Path) -> Result<Vec<GeneratedFile>> {
        let mut files = Vec::new();

        // Generate settings.json
        let settings = self.generate_settings(config);
        files.push(GeneratedFile::new(
            ".zed/settings.json",
            serde_json::to_string_pretty(&settings).unwrap_or_default(),
        ));

        // Generate tasks.json
        let tasks = self.generate_tasks(config);
        files.push(GeneratedFile::new(
            ".zed/tasks.json",
            serde_json::to_string_pretty(&tasks).unwrap_or_default(),
        ));

        // Write files
        let zed_dir = output_dir.join(".zed");
        fs::create_dir_all(&zed_dir).map_err(|e| crate::Error::io(&zed_dir, e))?;

        for file in &files {
            let path = output_dir.join(&file.path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| crate::Error::io(parent, e))?;
            }
            fs::write(&path, &file.content).map_err(|e| crate::Error::io(&path, e))?;
        }

        Ok(files)
    }

    fn exists(&self, project_dir: &Path) -> bool {
        project_dir.join(".zed").exists()
    }

    fn clean(&self, project_dir: &Path) -> Result<()> {
        let zed_dir = project_dir.join(".zed");
        if zed_dir.exists() {
            fs::remove_dir_all(&zed_dir).map_err(|e| crate::Error::io(&zed_dir, e))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_settings() {
        let mut config = WorkspaceConfig::new("test");
        config.editor.tab_size = 2;
        config.detected_features.is_cargo_project = true;

        let generator = ZedGenerator::new();
        let settings = generator.generate_settings(&config);

        assert_eq!(settings["tab_size"], 2);
        assert!(settings["lsp"]["rust-analyzer"].is_object());
    }
}
