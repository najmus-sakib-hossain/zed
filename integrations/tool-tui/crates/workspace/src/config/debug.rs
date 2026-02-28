//! Debug and launch configuration types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Debug and launch configurations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DebugConfig {
    /// Launch configurations for debugging.
    #[serde(default)]
    pub launch_configs: Vec<LaunchConfig>,

    /// Compound configurations that launch multiple configs.
    #[serde(default)]
    pub compounds: Vec<CompoundConfig>,

    /// Global debug settings.
    #[serde(default)]
    pub settings: DebugSettings,
}

impl DebugConfig {
    /// Validate debug configuration.
    pub fn validate(&self) -> crate::Result<()> {
        for config in &self.launch_configs {
            if config.name.is_empty() {
                return Err(crate::Error::validation("Launch configuration name cannot be empty"));
            }
        }

        for compound in &self.compounds {
            if compound.name.is_empty() {
                return Err(crate::Error::validation(
                    "Compound configuration name cannot be empty",
                ));
            }
            if compound.configurations.is_empty() {
                return Err(crate::Error::validation(
                    "Compound configuration must have at least one configuration",
                ));
            }
        }

        Ok(())
    }

    /// Create default dx launch configurations.
    pub fn dx_defaults() -> Self {
        Self {
            launch_configs: vec![
                LaunchConfig {
                    name: "dx dev".to_string(),
                    request: LaunchRequest::Launch,
                    debug_type: DebugType::DxCli,
                    program: Some("dx".to_string()),
                    args: vec!["dev".to_string()],
                    cwd: Some("${workspaceFolder}".to_string()),
                    ..Default::default()
                },
                LaunchConfig {
                    name: "dx build".to_string(),
                    request: LaunchRequest::Launch,
                    debug_type: DebugType::DxCli,
                    program: Some("dx".to_string()),
                    args: vec!["build".to_string()],
                    cwd: Some("${workspaceFolder}".to_string()),
                    ..Default::default()
                },
                LaunchConfig {
                    name: "Debug WASM".to_string(),
                    request: LaunchRequest::Launch,
                    debug_type: DebugType::Wasm,
                    program: None,
                    args: vec![],
                    cwd: Some("${workspaceFolder}".to_string()),
                    port: Some(9222),
                    ..Default::default()
                },
                LaunchConfig {
                    name: "Attach to dx-server".to_string(),
                    request: LaunchRequest::Attach,
                    debug_type: DebugType::Lldb,
                    program: None,
                    args: vec![],
                    cwd: Some("${workspaceFolder}".to_string()),
                    port: Some(1234),
                    ..Default::default()
                },
            ],
            compounds: vec![CompoundConfig {
                name: "Full Stack Debug".to_string(),
                configurations: vec!["dx dev".to_string(), "Debug WASM".to_string()],
                stop_all: true,
                presentation: None,
            }],
            settings: DebugSettings::default(),
        }
    }
}

/// A single launch/debug configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LaunchConfig {
    /// Display name for this configuration.
    pub name: String,

    /// Launch or attach request type.
    #[serde(default)]
    pub request: LaunchRequest,

    /// Debug adapter type.
    #[serde(rename = "type")]
    pub debug_type: DebugType,

    /// Program to launch.
    #[serde(default)]
    pub program: Option<String>,

    /// Command-line arguments.
    #[serde(default)]
    pub args: Vec<String>,

    /// Working directory.
    #[serde(default)]
    pub cwd: Option<String>,

    /// Environment variables.
    #[serde(default)]
    pub env: HashMap<String, String>,

    /// Port for attach/remote debugging.
    #[serde(default)]
    pub port: Option<u16>,

    /// Pre-launch task to run.
    #[serde(default)]
    pub pre_launch_task: Option<String>,

    /// Post-debug task to run.
    #[serde(default)]
    pub post_debug_task: Option<String>,

    /// Stop on entry point.
    #[serde(default)]
    pub stop_on_entry: bool,

    /// Console type for output.
    #[serde(default)]
    pub console: ConsoleType,

    /// Presentation settings.
    #[serde(default)]
    pub presentation: Option<PresentationConfig>,

    /// Additional debug adapter specific options.
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Launch request type.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LaunchRequest {
    /// Launch the program.
    #[default]
    Launch,
    /// Attach to running program.
    Attach,
}

/// Debug adapter types supported.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DebugType {
    /// dx-cli command execution.
    #[default]
    #[serde(rename = "dx-cli")]
    DxCli,
    /// WebAssembly debugging.
    Wasm,
    /// Rust debugging via LLDB.
    Lldb,
    /// Rust debugging via CodeLLDB.
    #[serde(rename = "codelldb")]
    CodeLldb,
    /// Rust debugging via GDB.
    Gdb,
    /// Node.js debugging.
    Node,
    /// Chrome/Chromium debugging.
    Chrome,
    /// Custom debug adapter.
    Custom(String),
}

/// Console type for debug output.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ConsoleType {
    /// Internal debug console.
    #[default]
    InternalConsole,
    /// Integrated terminal.
    IntegratedTerminal,
    /// External terminal.
    ExternalTerminal,
}

/// Compound configuration that launches multiple debug configs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompoundConfig {
    /// Display name.
    pub name: String,

    /// List of configuration names to launch.
    pub configurations: Vec<String>,

    /// Stop all when one stops.
    #[serde(default)]
    pub stop_all: bool,

    /// Presentation settings.
    #[serde(default)]
    pub presentation: Option<PresentationConfig>,
}

/// Presentation settings for configurations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresentationConfig {
    /// Show in picker.
    #[serde(default = "default_true")]
    pub hidden: bool,

    /// Group name for organization.
    #[serde(default)]
    pub group: Option<String>,

    /// Order within group.
    #[serde(default)]
    pub order: Option<i32>,
}

fn default_true() -> bool {
    true
}

/// Global debug settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DebugSettings {
    /// Allow breakpoints in any file.
    #[serde(default)]
    pub allow_breakpoints_everywhere: bool,

    /// Open debug viewlet on session start.
    #[serde(default = "default_true_fn")]
    pub open_debug_on_session_start: bool,

    /// Show inline values during debugging.
    #[serde(default)]
    pub inline_values: InlineValuesConfig,

    /// Terminal settings for debug.
    #[serde(default)]
    pub terminal: DebugTerminalConfig,
}

fn default_true_fn() -> bool {
    true
}

/// Inline values display configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum InlineValuesConfig {
    /// Never show inline values.
    Off,
    /// Show for supported languages.
    #[default]
    On,
    /// Auto-detect based on debugger.
    Auto,
}

/// Debug terminal configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DebugTerminalConfig {
    /// Clear terminal before launch.
    #[serde(default)]
    pub clear_before_reusing: bool,

    /// Integrated terminal profile to use.
    #[serde(default)]
    pub profile: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_config_defaults() {
        let config = DebugConfig::default();
        assert!(config.launch_configs.is_empty());
    }

    #[test]
    fn test_dx_defaults() {
        let config = DebugConfig::dx_defaults();
        assert!(!config.launch_configs.is_empty());
        assert!(config.launch_configs.iter().any(|c| c.name == "dx dev"));
    }

    #[test]
    fn test_launch_config_serialization() {
        let config = LaunchConfig {
            name: "Test".to_string(),
            request: LaunchRequest::Launch,
            debug_type: DebugType::DxCli,
            program: Some("dx".to_string()),
            args: vec!["build".to_string()],
            ..Default::default()
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"name\":\"Test\""));
    }
}
