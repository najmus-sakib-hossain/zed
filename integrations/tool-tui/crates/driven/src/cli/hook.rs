//! Hook CLI Commands
//!
//! CLI commands for managing agent hooks.

use crate::hooks::{
    AgentHook, BuildEvent, GitOp, HookAction, HookCondition, HookEngine, HookTrigger, TestFilter,
    TestStatus,
};
use crate::{DrivenError, Result};
use std::path::{Path, PathBuf};

/// Hook command handler
pub struct HookCommand;

impl HookCommand {
    /// List all hooks
    pub fn list(project_root: &Path) -> Result<Vec<HookInfo>> {
        let hooks_dir = project_root.join(".driven/hooks");
        let mut engine = HookEngine::with_hooks_dir(&hooks_dir);

        if hooks_dir.exists() {
            engine.load_hooks(&hooks_dir)?;
        }

        let hooks: Vec<HookInfo> = engine.list_hooks().iter().map(HookInfo::from).collect();

        Ok(hooks)
    }

    /// Add a new hook
    pub fn add(
        project_root: &Path,
        id: &str,
        name: Option<&str>,
        trigger_type: &str,
        trigger_value: &str,
        agent: &str,
        message: &str,
        workflow: Option<&str>,
        condition: Option<&str>,
        enabled: bool,
    ) -> Result<()> {
        let hooks_dir = project_root.join(".driven/hooks");
        std::fs::create_dir_all(&hooks_dir).map_err(DrivenError::Io)?;

        let mut engine = HookEngine::with_hooks_dir(&hooks_dir);

        // Load existing hooks
        if hooks_dir.exists() {
            engine.load_hooks(&hooks_dir)?;
        }

        // Check if hook already exists
        if engine.get_hook(id).is_some() {
            return Err(DrivenError::Config(format!("Hook with ID '{}' already exists", id)));
        }

        // Parse trigger
        let trigger = Self::parse_trigger(trigger_type, trigger_value)?;

        // Create action
        let mut action = HookAction::new(agent, message);
        if let Some(wf) = workflow {
            action = action.with_workflow(wf);
        }

        // Create hook
        let mut hook = AgentHook::new(id)
            .with_name(name.unwrap_or(id))
            .with_trigger(trigger)
            .with_action(action)
            .with_enabled(enabled);

        // Add condition if provided
        if let Some(cond) = condition {
            hook = hook.with_condition(HookCondition::new(cond));
        }

        // Save hook to file
        let hook_path = hooks_dir.join(format!("{}.json", id));
        engine.save_hook(&hook, &hook_path)?;

        // Register hook
        engine.register_hook(hook)?;

        super::print_success(&format!("Hook '{}' added successfully", id));
        Ok(())
    }

    /// Remove a hook
    pub fn remove(project_root: &Path, id: &str) -> Result<()> {
        let hooks_dir = project_root.join(".driven/hooks");

        if !hooks_dir.exists() {
            return Err(DrivenError::Config("No hooks directory found".to_string()));
        }

        // Try to find and remove the hook file
        let json_path = hooks_dir.join(format!("{}.json", id));
        let yaml_path = hooks_dir.join(format!("{}.yaml", id));
        let yml_path = hooks_dir.join(format!("{}.yml", id));

        let removed = if json_path.exists() {
            std::fs::remove_file(&json_path).map_err(DrivenError::Io)?;
            true
        } else if yaml_path.exists() {
            std::fs::remove_file(&yaml_path).map_err(DrivenError::Io)?;
            true
        } else if yml_path.exists() {
            std::fs::remove_file(&yml_path).map_err(DrivenError::Io)?;
            true
        } else {
            false
        };

        if removed {
            super::print_success(&format!("Hook '{}' removed successfully", id));
            Ok(())
        } else {
            Err(DrivenError::Config(format!("Hook with ID '{}' not found", id)))
        }
    }

    /// Manually trigger a hook
    pub fn trigger(project_root: &Path, command: &str) -> Result<Vec<TriggerResult>> {
        let hooks_dir = project_root.join(".driven/hooks");
        let mut engine = HookEngine::with_hooks_dir(&hooks_dir);

        if hooks_dir.exists() {
            engine.load_hooks(&hooks_dir)?;
        }

        let results = engine.trigger_manual(command);

        if results.is_empty() {
            super::print_warning(&format!("No hooks found for command '{}'", command));
        } else {
            for result in &results {
                if result.success {
                    super::print_success(&format!(
                        "Hook '{}' executed successfully ({}ms)",
                        result.hook_id, result.duration_ms
                    ));
                } else {
                    super::print_error(&format!(
                        "Hook '{}' failed: {}",
                        result.hook_id,
                        result.error.as_deref().unwrap_or("Unknown error")
                    ));
                }
            }
        }

        Ok(results.into_iter().map(TriggerResult::from).collect())
    }

    /// Enable a hook
    pub fn enable(project_root: &Path, id: &str) -> Result<()> {
        Self::set_enabled(project_root, id, true)
    }

    /// Disable a hook
    pub fn disable(project_root: &Path, id: &str) -> Result<()> {
        Self::set_enabled(project_root, id, false)
    }

    /// Set hook enabled state
    fn set_enabled(project_root: &Path, id: &str, enabled: bool) -> Result<()> {
        let hooks_dir = project_root.join(".driven/hooks");

        if !hooks_dir.exists() {
            return Err(DrivenError::Config("No hooks directory found".to_string()));
        }

        // Find the hook file
        let (hook_path, mut hook) = Self::find_and_load_hook(&hooks_dir, id)?;

        // Update enabled state
        hook.enabled = enabled;

        // Save back
        let engine = HookEngine::new();
        engine.save_hook(&hook, &hook_path)?;

        let action = if enabled { "enabled" } else { "disabled" };
        super::print_success(&format!("Hook '{}' {}", id, action));
        Ok(())
    }

    /// Show hook details
    pub fn show(project_root: &Path, id: &str) -> Result<HookInfo> {
        let hooks_dir = project_root.join(".driven/hooks");

        if !hooks_dir.exists() {
            return Err(DrivenError::Config("No hooks directory found".to_string()));
        }

        let (_, hook) = Self::find_and_load_hook(&hooks_dir, id)?;
        Ok(HookInfo::from(&hook))
    }

    /// Find and load a hook from the hooks directory
    fn find_and_load_hook(hooks_dir: &Path, id: &str) -> Result<(PathBuf, AgentHook)> {
        let json_path = hooks_dir.join(format!("{}.json", id));
        let yaml_path = hooks_dir.join(format!("{}.yaml", id));
        let yml_path = hooks_dir.join(format!("{}.yml", id));

        let hook_path = if json_path.exists() {
            json_path
        } else if yaml_path.exists() {
            yaml_path
        } else if yml_path.exists() {
            yml_path
        } else {
            return Err(DrivenError::Config(format!("Hook with ID '{}' not found", id)));
        };

        let content = std::fs::read_to_string(&hook_path).map_err(DrivenError::Io)?;
        let ext = hook_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let hook: AgentHook = match ext {
            "json" => {
                serde_json::from_str(&content).map_err(|e| DrivenError::Parse(e.to_string()))?
            }
            "yaml" | "yml" => {
                serde_yaml::from_str(&content).map_err(|e| DrivenError::Parse(e.to_string()))?
            }
            _ => return Err(DrivenError::Parse(format!("Unknown file extension: {}", ext))),
        };

        Ok((hook_path, hook))
    }

    /// Parse trigger type and value into HookTrigger
    fn parse_trigger(trigger_type: &str, trigger_value: &str) -> Result<HookTrigger> {
        match trigger_type.to_lowercase().as_str() {
            "file" | "file_change" | "filechange" => {
                let patterns: Vec<String> =
                    trigger_value.split(',').map(|s| s.trim().to_string()).collect();
                Ok(HookTrigger::FileChange { patterns })
            }
            "git" | "git_operation" | "gitoperation" => {
                let operations: Vec<GitOp> = trigger_value
                    .split(',')
                    .map(|s| Self::parse_git_op(s.trim()))
                    .collect::<Result<Vec<_>>>()?;
                Ok(HookTrigger::GitOperation { operations })
            }
            "build" | "build_event" | "buildevent" => {
                let events: Vec<BuildEvent> = trigger_value
                    .split(',')
                    .map(|s| Self::parse_build_event(s.trim()))
                    .collect::<Result<Vec<_>>>()?;
                Ok(HookTrigger::BuildEvent { events })
            }
            "test" | "test_result" | "testresult" => {
                let filter = Self::parse_test_filter(trigger_value)?;
                Ok(HookTrigger::TestResult { filter })
            }
            "manual" => Ok(HookTrigger::Manual {
                command: trigger_value.to_string(),
            }),
            "scheduled" | "cron" => Ok(HookTrigger::Scheduled {
                cron: trigger_value.to_string(),
            }),
            _ => Err(DrivenError::Config(format!(
                "Unknown trigger type: {}. Valid types: file, git, build, test, manual, scheduled",
                trigger_type
            ))),
        }
    }

    /// Parse git operation string
    fn parse_git_op(s: &str) -> Result<GitOp> {
        match s.to_lowercase().as_str() {
            "commit" => Ok(GitOp::Commit),
            "push" => Ok(GitOp::Push),
            "pull" => Ok(GitOp::Pull),
            "merge" => Ok(GitOp::Merge),
            "checkout" => Ok(GitOp::Checkout),
            "stash" => Ok(GitOp::Stash),
            _ => Err(DrivenError::Config(format!(
                "Unknown git operation: {}. Valid: commit, push, pull, merge, checkout, stash",
                s
            ))),
        }
    }

    /// Parse build event string
    fn parse_build_event(s: &str) -> Result<BuildEvent> {
        match s.to_lowercase().as_str() {
            "start" => Ok(BuildEvent::Start),
            "success" => Ok(BuildEvent::Success),
            "failure" | "fail" => Ok(BuildEvent::Failure),
            "warning" | "warn" => Ok(BuildEvent::Warning),
            _ => Err(DrivenError::Config(format!(
                "Unknown build event: {}. Valid: start, success, failure, warning",
                s
            ))),
        }
    }

    /// Parse test filter string (format: "status:failed,name:test_*")
    fn parse_test_filter(s: &str) -> Result<TestFilter> {
        let mut filter = TestFilter::new();

        for part in s.split(',') {
            let part = part.trim();
            if let Some((key, value)) = part.split_once(':') {
                match key.trim().to_lowercase().as_str() {
                    "status" => {
                        filter.status = Some(Self::parse_test_status(value.trim())?);
                    }
                    "name" => {
                        filter.name_pattern = Some(value.trim().to_string());
                    }
                    "file" => {
                        filter.file_pattern = Some(value.trim().to_string());
                    }
                    _ => {
                        return Err(DrivenError::Config(format!(
                            "Unknown test filter key: {}. Valid: status, name, file",
                            key
                        )));
                    }
                }
            }
        }

        Ok(filter)
    }

    /// Parse test status string
    fn parse_test_status(s: &str) -> Result<TestStatus> {
        match s.to_lowercase().as_str() {
            "passed" | "pass" => Ok(TestStatus::Passed),
            "failed" | "fail" => Ok(TestStatus::Failed),
            "skipped" | "skip" => Ok(TestStatus::Skipped),
            "timeout" => Ok(TestStatus::Timeout),
            _ => Err(DrivenError::Config(format!(
                "Unknown test status: {}. Valid: passed, failed, skipped, timeout",
                s
            ))),
        }
    }
}

/// Hook information for display
#[derive(Debug, Clone)]
pub struct HookInfo {
    pub id: String,
    pub name: String,
    pub trigger_type: String,
    pub trigger_value: String,
    pub agent: String,
    pub workflow: Option<String>,
    pub enabled: bool,
    pub priority: u8,
    pub has_condition: bool,
    pub chain_count: usize,
}

impl From<&AgentHook> for HookInfo {
    fn from(hook: &AgentHook) -> Self {
        let (trigger_type, trigger_value) = match &hook.trigger {
            HookTrigger::FileChange { patterns } => {
                ("file_change".to_string(), patterns.join(", "))
            }
            HookTrigger::GitOperation { operations } => {
                let ops: Vec<&str> = operations
                    .iter()
                    .map(|o| match o {
                        GitOp::Commit => "commit",
                        GitOp::Push => "push",
                        GitOp::Pull => "pull",
                        GitOp::Merge => "merge",
                        GitOp::Checkout => "checkout",
                        GitOp::Stash => "stash",
                    })
                    .collect();
                ("git_operation".to_string(), ops.join(", "))
            }
            HookTrigger::BuildEvent { events } => {
                let evts: Vec<&str> = events
                    .iter()
                    .map(|e| match e {
                        BuildEvent::Start => "start",
                        BuildEvent::Success => "success",
                        BuildEvent::Failure => "failure",
                        BuildEvent::Warning => "warning",
                    })
                    .collect();
                ("build_event".to_string(), evts.join(", "))
            }
            HookTrigger::TestResult { filter } => {
                let mut parts = Vec::new();
                if let Some(status) = &filter.status {
                    parts.push(format!("status:{:?}", status).to_lowercase());
                }
                if let Some(name) = &filter.name_pattern {
                    parts.push(format!("name:{}", name));
                }
                if let Some(file) = &filter.file_pattern {
                    parts.push(format!("file:{}", file));
                }
                ("test_result".to_string(), parts.join(", "))
            }
            HookTrigger::Manual { command } => ("manual".to_string(), command.clone()),
            HookTrigger::Scheduled { cron } => ("scheduled".to_string(), cron.clone()),
        };

        Self {
            id: hook.id.clone(),
            name: hook.name.clone(),
            trigger_type,
            trigger_value,
            agent: hook.action.agent.clone(),
            workflow: hook.action.workflow.clone(),
            enabled: hook.enabled,
            priority: hook.priority,
            has_condition: hook.condition.is_some(),
            chain_count: hook.chain.as_ref().map_or(0, |c| c.len()),
        }
    }
}

/// Trigger result for display
#[derive(Debug, Clone)]
pub struct TriggerResult {
    pub hook_id: String,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub chained: Vec<String>,
}

impl From<crate::hooks::HookExecutionResult> for TriggerResult {
    fn from(result: crate::hooks::HookExecutionResult) -> Self {
        Self {
            hook_id: result.hook_id,
            success: result.success,
            output: result.output,
            error: result.error,
            duration_ms: result.duration_ms,
            chained: result.chained,
        }
    }
}

/// Print hooks in a formatted table
pub fn print_hooks_table(hooks: &[HookInfo]) {
    use console::style;

    if hooks.is_empty() {
        println!("No hooks configured.");
        return;
    }

    // Print header
    println!(
        "{:<20} {:<15} {:<30} {:<10} {:<8}",
        style("ID").bold(),
        style("Trigger").bold(),
        style("Agent/Workflow").bold(),
        style("Enabled").bold(),
        style("Priority").bold(),
    );
    println!("{}", "-".repeat(85));

    // Print hooks
    for hook in hooks {
        let enabled = if hook.enabled {
            style("✓").green().to_string()
        } else {
            style("✗").red().to_string()
        };

        let agent_workflow = if let Some(wf) = &hook.workflow {
            format!("{}/{}", hook.agent, wf)
        } else {
            hook.agent.clone()
        };

        println!(
            "{:<20} {:<15} {:<30} {:<10} {:<8}",
            hook.id, hook.trigger_type, agent_workflow, enabled, hook.priority,
        );
    }
}

/// Print detailed hook information
pub fn print_hook_details(hook: &HookInfo) {
    use console::style;

    println!("{}", style("Hook Details").bold().underlined());
    println!();
    println!("  {}: {}", style("ID").bold(), hook.id);
    println!("  {}: {}", style("Name").bold(), hook.name);
    println!("  {}: {}", style("Trigger Type").bold(), hook.trigger_type);
    println!("  {}: {}", style("Trigger Value").bold(), hook.trigger_value);
    println!("  {}: {}", style("Agent").bold(), hook.agent);
    if let Some(wf) = &hook.workflow {
        println!("  {}: {}", style("Workflow").bold(), wf);
    }
    println!("  {}: {}", style("Enabled").bold(), if hook.enabled { "Yes" } else { "No" });
    println!("  {}: {}", style("Priority").bold(), hook.priority);
    println!(
        "  {}: {}",
        style("Has Condition").bold(),
        if hook.has_condition { "Yes" } else { "No" }
    );
    if hook.chain_count > 0 {
        println!("  {}: {} hooks", style("Chain").bold(), hook.chain_count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_trigger_file_change() {
        let trigger = HookCommand::parse_trigger("file", "**/*.rs, **/*.py").unwrap();
        match trigger {
            HookTrigger::FileChange { patterns } => {
                assert_eq!(patterns.len(), 2);
                assert!(patterns.contains(&"**/*.rs".to_string()));
                assert!(patterns.contains(&"**/*.py".to_string()));
            }
            _ => panic!("Wrong trigger type"),
        }
    }

    #[test]
    fn test_parse_trigger_git() {
        let trigger = HookCommand::parse_trigger("git", "commit, push").unwrap();
        match trigger {
            HookTrigger::GitOperation { operations } => {
                assert_eq!(operations.len(), 2);
                assert!(operations.contains(&GitOp::Commit));
                assert!(operations.contains(&GitOp::Push));
            }
            _ => panic!("Wrong trigger type"),
        }
    }

    #[test]
    fn test_parse_trigger_build() {
        let trigger = HookCommand::parse_trigger("build", "failure, warning").unwrap();
        match trigger {
            HookTrigger::BuildEvent { events } => {
                assert_eq!(events.len(), 2);
                assert!(events.contains(&BuildEvent::Failure));
                assert!(events.contains(&BuildEvent::Warning));
            }
            _ => panic!("Wrong trigger type"),
        }
    }

    #[test]
    fn test_parse_trigger_test() {
        let trigger = HookCommand::parse_trigger("test", "status:failed, name:test_*").unwrap();
        match trigger {
            HookTrigger::TestResult { filter } => {
                assert_eq!(filter.status, Some(TestStatus::Failed));
                assert_eq!(filter.name_pattern, Some("test_*".to_string()));
            }
            _ => panic!("Wrong trigger type"),
        }
    }

    #[test]
    fn test_parse_trigger_manual() {
        let trigger = HookCommand::parse_trigger("manual", "lint").unwrap();
        match trigger {
            HookTrigger::Manual { command } => {
                assert_eq!(command, "lint");
            }
            _ => panic!("Wrong trigger type"),
        }
    }

    #[test]
    fn test_parse_trigger_scheduled() {
        let trigger = HookCommand::parse_trigger("scheduled", "0 * * * *").unwrap();
        match trigger {
            HookTrigger::Scheduled { cron } => {
                assert_eq!(cron, "0 * * * *");
            }
            _ => panic!("Wrong trigger type"),
        }
    }

    #[test]
    fn test_add_and_list_hooks() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Add a hook
        HookCommand::add(
            project_root,
            "test-hook",
            Some("Test Hook"),
            "manual",
            "test",
            "reviewer",
            "Review the code",
            Some("quick-review"),
            None,
            true,
        )
        .unwrap();

        // List hooks
        let hooks = HookCommand::list(project_root).unwrap();
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].id, "test-hook");
        assert_eq!(hooks[0].name, "Test Hook");
        assert_eq!(hooks[0].agent, "reviewer");
        assert_eq!(hooks[0].workflow, Some("quick-review".to_string()));
    }

    #[test]
    fn test_remove_hook() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Add a hook
        HookCommand::add(
            project_root,
            "test-hook",
            None,
            "manual",
            "test",
            "reviewer",
            "Review",
            None,
            None,
            true,
        )
        .unwrap();

        // Remove the hook
        HookCommand::remove(project_root, "test-hook").unwrap();

        // List should be empty
        let hooks = HookCommand::list(project_root).unwrap();
        assert!(hooks.is_empty());
    }

    #[test]
    fn test_enable_disable_hook() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Add a hook
        HookCommand::add(
            project_root,
            "test-hook",
            None,
            "manual",
            "test",
            "reviewer",
            "Review",
            None,
            None,
            true,
        )
        .unwrap();

        // Disable the hook
        HookCommand::disable(project_root, "test-hook").unwrap();
        let hook = HookCommand::show(project_root, "test-hook").unwrap();
        assert!(!hook.enabled);

        // Enable the hook
        HookCommand::enable(project_root, "test-hook").unwrap();
        let hook = HookCommand::show(project_root, "test-hook").unwrap();
        assert!(hook.enabled);
    }

    #[test]
    fn test_trigger_manual_hook() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Add a manual hook
        HookCommand::add(
            project_root,
            "lint-hook",
            None,
            "manual",
            "lint",
            "reviewer",
            "Lint the code",
            None,
            None,
            true,
        )
        .unwrap();

        // Trigger the hook
        let results = HookCommand::trigger(project_root, "lint").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        assert_eq!(results[0].hook_id, "lint-hook");
    }
}
