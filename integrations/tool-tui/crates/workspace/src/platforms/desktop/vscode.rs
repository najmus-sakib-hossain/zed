//! VS Code / VS Codium configuration generator.
//!
//! Generates:
//! - .vscode/settings.json
//! - .vscode/tasks.json
//! - .vscode/launch.json
//! - .vscode/extensions.json

use super::{DesktopGenerator, GeneratedFile};
use crate::config::{TaskGroup, TaskPanel, TaskReveal};
use crate::{Result, WorkspaceConfig};
use serde_json::{Map, Value, json};
use std::fs;
use std::path::Path;

/// VS Code configuration generator.
#[derive(Debug, Default)]
pub struct VsCodeGenerator;

impl VsCodeGenerator {
    /// Create a new VS Code generator.
    pub fn new() -> Self {
        Self
    }

    /// Generate settings.json content.
    fn generate_settings(&self, config: &WorkspaceConfig) -> Value {
        let editor = &config.editor;
        let project = &config.project;

        let mut settings = Map::new();

        // Editor settings
        settings.insert("editor.tabSize".to_string(), json!(editor.tab_size));
        settings.insert("editor.insertSpaces".to_string(), json!(editor.insert_spaces));

        if let Some(ref font) = editor.font_family {
            settings.insert("editor.fontFamily".to_string(), json!(font));
        }
        if let Some(size) = editor.font_size {
            settings.insert("editor.fontSize".to_string(), json!(size));
        }
        if let Some(height) = editor.line_height {
            settings.insert("editor.lineHeight".to_string(), json!(height));
        }
        if let Some(ref theme) = editor.theme {
            settings.insert("workbench.colorTheme".to_string(), json!(theme));
        }
        if let Some(ref icon_theme) = editor.icon_theme {
            settings.insert("workbench.iconTheme".to_string(), json!(icon_theme));
        }

        // Word wrap
        settings.insert(
            "editor.wordWrap".to_string(),
            json!(match editor.word_wrap {
                crate::config::WordWrap::Off => "off",
                crate::config::WordWrap::On => "on",
                crate::config::WordWrap::WordWrapColumn => "wordWrapColumn",
                crate::config::WordWrap::Bounded => "bounded",
            }),
        );

        // Minimap
        settings.insert("editor.minimap.enabled".to_string(), json!(editor.minimap.enabled));
        settings.insert(
            "editor.minimap.side".to_string(),
            json!(match editor.minimap.side {
                crate::config::MinimapSide::Left => "left",
                crate::config::MinimapSide::Right => "right",
            }),
        );
        settings.insert("editor.minimap.maxColumn".to_string(), json!(editor.minimap.max_column));

        // Breadcrumbs
        settings.insert("breadcrumbs.enabled".to_string(), json!(editor.breadcrumbs_enabled));

        // File associations
        if !project.file_associations.is_empty() {
            settings.insert("files.associations".to_string(), json!(project.file_associations));
        }

        // Search exclusions
        if !project.search_exclude.is_empty() {
            let exclude: Map<String, Value> =
                project.search_exclude.iter().map(|p| (p.clone(), json!(true))).collect();
            settings.insert("search.exclude".to_string(), json!(exclude));
        }

        // Watcher exclusions
        if !project.watcher_exclude.is_empty() {
            let exclude: Map<String, Value> =
                project.watcher_exclude.iter().map(|p| (p.clone(), json!(true))).collect();
            settings.insert("files.watcherExclude".to_string(), json!(exclude));
        }

        // File nesting
        if project.file_nesting.enabled {
            settings.insert(
                "explorer.fileNesting.enabled".to_string(),
                json!(project.file_nesting.enabled),
            );
            settings.insert(
                "explorer.fileNesting.expand".to_string(),
                json!(project.file_nesting.expand),
            );
            if !project.file_nesting.patterns.is_empty() {
                let patterns: Map<String, Value> = project
                    .file_nesting
                    .patterns
                    .iter()
                    .map(|(k, v)| (k.clone(), json!(v.join(", "))))
                    .collect();
                settings.insert("explorer.fileNesting.patterns".to_string(), json!(patterns));
            }
        }

        // Rust-analyzer settings for dx projects
        if config.detected_features.is_cargo_project {
            settings.insert("rust-analyzer.cargo.features".to_string(), json!("all"));
            settings.insert("rust-analyzer.checkOnSave.command".to_string(), json!("clippy"));
            settings
                .insert("rust-analyzer.inlayHints.chainingHints.enable".to_string(), json!(true));
            settings
                .insert("rust-analyzer.inlayHints.parameterHints.enable".to_string(), json!(true));
        }

        // dx-specific settings
        settings.insert(
            "[rust]".to_string(),
            json!({
                "editor.defaultFormatter": "rust-lang.rust-analyzer",
                "editor.formatOnSave": true
            }),
        );

        // Add any extra settings
        for (key, value) in &editor.extra {
            settings.insert(key.clone(), value.clone());
        }

        json!(settings)
    }

    /// Generate tasks.json content.
    fn generate_tasks(&self, config: &WorkspaceConfig) -> Value {
        let tasks: Vec<Value> = config
            .tasks
            .tasks
            .iter()
            .map(|task| {
                let mut t = Map::new();
                t.insert("label".to_string(), json!(task.label));
                t.insert(
                    "type".to_string(),
                    json!(match &task.task_type {
                        crate::config::TaskType::Shell => "shell",
                        crate::config::TaskType::Process => "process",
                        crate::config::TaskType::Npm => "npm",
                        crate::config::TaskType::Custom(s) => s.as_str(),
                    }),
                );
                t.insert("command".to_string(), json!(task.command));

                if !task.args.is_empty() {
                    t.insert("args".to_string(), json!(task.args));
                }

                if let Some(ref group) = task.group {
                    t.insert(
                        "group".to_string(),
                        match group {
                            TaskGroup::Build { is_default } => json!({
                                "kind": "build",
                                "isDefault": is_default
                            }),
                            TaskGroup::Test { is_default } => json!({
                                "kind": "test",
                                "isDefault": is_default
                            }),
                            TaskGroup::None => json!("none"),
                        },
                    );
                }

                if let Some(ref cwd) = task.cwd {
                    t.insert("options".to_string(), json!({ "cwd": cwd }));
                }

                if task.is_background {
                    t.insert("isBackground".to_string(), json!(true));
                }

                // Presentation
                let presentation = json!({
                    "reveal": match task.presentation.reveal {
                        TaskReveal::Always => "always",
                        TaskReveal::Silent => "silent",
                        TaskReveal::Never => "never",
                    },
                    "echo": task.presentation.echo,
                    "focus": task.presentation.focus,
                    "panel": match task.presentation.panel {
                        TaskPanel::Shared => "shared",
                        TaskPanel::Dedicated => "dedicated",
                        TaskPanel::New => "new",
                    },
                    "showReuseMessage": task.presentation.show_rerun_button,
                    "clear": task.presentation.clear
                });
                t.insert("presentation".to_string(), presentation);

                if !task.problem_matcher.is_empty() {
                    t.insert("problemMatcher".to_string(), json!(task.problem_matcher));
                }

                if !task.depends_on.is_empty() {
                    t.insert("dependsOn".to_string(), json!(task.depends_on));
                }

                json!(t)
            })
            .collect();

        let mut inputs: Vec<Value> = Vec::new();
        for input in &config.tasks.inputs {
            inputs.push(json!({
                "id": input.id,
                "type": match &input.input_type {
                    crate::config::TaskInputType::PromptString => "promptString",
                    crate::config::TaskInputType::PickString => "pickString",
                    crate::config::TaskInputType::Command => "command",
                },
                "description": input.description,
                "default": input.default
            }));
        }

        let mut result = Map::new();
        result.insert("version".to_string(), json!("2.0.0"));
        result.insert("tasks".to_string(), json!(tasks));
        if !inputs.is_empty() {
            result.insert("inputs".to_string(), json!(inputs));
        }

        json!(result)
    }

    /// Generate launch.json content.
    fn generate_launch(&self, config: &WorkspaceConfig) -> Value {
        let configurations: Vec<Value> = config
            .debug
            .launch_configs
            .iter()
            .map(|lc| {
                let mut c = Map::new();
                c.insert("name".to_string(), json!(lc.name));
                c.insert(
                    "request".to_string(),
                    json!(match lc.request {
                        crate::config::LaunchRequest::Launch => "launch",
                        crate::config::LaunchRequest::Attach => "attach",
                    }),
                );
                c.insert(
                    "type".to_string(),
                    json!(match &lc.debug_type {
                        crate::config::DebugType::DxCli => "node",
                        crate::config::DebugType::Wasm => "pwa-chrome",
                        crate::config::DebugType::Lldb => "lldb",
                        crate::config::DebugType::CodeLldb => "lldb",
                        crate::config::DebugType::Gdb => "cppdbg",
                        crate::config::DebugType::Node => "node",
                        crate::config::DebugType::Chrome => "pwa-chrome",
                        crate::config::DebugType::Custom(s) => s.as_str(),
                    }),
                );

                if let Some(ref program) = lc.program {
                    c.insert("program".to_string(), json!(program));
                }

                if !lc.args.is_empty() {
                    c.insert("args".to_string(), json!(lc.args));
                }

                if let Some(ref cwd) = lc.cwd {
                    c.insert("cwd".to_string(), json!(cwd));
                }

                if !lc.env.is_empty() {
                    c.insert("env".to_string(), json!(lc.env));
                }

                if let Some(port) = lc.port {
                    c.insert("port".to_string(), json!(port));
                }

                if let Some(ref task) = lc.pre_launch_task {
                    c.insert("preLaunchTask".to_string(), json!(task));
                }

                if let Some(ref task) = lc.post_debug_task {
                    c.insert("postDebugTask".to_string(), json!(task));
                }

                if lc.stop_on_entry {
                    c.insert("stopOnEntry".to_string(), json!(true));
                }

                c.insert(
                    "console".to_string(),
                    json!(match lc.console {
                        crate::config::ConsoleType::InternalConsole => "internalConsole",
                        crate::config::ConsoleType::IntegratedTerminal => "integratedTerminal",
                        crate::config::ConsoleType::ExternalTerminal => "externalTerminal",
                    }),
                );

                // Add extra options
                for (key, value) in &lc.extra {
                    c.insert(key.clone(), value.clone());
                }

                json!(c)
            })
            .collect();

        let compounds: Vec<Value> = config
            .debug
            .compounds
            .iter()
            .map(|compound| {
                json!({
                    "name": compound.name,
                    "configurations": compound.configurations,
                    "stopAll": compound.stop_all
                })
            })
            .collect();

        let mut result = Map::new();
        result.insert("version".to_string(), json!("0.2.0"));
        result.insert("configurations".to_string(), json!(configurations));
        if !compounds.is_empty() {
            result.insert("compounds".to_string(), json!(compounds));
        }

        json!(result)
    }

    /// Generate extensions.json content.
    fn generate_extensions(&self, config: &WorkspaceConfig) -> Value {
        let recommendations: Vec<String> = config
            .extensions
            .core
            .iter()
            .chain(config.extensions.recommended.iter())
            .map(|e| e.id.clone())
            .collect();

        json!({
            "recommendations": recommendations,
            "unwantedRecommendations": config.extensions.unwanted
        })
    }
}

impl DesktopGenerator for VsCodeGenerator {
    fn generate(&self, config: &WorkspaceConfig, output_dir: &Path) -> Result<Vec<GeneratedFile>> {
        let mut files = Vec::new();

        // Generate settings.json
        let settings = self.generate_settings(config);
        files.push(GeneratedFile::new(
            ".vscode/settings.json",
            serde_json::to_string_pretty(&settings).unwrap_or_default(),
        ));

        // Generate tasks.json
        let tasks = self.generate_tasks(config);
        files.push(GeneratedFile::new(
            ".vscode/tasks.json",
            serde_json::to_string_pretty(&tasks).unwrap_or_default(),
        ));

        // Generate launch.json
        let launch = self.generate_launch(config);
        files.push(GeneratedFile::new(
            ".vscode/launch.json",
            serde_json::to_string_pretty(&launch).unwrap_or_default(),
        ));

        // Generate extensions.json
        let extensions = self.generate_extensions(config);
        files.push(GeneratedFile::new(
            ".vscode/extensions.json",
            serde_json::to_string_pretty(&extensions).unwrap_or_default(),
        ));

        // Write files
        let vscode_dir = output_dir.join(".vscode");
        fs::create_dir_all(&vscode_dir).map_err(|e| crate::Error::io(&vscode_dir, e))?;

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
        project_dir.join(".vscode").exists()
    }

    fn clean(&self, project_dir: &Path) -> Result<()> {
        let vscode_dir = project_dir.join(".vscode");
        if vscode_dir.exists() {
            fs::remove_dir_all(&vscode_dir).map_err(|e| crate::Error::io(&vscode_dir, e))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DebugConfig, TaskConfig};

    #[test]
    fn test_generate_settings() {
        let mut config = WorkspaceConfig::new("test");
        config.editor.tab_size = 2;
        config.detected_features.is_cargo_project = true;

        let generator = VsCodeGenerator::new();
        let settings = generator.generate_settings(&config);

        assert_eq!(settings["editor.tabSize"], 2);
        assert!(settings["rust-analyzer.checkOnSave.command"].as_str().is_some());
    }

    #[test]
    fn test_generate_tasks() {
        let mut config = WorkspaceConfig::new("test");
        config.tasks = TaskConfig::dx_defaults();

        let generator = VsCodeGenerator::new();
        let tasks = generator.generate_tasks(&config);

        assert_eq!(tasks["version"], "2.0.0");
        assert!(tasks["tasks"].is_array());
    }

    #[test]
    fn test_generate_launch() {
        let mut config = WorkspaceConfig::new("test");
        config.debug = DebugConfig::dx_defaults();

        let generator = VsCodeGenerator::new();
        let launch = generator.generate_launch(&config);

        assert_eq!(launch["version"], "0.2.0");
        assert!(launch["configurations"].is_array());
    }
}
