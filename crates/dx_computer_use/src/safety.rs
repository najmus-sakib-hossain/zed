//! Safety boundaries — prevents the AI agent from performing dangerous actions.

use serde::{Deserialize, Serialize};

/// Safety boundary configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    /// Maximum number of actions per session.
    pub max_actions_per_session: usize,
    /// Require user confirmation for destructive actions.
    pub require_confirmation: bool,
    /// Blocked application patterns.
    pub blocked_apps: Vec<String>,
    /// Blocked command patterns.
    pub blocked_commands: Vec<String>,
    /// Whether to allow file system writes.
    pub allow_fs_writes: bool,
    /// Whether to allow network access.
    pub allow_network: bool,
    /// Maximum cost budget per session (USD).
    pub max_cost_usd: f64,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            max_actions_per_session: 100,
            require_confirmation: true,
            blocked_apps: vec![
                "Terminal".into(),
                "cmd.exe".into(),
                "powershell".into(),
            ],
            blocked_commands: vec![
                "rm -rf".into(),
                "format".into(),
                "del /f".into(),
                "shutdown".into(),
                "reboot".into(),
            ],
            allow_fs_writes: false,
            allow_network: true,
            max_cost_usd: 10.0,
        }
    }
}

/// Result of a safety check.
#[derive(Debug, Clone)]
pub enum SafetyCheckResult {
    /// Action is allowed.
    Allowed,
    /// Action is blocked.
    Blocked { reason: String },
    /// Action requires user confirmation.
    NeedsConfirmation { reason: String },
}

/// Enforces safety boundaries for computer use.
pub struct SafetyBoundary {
    config: SafetyConfig,
    actions_this_session: usize,
    cost_this_session: f64,
}

impl SafetyBoundary {
    pub fn new(config: SafetyConfig) -> Self {
        Self {
            config,
            actions_this_session: 0,
            cost_this_session: 0.0,
        }
    }

    /// Check if an action is safe to execute.
    pub fn check_action(&self, action: &super::actions::ComputerAction) -> SafetyCheckResult {
        // Check action count
        if self.actions_this_session >= self.config.max_actions_per_session {
            return SafetyCheckResult::Blocked {
                reason: "Maximum actions per session reached".into(),
            };
        }

        // Check blocked commands
        if let super::actions::ComputerAction::RunCommand { command } = action {
            for blocked in &self.config.blocked_commands {
                if command.contains(blocked) {
                    return SafetyCheckResult::Blocked {
                        reason: format!("Command contains blocked pattern: {}", blocked),
                    };
                }
            }
            if self.config.require_confirmation {
                return SafetyCheckResult::NeedsConfirmation {
                    reason: "Shell commands require user confirmation".into(),
                };
            }
        }

        // Check blocked apps
        if let super::actions::ComputerAction::OpenApp { name } = action {
            for blocked in &self.config.blocked_apps {
                if name.contains(blocked) {
                    return SafetyCheckResult::Blocked {
                        reason: format!("Application is blocked: {}", blocked),
                    };
                }
            }
        }

        SafetyCheckResult::Allowed
    }

    /// Record that an action was executed.
    pub fn record_action(&mut self) {
        self.actions_this_session += 1;
    }

    /// Record a cost.
    pub fn record_cost(&mut self, cost_usd: f64) {
        self.cost_this_session += cost_usd;
    }

    /// Check if budget is exceeded.
    pub fn budget_exceeded(&self) -> bool {
        self.cost_this_session >= self.config.max_cost_usd
    }

    /// Reset session counters.
    pub fn reset_session(&mut self) {
        self.actions_this_session = 0;
        self.cost_this_session = 0.0;
    }
}
