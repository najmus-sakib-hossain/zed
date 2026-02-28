//! CodeSandbox configuration generator.
//!
//! Generates:
//! - .codesandbox/tasks.json
//! - sandbox.config.json

use super::{CloudGenerator, GeneratedFile};
use crate::{Result, WorkspaceConfig};
use serde_json::{Map, Value, json};
use std::fs;
use std::path::Path;

/// CodeSandbox configuration generator.
#[derive(Debug, Default)]
pub struct CodeSandboxGenerator;

impl CodeSandboxGenerator {
    /// Create a new CodeSandbox generator.
    pub fn new() -> Self {
        Self
    }

    /// Generate .codesandbox/tasks.json content.
    fn generate_tasks(&self, config: &WorkspaceConfig) -> Value {
        let mut setup_tasks = Map::new();

        // Setup tasks
        if config.detected_features.is_cargo_project {
            setup_tasks.insert(
                "install-deps".to_string(),
                json!({
                    "name": "Install Dependencies",
                    "command": "cargo fetch"
                }),
            );
        }

        let mut tasks = Map::new();

        // Add configured tasks
        for task in &config.tasks.tasks {
            let cmd = format!("{} {}", task.command, task.args.join(" "));
            tasks.insert(
                task.label.to_lowercase().replace(' ', "-"),
                json!({
                    "name": task.label,
                    "command": cmd,
                    "runAtStart": task.is_background
                }),
            );
        }

        json!({
            "setupTasks": [setup_tasks],
            "tasks": tasks
        })
    }

    /// Generate sandbox.config.json content.
    fn generate_sandbox_config(&self, config: &WorkspaceConfig) -> Value {
        let mut sandbox = Map::new();

        // Template based on project type
        if config.detected_features.is_cargo_project {
            sandbox.insert("template".to_string(), json!("rust"));
        } else if config.detected_features.uses_typescript {
            sandbox.insert("template".to_string(), json!("node"));
        }

        // Container configuration
        sandbox.insert(
            "container".to_string(),
            json!({
                "node": "18",
                "startScript": "dev"
            }),
        );

        // Infinite loop protection
        sandbox.insert("infiniteLoopProtection".to_string(), json!(true));

        // Hard reload on change for dx projects
        sandbox.insert("hardReloadOnChange".to_string(), json!(false));

        // View
        sandbox.insert("view".to_string(), json!("browser"));

        json!(sandbox)
    }
}

impl CloudGenerator for CodeSandboxGenerator {
    fn generate(&self, config: &WorkspaceConfig, output_dir: &Path) -> Result<Vec<GeneratedFile>> {
        let mut files = Vec::new();

        // Create .codesandbox directory
        let codesandbox_dir = output_dir.join(".codesandbox");
        fs::create_dir_all(&codesandbox_dir).map_err(|e| crate::Error::io(&codesandbox_dir, e))?;

        // Generate tasks.json
        let tasks = self.generate_tasks(config);
        let tasks_content = serde_json::to_string_pretty(&tasks).unwrap_or_default();
        files.push(GeneratedFile::new(".codesandbox/tasks.json", tasks_content.clone()));

        let tasks_path = codesandbox_dir.join("tasks.json");
        fs::write(&tasks_path, &tasks_content).map_err(|e| crate::Error::io(&tasks_path, e))?;

        // Generate sandbox.config.json
        let sandbox = self.generate_sandbox_config(config);
        let sandbox_content = serde_json::to_string_pretty(&sandbox).unwrap_or_default();
        files.push(GeneratedFile::new("sandbox.config.json", sandbox_content.clone()));

        let sandbox_path = output_dir.join("sandbox.config.json");
        fs::write(&sandbox_path, &sandbox_content)
            .map_err(|e| crate::Error::io(&sandbox_path, e))?;

        Ok(files)
    }

    fn exists(&self, project_dir: &Path) -> bool {
        project_dir.join(".codesandbox").exists()
            || project_dir.join("sandbox.config.json").exists()
    }

    fn clean(&self, project_dir: &Path) -> Result<()> {
        let codesandbox_dir = project_dir.join(".codesandbox");
        if codesandbox_dir.exists() {
            fs::remove_dir_all(&codesandbox_dir)
                .map_err(|e| crate::Error::io(&codesandbox_dir, e))?;
        }

        let sandbox_config = project_dir.join("sandbox.config.json");
        if sandbox_config.exists() {
            fs::remove_file(&sandbox_config).map_err(|e| crate::Error::io(&sandbox_config, e))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TaskConfig;

    #[test]
    fn test_generate_tasks() {
        let mut config = WorkspaceConfig::new("test");
        config.detected_features.is_cargo_project = true;
        config.tasks = TaskConfig::dx_defaults();

        let generator = CodeSandboxGenerator::new();
        let tasks = generator.generate_tasks(&config);

        assert!(tasks["setupTasks"].is_array());
        assert!(tasks["tasks"].is_object());
    }

    #[test]
    fn test_generate_sandbox_config() {
        let mut config = WorkspaceConfig::new("test");
        config.detected_features.is_cargo_project = true;

        let generator = CodeSandboxGenerator::new();
        let sandbox = generator.generate_sandbox_config(&config);

        assert_eq!(sandbox["template"], "rust");
    }
}
