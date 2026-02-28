//! Task automation configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Task automation configurations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskConfig {
    /// Task definitions.
    #[serde(default)]
    pub tasks: Vec<TaskDefinition>,

    /// Input definitions for parameterized tasks.
    #[serde(default)]
    pub inputs: Vec<TaskInput>,

    /// Global task settings.
    #[serde(default)]
    pub settings: TaskSettings,
}

impl TaskConfig {
    /// Validate task configuration.
    pub fn validate(&self) -> crate::Result<()> {
        let mut seen_labels = std::collections::HashSet::new();

        for task in &self.tasks {
            if task.label.is_empty() {
                return Err(crate::Error::validation("Task label cannot be empty"));
            }

            if !seen_labels.insert(&task.label) {
                return Err(crate::Error::validation(format!(
                    "Duplicate task label: {}",
                    task.label
                )));
            }
        }

        Ok(())
    }

    /// Create default dx task configurations.
    pub fn dx_defaults() -> Self {
        Self {
            tasks: vec![
                TaskDefinition {
                    label: "dx build".to_string(),
                    task_type: TaskType::Shell,
                    command: "dx".to_string(),
                    args: vec!["build".to_string()],
                    group: Some(TaskGroup::Build { is_default: true }),
                    presentation: TaskPresentation::default(),
                    problem_matcher: vec!["$rustc".to_string()],
                    ..Default::default()
                },
                TaskDefinition {
                    label: "dx dev".to_string(),
                    task_type: TaskType::Shell,
                    command: "dx".to_string(),
                    args: vec!["dev".to_string()],
                    group: None,
                    is_background: true,
                    presentation: TaskPresentation {
                        reveal: TaskReveal::Always,
                        panel: TaskPanel::New,
                        ..Default::default()
                    },
                    problem_matcher: vec!["$rustc-watch".to_string()],
                    ..Default::default()
                },
                TaskDefinition {
                    label: "dx check".to_string(),
                    task_type: TaskType::Shell,
                    command: "dx".to_string(),
                    args: vec!["check".to_string()],
                    group: Some(TaskGroup::Test { is_default: false }),
                    presentation: TaskPresentation::default(),
                    problem_matcher: vec!["$rustc".to_string()],
                    ..Default::default()
                },
                TaskDefinition {
                    label: "dx forge".to_string(),
                    task_type: TaskType::Shell,
                    command: "dx".to_string(),
                    args: vec!["forge".to_string()],
                    group: Some(TaskGroup::Build { is_default: false }),
                    presentation: TaskPresentation::default(),
                    problem_matcher: vec!["$rustc".to_string()],
                    ..Default::default()
                },
                TaskDefinition {
                    label: "cargo fmt".to_string(),
                    task_type: TaskType::Shell,
                    command: "cargo".to_string(),
                    args: vec!["fmt".to_string(), "--all".to_string()],
                    group: None,
                    presentation: TaskPresentation::default(),
                    problem_matcher: vec![],
                    ..Default::default()
                },
                TaskDefinition {
                    label: "cargo clippy".to_string(),
                    task_type: TaskType::Shell,
                    command: "cargo".to_string(),
                    args: vec![
                        "clippy".to_string(),
                        "--all-targets".to_string(),
                        "--all-features".to_string(),
                    ],
                    group: Some(TaskGroup::Test { is_default: false }),
                    presentation: TaskPresentation::default(),
                    problem_matcher: vec!["$rustc".to_string()],
                    ..Default::default()
                },
            ],
            inputs: vec![],
            settings: TaskSettings::default(),
        }
    }
}

/// A single task definition.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskDefinition {
    /// Task label/name.
    pub label: String,

    /// Task type.
    #[serde(rename = "type")]
    pub task_type: TaskType,

    /// Command to execute.
    pub command: String,

    /// Command arguments.
    #[serde(default)]
    pub args: Vec<String>,

    /// Task group membership.
    #[serde(default)]
    pub group: Option<TaskGroup>,

    /// Working directory.
    #[serde(default)]
    pub cwd: Option<String>,

    /// Environment variables.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Is this a background task.
    #[serde(default)]
    pub is_background: bool,

    /// Presentation settings.
    #[serde(default)]
    pub presentation: TaskPresentation,

    /// Problem matchers for parsing output.
    #[serde(default)]
    pub problem_matcher: Vec<String>,

    /// Tasks this depends on.
    #[serde(default)]
    pub depends_on: Vec<String>,

    /// Run options.
    #[serde(default)]
    pub run_options: TaskRunOptions,

    /// Additional task-specific options.
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Task type.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskType {
    /// Shell command.
    #[default]
    Shell,
    /// Process execution.
    Process,
    /// npm script.
    Npm,
    /// Custom task type.
    Custom(String),
}

/// Task group configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskGroup {
    /// Build task group.
    Build {
        /// Is this the default build task.
        #[serde(default)]
        is_default: bool,
    },
    /// Test task group.
    Test {
        /// Is this the default test task.
        #[serde(default)]
        is_default: bool,
    },
    /// No group (clean, etc).
    None,
}

/// Task presentation settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskPresentation {
    /// When to reveal the terminal.
    #[serde(default)]
    pub reveal: TaskReveal,

    /// Echo the command.
    #[serde(default = "default_true")]
    pub echo: bool,

    /// Focus the terminal.
    #[serde(default)]
    pub focus: bool,

    /// Panel behavior.
    #[serde(default)]
    pub panel: TaskPanel,

    /// Show rerun button.
    #[serde(default = "default_true")]
    pub show_rerun_button: bool,

    /// Clear terminal before running.
    #[serde(default)]
    pub clear: bool,

    /// Close terminal on exit.
    #[serde(default)]
    pub close: bool,
}

fn default_true() -> bool {
    true
}

/// When to reveal the task terminal.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskReveal {
    /// Always reveal.
    #[default]
    Always,
    /// Reveal only on problem.
    Silent,
    /// Never reveal.
    Never,
}

/// Task panel behavior.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskPanel {
    /// Share panel with other tasks.
    #[default]
    Shared,
    /// Dedicated panel.
    Dedicated,
    /// New panel each run.
    New,
}

/// Task run options.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskRunOptions {
    /// Reevaluate on rerun.
    #[serde(default)]
    pub reevaluate_on_rerun: bool,

    /// Run on folder open.
    #[serde(default)]
    pub run_on: TaskRunOn,

    /// Instance limit.
    #[serde(default)]
    pub instance_limit: Option<u32>,
}

/// When to run the task.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TaskRunOn {
    /// Run manually.
    #[default]
    Default,
    /// Run on folder open.
    FolderOpen,
}

/// Task input definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInput {
    /// Input identifier.
    pub id: String,

    /// Input type.
    #[serde(rename = "type")]
    pub input_type: TaskInputType,

    /// Description shown to user.
    #[serde(default)]
    pub description: Option<String>,

    /// Default value.
    #[serde(default)]
    pub default: Option<String>,
}

/// Task input types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum TaskInputType {
    /// Prompt for string input.
    PromptString,
    /// Pick from list.
    PickString,
    /// Run a command for input.
    Command,
}

/// Global task settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskSettings {
    /// Auto-detect tasks from package.json, Cargo.toml, etc.
    #[serde(default = "default_true")]
    pub auto_detect: bool,

    /// Show task output in terminal.
    #[serde(default)]
    pub show_output: TaskShowOutput,
}

/// When to show task output.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskShowOutput {
    /// Always show output.
    #[default]
    Always,
    /// Show only on error.
    OnError,
    /// Never show.
    Never,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_config_defaults() {
        let config = TaskConfig::default();
        assert!(config.tasks.is_empty());
    }

    #[test]
    fn test_dx_defaults() {
        let config = TaskConfig::dx_defaults();
        assert!(!config.tasks.is_empty());
        assert!(config.tasks.iter().any(|t| t.label == "dx build"));
        assert!(config.tasks.iter().any(|t| t.label == "dx dev"));
    }

    #[test]
    fn test_validation() {
        let mut config = TaskConfig::dx_defaults();
        assert!(config.validate().is_ok());

        // Add duplicate label
        config.tasks.push(TaskDefinition {
            label: "dx build".to_string(),
            ..Default::default()
        });
        assert!(config.validate().is_err());
    }
}
