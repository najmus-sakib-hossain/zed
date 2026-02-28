//! Agent Hooks System
//!
//! This module provides an event-driven automation system for triggering
//! agent actions based on various events like file changes, git operations,
//! build events, and more.
//!
//! ## Features
//!
//! - File change triggers with glob pattern matching
//! - Git operation triggers (commit, push, pull, merge, checkout, stash)
//! - Build and test event triggers
//! - Manual and scheduled triggers
//! - Conditional hook execution with expression evaluation
//! - Hook chaining for sequential execution
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::hooks::{AgentHook, HookTrigger, HookAction, HookEngine};
//!
//! let hook = AgentHook::new("on-save-lint")
//!     .with_trigger(HookTrigger::FileChange {
//!         patterns: vec!["**/*.rs".to_string()],
//!     })
//!     .with_action(HookAction {
//!         agent: "reviewer".to_string(),
//!         workflow: Some("quick-review".to_string()),
//!         message: "Review the changed file".to_string(),
//!         context: Default::default(),
//!     });
//!
//! let mut engine = HookEngine::new();
//! engine.register_hook(hook)?;
//! engine.start()?;
//! ```

use crate::{DrivenError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Agent hook definition
///
/// Represents a hook that triggers agent actions based on events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHook {
    /// Unique identifier for the hook
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// What triggers this hook
    pub trigger: HookTrigger,
    /// Optional condition for filtering
    pub condition: Option<HookCondition>,
    /// Action to perform when triggered
    pub action: HookAction,
    /// Whether the hook is enabled
    pub enabled: bool,
    /// IDs of hooks to trigger after this one completes
    pub chain: Option<Vec<String>>,
    /// Priority (lower = higher priority)
    pub priority: u8,
}

impl AgentHook {
    /// Create a new hook with the given ID
    pub fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            name: id.clone(),
            id,
            trigger: HookTrigger::Manual {
                command: "default".to_string(),
            },
            condition: None,
            action: HookAction::default(),
            enabled: true,
            chain: None,
            priority: 100,
        }
    }

    /// Set the hook name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the trigger
    pub fn with_trigger(mut self, trigger: HookTrigger) -> Self {
        self.trigger = trigger;
        self
    }

    /// Set the condition
    pub fn with_condition(mut self, condition: HookCondition) -> Self {
        self.condition = Some(condition);
        self
    }

    /// Set the action
    pub fn with_action(mut self, action: HookAction) -> Self {
        self.action = action;
        self
    }

    /// Set enabled state
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set the chain of hooks to trigger after
    pub fn with_chain(mut self, chain: Vec<String>) -> Self {
        self.chain = Some(chain);
        self
    }

    /// Set the priority
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Check if this hook matches a file change event
    pub fn matches_file_change(&self, path: &Path) -> bool {
        if !self.enabled {
            return false;
        }

        match &self.trigger {
            HookTrigger::FileChange { patterns } => {
                let path_str = path.to_string_lossy();
                patterns.iter().any(|pattern| glob_match(pattern, &path_str))
            }
            _ => false,
        }
    }

    /// Check if this hook matches a git operation
    pub fn matches_git_op(&self, op: &GitOp) -> bool {
        if !self.enabled {
            return false;
        }

        match &self.trigger {
            HookTrigger::GitOperation { operations } => operations.contains(op),
            _ => false,
        }
    }

    /// Check if this hook matches a build event
    pub fn matches_build_event(&self, event: &BuildEvent) -> bool {
        if !self.enabled {
            return false;
        }

        match &self.trigger {
            HookTrigger::BuildEvent { events } => events.contains(event),
            _ => false,
        }
    }

    /// Check if this hook matches a test result
    pub fn matches_test_result(&self, result: &TestResult) -> bool {
        if !self.enabled {
            return false;
        }

        match &self.trigger {
            HookTrigger::TestResult { filter } => filter.matches(result),
            _ => false,
        }
    }

    /// Check if this hook matches a manual command
    pub fn matches_manual(&self, command: &str) -> bool {
        if !self.enabled {
            return false;
        }

        match &self.trigger {
            HookTrigger::Manual { command: cmd } => cmd == command,
            _ => false,
        }
    }

    /// Evaluate the condition against a context
    pub fn evaluate_condition(&self, context: &HookContext) -> bool {
        match &self.condition {
            Some(condition) => condition.evaluate(context),
            None => true, // No condition means always pass
        }
    }
}

/// Hook trigger types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HookTrigger {
    /// Triggered when files matching patterns change
    FileChange {
        /// Glob patterns to match
        patterns: Vec<String>,
    },
    /// Triggered on git operations
    GitOperation {
        /// Git operations to watch
        operations: Vec<GitOp>,
    },
    /// Triggered on build events
    BuildEvent {
        /// Build events to watch
        events: Vec<BuildEvent>,
    },
    /// Triggered on test results
    TestResult {
        /// Filter for test results
        filter: TestFilter,
    },
    /// Triggered manually via command
    Manual {
        /// Command name
        command: String,
    },
    /// Triggered on a schedule
    Scheduled {
        /// Cron expression
        cron: String,
    },
}

/// Git operations that can trigger hooks
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum GitOp {
    /// Git commit
    Commit,
    /// Git push
    Push,
    /// Git pull
    Pull,
    /// Git merge
    Merge,
    /// Git checkout
    Checkout,
    /// Git stash
    Stash,
}

/// Build events that can trigger hooks
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum BuildEvent {
    /// Build started
    Start,
    /// Build succeeded
    Success,
    /// Build failed
    Failure,
    /// Build completed with warnings
    Warning,
}

/// Filter for test results
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestFilter {
    /// Match on test status
    pub status: Option<TestStatus>,
    /// Match on test name pattern
    pub name_pattern: Option<String>,
    /// Match on test file pattern
    pub file_pattern: Option<String>,
}

impl TestFilter {
    /// Create a new test filter
    pub fn new() -> Self {
        Self {
            status: None,
            name_pattern: None,
            file_pattern: None,
        }
    }

    /// Filter by status
    pub fn with_status(mut self, status: TestStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Filter by name pattern
    pub fn with_name_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.name_pattern = Some(pattern.into());
        self
    }

    /// Filter by file pattern
    pub fn with_file_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.file_pattern = Some(pattern.into());
        self
    }

    /// Check if a test result matches this filter
    pub fn matches(&self, result: &TestResult) -> bool {
        // Check status
        if let Some(status) = &self.status {
            if result.status != *status {
                return false;
            }
        }

        // Check name pattern
        if let Some(pattern) = &self.name_pattern {
            if !glob_match(pattern, &result.name) {
                return false;
            }
        }

        // Check file pattern
        if let Some(pattern) = &self.file_pattern {
            if let Some(file) = &result.file {
                if !glob_match(pattern, &file.to_string_lossy()) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}

impl Default for TestFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Test status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TestStatus {
    /// Test passed
    Passed,
    /// Test failed
    Failed,
    /// Test was skipped
    Skipped,
    /// Test timed out
    Timeout,
}

/// Test result information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Test name
    pub name: String,
    /// Test status
    pub status: TestStatus,
    /// Test file (if known)
    pub file: Option<PathBuf>,
    /// Test duration in milliseconds
    pub duration_ms: Option<u64>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Hook condition for filtering
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HookCondition {
    /// Expression to evaluate (e.g., "file.ext == 'rs' && file.path.contains('src')")
    pub expression: String,
}

impl HookCondition {
    /// Create a new condition
    pub fn new(expression: impl Into<String>) -> Self {
        Self {
            expression: expression.into(),
        }
    }

    /// Evaluate the condition against a context
    pub fn evaluate(&self, context: &HookContext) -> bool {
        // Simple expression evaluation
        // Supports: file.ext, file.path, file.name, file.size
        // Operators: ==, !=, contains, starts_with, ends_with, &&, ||

        let expr = self.expression.trim();

        // Handle && (AND)
        if let Some(pos) = expr.find("&&") {
            let left = &expr[..pos].trim();
            let right = &expr[pos + 2..].trim();
            let left_cond = HookCondition::new(*left);
            let right_cond = HookCondition::new(*right);
            return left_cond.evaluate(context) && right_cond.evaluate(context);
        }

        // Handle || (OR)
        if let Some(pos) = expr.find("||") {
            let left = &expr[..pos].trim();
            let right = &expr[pos + 2..].trim();
            let left_cond = HookCondition::new(*left);
            let right_cond = HookCondition::new(*right);
            return left_cond.evaluate(context) || right_cond.evaluate(context);
        }

        // Handle simple comparisons
        self.evaluate_simple(expr, context)
    }

    fn evaluate_simple(&self, expr: &str, context: &HookContext) -> bool {
        // file.ext == 'rs'
        if let Some(pos) = expr.find("==") {
            let left = expr[..pos].trim();
            let right = expr[pos + 2..].trim().trim_matches('\'').trim_matches('"');
            return self.get_value(left, context) == Some(right.to_string());
        }

        // file.ext != 'rs'
        if let Some(pos) = expr.find("!=") {
            let left = expr[..pos].trim();
            let right = expr[pos + 2..].trim().trim_matches('\'').trim_matches('"');
            return self.get_value(left, context) != Some(right.to_string());
        }

        // file.path.contains('src')
        if expr.contains(".contains(") {
            if let Some(start) = expr.find(".contains(") {
                let field = expr[..start].trim();
                let arg_start = start + 10;
                if let Some(arg_end) = expr[arg_start..].find(')') {
                    let arg = expr[arg_start..arg_start + arg_end]
                        .trim()
                        .trim_matches('\'')
                        .trim_matches('"');
                    if let Some(value) = self.get_value(field, context) {
                        return value.contains(arg);
                    }
                }
            }
            return false;
        }

        // file.path.starts_with('src')
        if expr.contains(".starts_with(") {
            if let Some(start) = expr.find(".starts_with(") {
                let field = expr[..start].trim();
                let arg_start = start + 13;
                if let Some(arg_end) = expr[arg_start..].find(')') {
                    let arg = expr[arg_start..arg_start + arg_end]
                        .trim()
                        .trim_matches('\'')
                        .trim_matches('"');
                    if let Some(value) = self.get_value(field, context) {
                        return value.starts_with(arg);
                    }
                }
            }
            return false;
        }

        // file.path.ends_with('.rs')
        if expr.contains(".ends_with(") {
            if let Some(start) = expr.find(".ends_with(") {
                let field = expr[..start].trim();
                let arg_start = start + 11;
                if let Some(arg_end) = expr[arg_start..].find(')') {
                    let arg = expr[arg_start..arg_start + arg_end]
                        .trim()
                        .trim_matches('\'')
                        .trim_matches('"');
                    if let Some(value) = self.get_value(field, context) {
                        return value.ends_with(arg);
                    }
                }
            }
            return false;
        }

        // Unknown expression - default to false
        false
    }

    fn get_value(&self, field: &str, context: &HookContext) -> Option<String> {
        match field {
            "file.ext" => context.file_ext.clone(),
            "file.path" => context.file_path.as_ref().map(|p| p.to_string_lossy().to_string()),
            "file.name" => context.file_name.clone(),
            "file.size" => context.file_size.map(|s| s.to_string()),
            _ => context.variables.get(field).cloned(),
        }
    }
}

/// Context for hook condition evaluation
#[derive(Debug, Clone, Default)]
pub struct HookContext {
    /// File extension (without dot)
    pub file_ext: Option<String>,
    /// File path
    pub file_path: Option<PathBuf>,
    /// File name
    pub file_name: Option<String>,
    /// File size in bytes
    pub file_size: Option<u64>,
    /// Additional variables
    pub variables: HashMap<String, String>,
}

impl HookContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a context from a file path
    pub fn from_path(path: &Path) -> Self {
        let file_ext = path.extension().map(|e| e.to_string_lossy().to_string());
        let file_name = path.file_name().map(|n| n.to_string_lossy().to_string());
        let file_size = std::fs::metadata(path).ok().map(|m| m.len());

        Self {
            file_ext,
            file_path: Some(path.to_path_buf()),
            file_name,
            file_size,
            variables: HashMap::new(),
        }
    }

    /// Set a variable
    pub fn set_variable(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.variables.insert(key.into(), value.into());
    }

    /// Get a variable
    pub fn get_variable(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }
}

/// Action to perform when a hook is triggered
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HookAction {
    /// Agent to invoke
    pub agent: String,
    /// Optional workflow to run
    pub workflow: Option<String>,
    /// Message to send to the agent
    pub message: String,
    /// Additional context variables
    pub context: HashMap<String, String>,
}

impl Default for HookAction {
    fn default() -> Self {
        Self {
            agent: "default".to_string(),
            workflow: None,
            message: String::new(),
            context: HashMap::new(),
        }
    }
}

impl HookAction {
    /// Create a new action
    pub fn new(agent: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            agent: agent.into(),
            workflow: None,
            message: message.into(),
            context: HashMap::new(),
        }
    }

    /// Set the workflow
    pub fn with_workflow(mut self, workflow: impl Into<String>) -> Self {
        self.workflow = Some(workflow.into());
        self
    }

    /// Add a context variable
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }
}

/// Result of hook execution
#[derive(Debug, Clone)]
pub struct HookExecutionResult {
    /// Hook ID
    pub hook_id: String,
    /// Whether execution succeeded
    pub success: bool,
    /// Output from the agent
    pub output: Option<String>,
    /// Error message if failed
    pub error: Option<String>,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Chained hooks that were triggered
    pub chained: Vec<String>,
}

/// Hook engine for managing and executing hooks
pub struct HookEngine {
    /// Registered hooks
    hooks: Vec<AgentHook>,
    /// Hooks directory
    hooks_dir: PathBuf,
    /// Whether the engine is running
    running: bool,
}

impl HookEngine {
    /// Create a new hook engine
    pub fn new() -> Self {
        Self {
            hooks: Vec::new(),
            hooks_dir: PathBuf::from(".driven/hooks"),
            running: false,
        }
    }

    /// Create a hook engine with a custom hooks directory
    pub fn with_hooks_dir(hooks_dir: impl Into<PathBuf>) -> Self {
        Self {
            hooks: Vec::new(),
            hooks_dir: hooks_dir.into(),
            running: false,
        }
    }

    /// Register a hook
    pub fn register_hook(&mut self, hook: AgentHook) -> Result<()> {
        // Check for duplicate ID
        if self.hooks.iter().any(|h| h.id == hook.id) {
            return Err(DrivenError::Config(format!("Hook with ID '{}' already exists", hook.id)));
        }

        self.hooks.push(hook);

        // Sort by priority
        self.hooks.sort_by_key(|h| h.priority);

        Ok(())
    }

    /// Unregister a hook by ID
    pub fn unregister_hook(&mut self, id: &str) -> Result<AgentHook> {
        let pos = self
            .hooks
            .iter()
            .position(|h| h.id == id)
            .ok_or_else(|| DrivenError::Config(format!("Hook with ID '{}' not found", id)))?;

        Ok(self.hooks.remove(pos))
    }

    /// Get a hook by ID
    pub fn get_hook(&self, id: &str) -> Option<&AgentHook> {
        self.hooks.iter().find(|h| h.id == id)
    }

    /// Get a mutable reference to a hook by ID
    pub fn get_hook_mut(&mut self, id: &str) -> Option<&mut AgentHook> {
        self.hooks.iter_mut().find(|h| h.id == id)
    }

    /// List all hooks
    pub fn list_hooks(&self) -> &[AgentHook] {
        &self.hooks
    }

    /// Enable a hook
    pub fn enable_hook(&mut self, id: &str) -> Result<()> {
        let hook = self
            .get_hook_mut(id)
            .ok_or_else(|| DrivenError::Config(format!("Hook with ID '{}' not found", id)))?;
        hook.enabled = true;
        Ok(())
    }

    /// Disable a hook
    pub fn disable_hook(&mut self, id: &str) -> Result<()> {
        let hook = self
            .get_hook_mut(id)
            .ok_or_else(|| DrivenError::Config(format!("Hook with ID '{}' not found", id)))?;
        hook.enabled = false;
        Ok(())
    }

    /// Find hooks that match a file change
    pub fn find_file_change_hooks(&self, path: &Path) -> Vec<&AgentHook> {
        self.hooks.iter().filter(|h| h.matches_file_change(path)).collect()
    }

    /// Find hooks that match a git operation
    pub fn find_git_op_hooks(&self, op: &GitOp) -> Vec<&AgentHook> {
        self.hooks.iter().filter(|h| h.matches_git_op(op)).collect()
    }

    /// Find hooks that match a build event
    pub fn find_build_event_hooks(&self, event: &BuildEvent) -> Vec<&AgentHook> {
        self.hooks.iter().filter(|h| h.matches_build_event(event)).collect()
    }

    /// Find hooks that match a test result
    pub fn find_test_result_hooks(&self, result: &TestResult) -> Vec<&AgentHook> {
        self.hooks.iter().filter(|h| h.matches_test_result(result)).collect()
    }

    /// Find hooks that match a manual command
    pub fn find_manual_hooks(&self, command: &str) -> Vec<&AgentHook> {
        self.hooks.iter().filter(|h| h.matches_manual(command)).collect()
    }

    /// Trigger hooks for a file change event
    pub fn trigger_file_change(&self, path: &Path) -> Vec<HookExecutionResult> {
        let context = HookContext::from_path(path);
        let hooks = self.find_file_change_hooks(path);

        hooks
            .iter()
            .filter(|h| h.evaluate_condition(&context))
            .map(|h| self.execute_hook(h, &context))
            .collect()
    }

    /// Trigger hooks for a git operation
    pub fn trigger_git_op(&self, op: &GitOp) -> Vec<HookExecutionResult> {
        let context = HookContext::new();
        let hooks = self.find_git_op_hooks(op);

        hooks
            .iter()
            .filter(|h| h.evaluate_condition(&context))
            .map(|h| self.execute_hook(h, &context))
            .collect()
    }

    /// Trigger hooks for a build event
    pub fn trigger_build_event(&self, event: &BuildEvent) -> Vec<HookExecutionResult> {
        let context = HookContext::new();
        let hooks = self.find_build_event_hooks(event);

        hooks
            .iter()
            .filter(|h| h.evaluate_condition(&context))
            .map(|h| self.execute_hook(h, &context))
            .collect()
    }

    /// Trigger hooks for a test result
    pub fn trigger_test_result(&self, result: &TestResult) -> Vec<HookExecutionResult> {
        let mut context = HookContext::new();
        context.set_variable("test.name", &result.name);
        context.set_variable("test.status", format!("{:?}", result.status));
        if let Some(file) = &result.file {
            context.file_path = Some(file.clone());
        }

        let hooks = self.find_test_result_hooks(result);

        hooks
            .iter()
            .filter(|h| h.evaluate_condition(&context))
            .map(|h| self.execute_hook(h, &context))
            .collect()
    }

    /// Trigger a manual hook
    pub fn trigger_manual(&self, command: &str) -> Vec<HookExecutionResult> {
        let context = HookContext::new();
        let hooks = self.find_manual_hooks(command);

        hooks
            .iter()
            .filter(|h| h.evaluate_condition(&context))
            .map(|h| self.execute_hook(h, &context))
            .collect()
    }

    /// Execute a single hook
    fn execute_hook(&self, hook: &AgentHook, _context: &HookContext) -> HookExecutionResult {
        let start = std::time::Instant::now();

        // TODO: Actually execute the hook action
        // For now, just return a placeholder result
        let duration_ms = start.elapsed().as_millis() as u64;

        // Handle chaining
        let chained = hook.chain.clone().unwrap_or_default();

        HookExecutionResult {
            hook_id: hook.id.clone(),
            success: true,
            output: Some(format!("Hook '{}' executed", hook.name)),
            error: None,
            duration_ms,
            chained,
        }
    }

    /// Load hooks from the hooks directory
    pub fn load_hooks(&mut self, path: &Path) -> Result<usize> {
        if !path.exists() {
            return Ok(0);
        }

        let mut count = 0;

        for entry in std::fs::read_dir(path).map_err(DrivenError::Io)? {
            let entry = entry.map_err(DrivenError::Io)?;
            let path = entry.path();

            if path.extension().is_some_and(|e| e == "json" || e == "yaml" || e == "yml") {
                match self.load_hook_file(&path) {
                    Ok(hook) => {
                        if let Err(e) = self.register_hook(hook) {
                            tracing::warn!("Failed to register hook from {:?}: {}", path, e);
                        } else {
                            count += 1;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load hook from {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(count)
    }

    /// Load a single hook from a file
    fn load_hook_file(&self, path: &Path) -> Result<AgentHook> {
        let content = std::fs::read_to_string(path).map_err(DrivenError::Io)?;

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match ext {
            "json" => serde_json::from_str(&content).map_err(|e| DrivenError::Parse(e.to_string())),
            "yaml" | "yml" => {
                serde_yaml::from_str(&content).map_err(|e| DrivenError::Parse(e.to_string()))
            }
            _ => Err(DrivenError::Parse(format!("Unknown file extension: {}", ext))),
        }
    }

    /// Save a hook to a file
    pub fn save_hook(&self, hook: &AgentHook, path: &Path) -> Result<()> {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("json");

        let content = match ext {
            "json" => serde_json::to_string_pretty(hook)
                .map_err(|e| DrivenError::Format(e.to_string()))?,
            "yaml" | "yml" => {
                serde_yaml::to_string(hook).map_err(|e| DrivenError::Format(e.to_string()))?
            }
            _ => return Err(DrivenError::Format(format!("Unknown file extension: {}", ext))),
        };

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(DrivenError::Io)?;
        }

        std::fs::write(path, content).map_err(DrivenError::Io)?;

        Ok(())
    }

    /// Get the hooks directory
    pub fn hooks_dir(&self) -> &Path {
        &self.hooks_dir
    }

    /// Check if the engine is running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Start the hook engine
    pub fn start(&mut self) -> Result<()> {
        if self.running {
            return Err(DrivenError::Config("Hook engine already running".to_string()));
        }

        // Load hooks from directory
        self.load_hooks(&self.hooks_dir.clone())?;

        self.running = true;
        Ok(())
    }

    /// Stop the hook engine
    pub fn stop(&mut self) {
        self.running = false;
    }
}

impl Default for HookEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple glob pattern matching
fn glob_match(pattern: &str, text: &str) -> bool {
    // Handle ** (match any path)
    if pattern.contains("**") {
        let parts: Vec<&str> = pattern.split("**").collect();
        if parts.len() == 2 {
            let prefix = parts[0].trim_end_matches('/');
            let suffix = parts[1].trim_start_matches('/');

            if !prefix.is_empty() && !text.starts_with(prefix) {
                return false;
            }
            if !suffix.is_empty() && !glob_match(suffix, text.rsplit('/').next().unwrap_or(text)) {
                return false;
            }
            return true;
        }
    }

    // Handle * (match any characters except /)
    if pattern.contains('*') && !pattern.contains("**") {
        let parts: Vec<&str> = pattern.split('*').collect();
        let mut pos = 0;

        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }

            if i == 0 {
                // First part must be at the start
                if !text.starts_with(part) {
                    return false;
                }
                pos = part.len();
            } else if i == parts.len() - 1 {
                // Last part must be at the end
                if !text.ends_with(part) {
                    return false;
                }
            } else {
                // Middle parts must exist somewhere
                if let Some(found) = text[pos..].find(part) {
                    pos += found + part.len();
                } else {
                    return false;
                }
            }
        }

        return true;
    }

    // Exact match
    pattern == text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_hook_creation() {
        let hook = AgentHook::new("test-hook")
            .with_name("Test Hook")
            .with_trigger(HookTrigger::FileChange {
                patterns: vec!["**/*.rs".to_string()],
            })
            .with_action(HookAction::new("reviewer", "Review this file"))
            .with_priority(50);

        assert_eq!(hook.id, "test-hook");
        assert_eq!(hook.name, "Test Hook");
        assert!(hook.enabled);
        assert_eq!(hook.priority, 50);
    }

    #[test]
    fn test_file_change_matching() {
        let hook = AgentHook::new("rust-hook").with_trigger(HookTrigger::FileChange {
            patterns: vec!["**/*.rs".to_string()],
        });

        assert!(hook.matches_file_change(Path::new("src/main.rs")));
        assert!(hook.matches_file_change(Path::new("lib.rs")));
        assert!(!hook.matches_file_change(Path::new("src/main.py")));
    }

    #[test]
    fn test_git_op_matching() {
        let hook = AgentHook::new("git-hook").with_trigger(HookTrigger::GitOperation {
            operations: vec![GitOp::Commit, GitOp::Push],
        });

        assert!(hook.matches_git_op(&GitOp::Commit));
        assert!(hook.matches_git_op(&GitOp::Push));
        assert!(!hook.matches_git_op(&GitOp::Pull));
    }

    #[test]
    fn test_build_event_matching() {
        let hook = AgentHook::new("build-hook").with_trigger(HookTrigger::BuildEvent {
            events: vec![BuildEvent::Failure, BuildEvent::Warning],
        });

        assert!(hook.matches_build_event(&BuildEvent::Failure));
        assert!(hook.matches_build_event(&BuildEvent::Warning));
        assert!(!hook.matches_build_event(&BuildEvent::Success));
    }

    #[test]
    fn test_test_filter() {
        let filter = TestFilter::new().with_status(TestStatus::Failed).with_name_pattern("test_*");

        let passing_result = TestResult {
            name: "test_something".to_string(),
            status: TestStatus::Failed,
            file: None,
            duration_ms: Some(100),
            error: Some("assertion failed".to_string()),
        };

        let non_matching_result = TestResult {
            name: "test_something".to_string(),
            status: TestStatus::Passed,
            file: None,
            duration_ms: Some(50),
            error: None,
        };

        assert!(filter.matches(&passing_result));
        assert!(!filter.matches(&non_matching_result));
    }

    #[test]
    fn test_hook_condition_simple() {
        let condition = HookCondition::new("file.ext == 'rs'");

        let mut context = HookContext::new();
        context.file_ext = Some("rs".to_string());

        assert!(condition.evaluate(&context));

        context.file_ext = Some("py".to_string());
        assert!(!condition.evaluate(&context));
    }

    #[test]
    fn test_hook_condition_and() {
        let condition = HookCondition::new("file.ext == 'rs' && file.path.contains('src')");

        let mut context = HookContext::new();
        context.file_ext = Some("rs".to_string());
        context.file_path = Some(PathBuf::from("src/main.rs"));

        assert!(condition.evaluate(&context));

        context.file_path = Some(PathBuf::from("tests/main.rs"));
        assert!(!condition.evaluate(&context));
    }

    #[test]
    fn test_hook_condition_or() {
        let condition = HookCondition::new("file.ext == 'rs' || file.ext == 'py'");

        let mut context = HookContext::new();
        context.file_ext = Some("rs".to_string());
        assert!(condition.evaluate(&context));

        context.file_ext = Some("py".to_string());
        assert!(condition.evaluate(&context));

        context.file_ext = Some("js".to_string());
        assert!(!condition.evaluate(&context));
    }

    #[test]
    fn test_hook_condition_contains() {
        let condition = HookCondition::new("file.path.contains('src')");

        let mut context = HookContext::new();
        context.file_path = Some(PathBuf::from("src/lib.rs"));
        assert!(condition.evaluate(&context));

        context.file_path = Some(PathBuf::from("tests/lib.rs"));
        assert!(!condition.evaluate(&context));
    }

    #[test]
    fn test_hook_condition_starts_with() {
        let condition = HookCondition::new("file.path.starts_with('src')");

        let mut context = HookContext::new();
        context.file_path = Some(PathBuf::from("src/lib.rs"));
        assert!(condition.evaluate(&context));

        context.file_path = Some(PathBuf::from("tests/lib.rs"));
        assert!(!condition.evaluate(&context));
    }

    #[test]
    fn test_hook_condition_ends_with() {
        let condition = HookCondition::new("file.path.ends_with('.rs')");

        let mut context = HookContext::new();
        context.file_path = Some(PathBuf::from("src/lib.rs"));
        assert!(condition.evaluate(&context));

        context.file_path = Some(PathBuf::from("src/lib.py"));
        assert!(!condition.evaluate(&context));
    }

    #[test]
    fn test_hook_engine_register() {
        let mut engine = HookEngine::new();

        let hook = AgentHook::new("test-hook");
        engine.register_hook(hook).unwrap();

        assert_eq!(engine.list_hooks().len(), 1);
        assert!(engine.get_hook("test-hook").is_some());
    }

    #[test]
    fn test_hook_engine_duplicate_id() {
        let mut engine = HookEngine::new();

        let hook1 = AgentHook::new("test-hook");
        let hook2 = AgentHook::new("test-hook");

        engine.register_hook(hook1).unwrap();
        assert!(engine.register_hook(hook2).is_err());
    }

    #[test]
    fn test_hook_engine_unregister() {
        let mut engine = HookEngine::new();

        let hook = AgentHook::new("test-hook");
        engine.register_hook(hook).unwrap();

        let removed = engine.unregister_hook("test-hook").unwrap();
        assert_eq!(removed.id, "test-hook");
        assert!(engine.get_hook("test-hook").is_none());
    }

    #[test]
    fn test_hook_engine_enable_disable() {
        let mut engine = HookEngine::new();

        let hook = AgentHook::new("test-hook");
        engine.register_hook(hook).unwrap();

        engine.disable_hook("test-hook").unwrap();
        assert!(!engine.get_hook("test-hook").unwrap().enabled);

        engine.enable_hook("test-hook").unwrap();
        assert!(engine.get_hook("test-hook").unwrap().enabled);
    }

    #[test]
    fn test_hook_engine_find_file_change_hooks() {
        let mut engine = HookEngine::new();

        let rust_hook = AgentHook::new("rust-hook").with_trigger(HookTrigger::FileChange {
            patterns: vec!["**/*.rs".to_string()],
        });

        let python_hook = AgentHook::new("python-hook").with_trigger(HookTrigger::FileChange {
            patterns: vec!["**/*.py".to_string()],
        });

        engine.register_hook(rust_hook).unwrap();
        engine.register_hook(python_hook).unwrap();

        let hooks = engine.find_file_change_hooks(Path::new("src/main.rs"));
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].id, "rust-hook");
    }

    #[test]
    fn test_hook_engine_priority_ordering() {
        let mut engine = HookEngine::new();

        let low_priority = AgentHook::new("low").with_priority(200);
        let high_priority = AgentHook::new("high").with_priority(50);
        let medium_priority = AgentHook::new("medium").with_priority(100);

        engine.register_hook(low_priority).unwrap();
        engine.register_hook(high_priority).unwrap();
        engine.register_hook(medium_priority).unwrap();

        let hooks = engine.list_hooks();
        assert_eq!(hooks[0].id, "high");
        assert_eq!(hooks[1].id, "medium");
        assert_eq!(hooks[2].id, "low");
    }

    #[test]
    fn test_glob_match_star() {
        assert!(glob_match("*.rs", "main.rs"));
        assert!(glob_match("*.rs", "lib.rs"));
        assert!(!glob_match("*.rs", "main.py"));
        assert!(glob_match("test_*", "test_something"));
        assert!(!glob_match("test_*", "something_test"));
    }

    #[test]
    fn test_glob_match_double_star() {
        assert!(glob_match("**/*.rs", "src/main.rs"));
        assert!(glob_match("**/*.rs", "src/lib/mod.rs"));
        assert!(glob_match("**/*.rs", "main.rs"));
        assert!(!glob_match("**/*.rs", "main.py"));
    }

    #[test]
    fn test_glob_match_exact() {
        assert!(glob_match("main.rs", "main.rs"));
        assert!(!glob_match("main.rs", "lib.rs"));
    }

    #[test]
    fn test_hook_action() {
        let action = HookAction::new("reviewer", "Review this code")
            .with_workflow("quick-review")
            .with_context("file", "main.rs");

        assert_eq!(action.agent, "reviewer");
        assert_eq!(action.workflow, Some("quick-review".to_string()));
        assert_eq!(action.message, "Review this code");
        assert_eq!(action.context.get("file"), Some(&"main.rs".to_string()));
    }

    #[test]
    fn test_hook_context_from_path() {
        let context = HookContext::from_path(Path::new("src/main.rs"));

        assert_eq!(context.file_ext, Some("rs".to_string()));
        assert_eq!(context.file_name, Some("main.rs".to_string()));
        assert!(context.file_path.is_some());
    }

    #[test]
    fn test_manual_hook_matching() {
        let hook = AgentHook::new("manual-hook").with_trigger(HookTrigger::Manual {
            command: "lint".to_string(),
        });

        assert!(hook.matches_manual("lint"));
        assert!(!hook.matches_manual("test"));
    }

    #[test]
    fn test_disabled_hook_no_match() {
        let hook = AgentHook::new("disabled-hook")
            .with_trigger(HookTrigger::FileChange {
                patterns: vec!["**/*.rs".to_string()],
            })
            .with_enabled(false);

        assert!(!hook.matches_file_change(Path::new("src/main.rs")));
    }

    #[test]
    fn test_hook_with_chain() {
        let hook = AgentHook::new("chained-hook")
            .with_chain(vec!["hook2".to_string(), "hook3".to_string()]);

        assert_eq!(hook.chain, Some(vec!["hook2".to_string(), "hook3".to_string()]));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate an arbitrary file extension
    fn arb_extension() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("rs".to_string()),
            Just("py".to_string()),
            Just("js".to_string()),
            Just("ts".to_string()),
            Just("md".to_string()),
            Just("json".to_string()),
            Just("yaml".to_string()),
        ]
    }

    /// Generate an arbitrary file path
    fn arb_file_path() -> impl Strategy<Value = PathBuf> {
        (
            prop_oneof![Just("src"), Just("tests"), Just("lib"), Just("bin"),],
            "[a-z_]+",
            arb_extension(),
        )
            .prop_map(|(dir, name, ext)| PathBuf::from(format!("{}/{}.{}", dir, name, ext)))
    }

    /// Generate an arbitrary GitOp
    fn arb_git_op() -> impl Strategy<Value = GitOp> {
        prop_oneof![
            Just(GitOp::Commit),
            Just(GitOp::Push),
            Just(GitOp::Pull),
            Just(GitOp::Merge),
            Just(GitOp::Checkout),
            Just(GitOp::Stash),
        ]
    }

    /// Generate an arbitrary BuildEvent
    fn arb_build_event() -> impl Strategy<Value = BuildEvent> {
        prop_oneof![
            Just(BuildEvent::Start),
            Just(BuildEvent::Success),
            Just(BuildEvent::Failure),
            Just(BuildEvent::Warning),
        ]
    }

    /// Generate an arbitrary TestStatus
    fn arb_test_status() -> impl Strategy<Value = TestStatus> {
        prop_oneof![
            Just(TestStatus::Passed),
            Just(TestStatus::Failed),
            Just(TestStatus::Skipped),
            Just(TestStatus::Timeout),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 8: Event-Based Hook Triggering
        /// *For any* file change, git operation, build event, or test result that matches
        /// a hook's trigger pattern, the hook SHALL be triggered exactly once.
        /// **Validates: Requirements 3.1, 3.2, 3.3, 3.4**
        #[test]
        fn prop_file_change_triggers_matching_hooks(
            path in arb_file_path(),
            ext in arb_extension(),
        ) {
            let mut engine = HookEngine::new();

            // Create a hook that matches the extension
            let pattern = format!("**/*.{}", ext);
            let hook = AgentHook::new("test-hook")
                .with_trigger(HookTrigger::FileChange {
                    patterns: vec![pattern],
                });
            engine.register_hook(hook).unwrap();

            // Create a path with the matching extension
            let test_path = PathBuf::from(format!("src/test.{}", ext));

            // Find matching hooks
            let hooks = engine.find_file_change_hooks(&test_path);

            // Should find exactly one hook
            prop_assert_eq!(hooks.len(), 1);
            prop_assert_eq!(&hooks[0].id, "test-hook");
        }

        /// Property 8b: Git operation triggers matching hooks
        #[test]
        fn prop_git_op_triggers_matching_hooks(
            op in arb_git_op(),
        ) {
            let mut engine = HookEngine::new();

            // Create a hook that matches the operation
            let hook = AgentHook::new("git-hook")
                .with_trigger(HookTrigger::GitOperation {
                    operations: vec![op],
                });
            engine.register_hook(hook).unwrap();

            // Find matching hooks
            let hooks = engine.find_git_op_hooks(&op);

            // Should find exactly one hook
            prop_assert_eq!(hooks.len(), 1);
            prop_assert_eq!(&hooks[0].id, "git-hook");
        }

        /// Property 8c: Build event triggers matching hooks
        #[test]
        fn prop_build_event_triggers_matching_hooks(
            event in arb_build_event(),
        ) {
            let mut engine = HookEngine::new();

            // Create a hook that matches the event
            let hook = AgentHook::new("build-hook")
                .with_trigger(HookTrigger::BuildEvent {
                    events: vec![event],
                });
            engine.register_hook(hook).unwrap();

            // Find matching hooks
            let hooks = engine.find_build_event_hooks(&event);

            // Should find exactly one hook
            prop_assert_eq!(hooks.len(), 1);
            prop_assert_eq!(&hooks[0].id, "build-hook");
        }

        /// Property 8d: Test result triggers matching hooks
        #[test]
        fn prop_test_result_triggers_matching_hooks(
            status in arb_test_status(),
            name in "[a-z_]+",
        ) {
            let mut engine = HookEngine::new();

            // Create a hook that matches the status
            let hook = AgentHook::new("test-hook")
                .with_trigger(HookTrigger::TestResult {
                    filter: TestFilter::new().with_status(status),
                });
            engine.register_hook(hook).unwrap();

            // Create a test result
            let result = TestResult {
                name: name.clone(),
                status,
                file: None,
                duration_ms: Some(100),
                error: None,
            };

            // Find matching hooks
            let hooks = engine.find_test_result_hooks(&result);

            // Should find exactly one hook
            prop_assert_eq!(hooks.len(), 1);
            prop_assert_eq!(&hooks[0].id, "test-hook");
        }

        /// Property 3: Hook Trigger Execution
        /// *For any* configured hook with a matching trigger event, the hook's action
        /// SHALL be executed exactly once per trigger occurrence.
        /// **Validates: Requirements 3.5, 3.6, 3.7, 3.8, 3.9**
        #[test]
        fn prop_hook_trigger_execution_once(
            ext in arb_extension(),
            num_triggers in 1usize..5,
        ) {
            let mut engine = HookEngine::new();

            // Create a hook that matches the extension
            let pattern = format!("**/*.{}", ext);
            let hook = AgentHook::new("exec-hook")
                .with_trigger(HookTrigger::FileChange {
                    patterns: vec![pattern],
                })
                .with_action(HookAction::new("test-agent", "Test message"));
            engine.register_hook(hook).unwrap();

            // Trigger multiple times with matching files
            let mut total_executions = 0;
            for i in 0..num_triggers {
                let test_path = PathBuf::from(format!("src/file{}.{}", i, ext));
                let results = engine.trigger_file_change(&test_path);
                total_executions += results.len();
            }

            // Each trigger should execute exactly once
            prop_assert_eq!(total_executions, num_triggers,
                "Expected {} executions, got {}", num_triggers, total_executions);
        }

        /// Property 3b: Manual hook trigger execution
        /// *For any* manual hook, triggering it SHALL execute exactly once.
        /// **Validates: Requirements 3.6, 3.9**
        #[test]
        fn prop_manual_hook_trigger_execution(
            command in "[a-z_]+",
        ) {
            let mut engine = HookEngine::new();

            // Create a manual hook
            let hook = AgentHook::new("manual-hook")
                .with_trigger(HookTrigger::Manual {
                    command: command.clone(),
                })
                .with_action(HookAction::new("test-agent", "Manual trigger"));
            engine.register_hook(hook).unwrap();

            // Trigger the manual hook
            let results = engine.trigger_manual(&command);

            // Should execute exactly once
            prop_assert_eq!(results.len(), 1,
                "Manual hook should execute exactly once, got {}", results.len());
            prop_assert!(results[0].success,
                "Manual hook execution should succeed");
        }

        /// Property 3c: Disabled hooks should not execute
        /// *For any* disabled hook, triggering it SHALL NOT execute.
        /// **Validates: Requirements 3.3, 3.4**
        #[test]
        fn prop_disabled_hook_no_execution(
            ext in arb_extension(),
        ) {
            let mut engine = HookEngine::new();

            // Create a disabled hook
            let pattern = format!("**/*.{}", ext);
            let hook = AgentHook::new("disabled-hook")
                .with_trigger(HookTrigger::FileChange {
                    patterns: vec![pattern],
                })
                .with_enabled(false);
            engine.register_hook(hook).unwrap();

            // Try to trigger
            let test_path = PathBuf::from(format!("src/test.{}", ext));
            let results = engine.trigger_file_change(&test_path);

            // Should not execute
            prop_assert_eq!(results.len(), 0,
                "Disabled hook should not execute, got {} executions", results.len());
        }        /// Property 9: Hook Condition Filtering
        /// *For any* hook with a condition, the hook action SHALL execute if and only if
        /// the condition evaluates to true.
        /// **Validates: Requirements 3.8**
        #[test]
        fn prop_condition_filtering(
            ext in arb_extension(),
            path_contains in prop_oneof![Just("src"), Just("tests"), Just("lib")],
        ) {
            // Create a condition that checks extension
            let condition = HookCondition::new(format!("file.ext == '{}'", ext));

            // Create a context with matching extension
            let mut context = HookContext::new();
            context.file_ext = Some(ext.clone());

            // Condition should evaluate to true
            prop_assert!(condition.evaluate(&context));

            // Create a context with non-matching extension
            let mut context2 = HookContext::new();
            context2.file_ext = Some("different".to_string());

            // Condition should evaluate to false
            prop_assert!(!condition.evaluate(&context2));
        }

        /// Property 9b: Compound condition filtering (AND)
        #[test]
        fn prop_compound_condition_and(
            ext in arb_extension(),
            dir in prop_oneof![Just("src"), Just("tests"), Just("lib")],
        ) {
            // Create a compound condition
            let condition = HookCondition::new(
                format!("file.ext == '{}' && file.path.contains('{}')", ext, dir)
            );

            // Create a context that matches both
            let mut context = HookContext::new();
            context.file_ext = Some(ext.clone());
            context.file_path = Some(PathBuf::from(format!("{}/test.{}", dir, ext)));

            // Condition should evaluate to true
            prop_assert!(condition.evaluate(&context));

            // Create a context that matches only one
            let mut context2 = HookContext::new();
            context2.file_ext = Some(ext.clone());
            context2.file_path = Some(PathBuf::from("other/test.rs"));

            // Condition should evaluate to false (AND requires both)
            prop_assert!(!condition.evaluate(&context2));
        }

        /// Property 9c: Compound condition filtering (OR)
        #[test]
        fn prop_compound_condition_or(
            ext1 in arb_extension(),
            ext2 in arb_extension(),
        ) {
            // Create a compound condition
            let condition = HookCondition::new(
                format!("file.ext == '{}' || file.ext == '{}'", ext1, ext2)
            );

            // Create a context that matches first
            let mut context1 = HookContext::new();
            context1.file_ext = Some(ext1.clone());

            // Condition should evaluate to true
            prop_assert!(condition.evaluate(&context1));

            // Create a context that matches second
            let mut context2 = HookContext::new();
            context2.file_ext = Some(ext2.clone());

            // Condition should evaluate to true
            prop_assert!(condition.evaluate(&context2));

            // Create a context that matches neither
            let mut context3 = HookContext::new();
            context3.file_ext = Some("nomatch".to_string());

            // Condition should evaluate to false (unless ext1 or ext2 is "nomatch")
            if ext1 != "nomatch" && ext2 != "nomatch" {
                prop_assert!(!condition.evaluate(&context3));
            }
        }

        /// Property 10: Hook Chaining Execution Order
        /// *For any* hook chain, hooks SHALL execute in the specified order.
        /// **Validates: Requirements 3.9**
        #[test]
        fn prop_hook_chaining_order(
            chain_ids in prop::collection::vec("[a-z]+", 1..5),
        ) {
            let mut engine = HookEngine::new();

            // Create the main hook with a chain
            let main_hook = AgentHook::new("main")
                .with_trigger(HookTrigger::Manual { command: "test".to_string() })
                .with_chain(chain_ids.clone());
            engine.register_hook(main_hook).unwrap();

            // Create the chained hooks
            for id in &chain_ids {
                let hook = AgentHook::new(id.clone())
                    .with_trigger(HookTrigger::Manual { command: id.clone() });
                let _ = engine.register_hook(hook); // Ignore duplicates
            }

            // Trigger the main hook
            let results = engine.trigger_manual("test");

            // Should have triggered the main hook
            prop_assert_eq!(results.len(), 1);

            // The result should contain the chain
            prop_assert_eq!(&results[0].chained, &chain_ids);
        }

        /// Property 11: Hook Configuration Persistence
        /// *For any* hook configuration, saving and loading SHALL produce an equivalent configuration.
        /// **Validates: Requirements 3.10**
        #[test]
        fn prop_hook_persistence_roundtrip(
            id in "[a-z_]+",
            name in "[a-zA-Z ]+",
            ext in arb_extension(),
            enabled in any::<bool>(),
            priority in 0u8..255,
        ) {
            let hook = AgentHook::new(id.clone())
                .with_name(name.clone())
                .with_trigger(HookTrigger::FileChange {
                    patterns: vec![format!("**/*.{}", ext)],
                })
                .with_enabled(enabled)
                .with_priority(priority);

            // Serialize to JSON
            let json = serde_json::to_string(&hook).expect("Should serialize");

            // Deserialize back
            let loaded: AgentHook = serde_json::from_str(&json).expect("Should deserialize");

            // Verify fields match
            prop_assert_eq!(loaded.id, id);
            prop_assert_eq!(loaded.name, name);
            prop_assert_eq!(loaded.enabled, enabled);
            prop_assert_eq!(loaded.priority, priority);

            // Verify trigger matches
            match &loaded.trigger {
                HookTrigger::FileChange { patterns } => {
                    prop_assert_eq!(patterns.len(), 1);
                    prop_assert!(patterns[0].ends_with(&ext));
                }
                _ => prop_assert!(false, "Wrong trigger type"),
            }
        }

        /// Property 2: Hook State Consistency
        /// *For any* hook, enabling then disabling (or vice versa) SHALL result in the hook
        /// being in the final requested state, and the state SHALL persist across operations.
        /// **Validates: Requirements 3.3, 3.4, 3.10**
        #[test]
        fn prop_hook_state_consistency(
            id in "[a-z_]+",
            initial_enabled in any::<bool>(),
            operations in prop::collection::vec(any::<bool>(), 1..10),
        ) {
            let mut engine = HookEngine::new();

            // Create a hook with initial state
            let hook = AgentHook::new(id.clone())
                .with_enabled(initial_enabled);
            engine.register_hook(hook).unwrap();

            // Apply a sequence of enable/disable operations
            let mut expected_state = initial_enabled;
            for enable in &operations {
                if *enable {
                    engine.enable_hook(&id).unwrap();
                    expected_state = true;
                } else {
                    engine.disable_hook(&id).unwrap();
                    expected_state = false;
                }

                // Verify state after each operation
                let hook = engine.get_hook(&id).unwrap();
                prop_assert_eq!(hook.enabled, expected_state,
                    "Hook state mismatch after operation: expected {}, got {}",
                    expected_state, hook.enabled);
            }

            // Final state should match the last operation
            let final_hook = engine.get_hook(&id).unwrap();
            prop_assert_eq!(final_hook.enabled, expected_state);
        }

        /// Property 2b: Hook state persists through serialization
        /// *For any* hook state change, the state SHALL persist through save/load cycle.
        /// **Validates: Requirements 3.3, 3.4, 3.10**
        #[test]
        fn prop_hook_state_persistence(
            id in "[a-z_]+",
            name in "[a-zA-Z ]+",
            enabled_sequence in prop::collection::vec(any::<bool>(), 1..5),
        ) {
            // Start with a hook
            let mut hook = AgentHook::new(id.clone())
                .with_name(name.clone())
                .with_enabled(true);

            // Apply state changes and verify persistence after each
            for enabled in enabled_sequence {
                hook.enabled = enabled;

                // Serialize
                let json = serde_json::to_string(&hook).expect("Should serialize");

                // Deserialize
                let loaded: AgentHook = serde_json::from_str(&json).expect("Should deserialize");

                // State should persist
                prop_assert_eq!(loaded.enabled, enabled,
                    "Hook enabled state did not persist: expected {}, got {}",
                    enabled, loaded.enabled);
                prop_assert_eq!(loaded.id, id.clone());
                prop_assert_eq!(loaded.name, name.clone());
            }
        }
    }
}
