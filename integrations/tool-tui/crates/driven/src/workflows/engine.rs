//! Workflow Engine
//!
//! Manages workflow execution, progress tracking, and resumption.

use super::{StepResult, Workflow, WorkflowPhase, WorkflowProgress, WorkflowSession};
use crate::{DrivenError, Result};
use std::collections::HashMap;

/// Engine for managing workflow execution
#[derive(Debug, Default)]
pub struct WorkflowEngine {
    workflows: HashMap<String, Workflow>,
    sessions: HashMap<String, WorkflowSession>,
    progress_dir: Option<std::path::PathBuf>,
}

impl WorkflowEngine {
    /// Create a new workflow engine
    pub fn new() -> Self {
        Self {
            workflows: HashMap::new(),
            sessions: HashMap::new(),
            progress_dir: None,
        }
    }

    /// Set the directory for persisting progress
    pub fn with_progress_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.progress_dir = Some(dir.into());
        self
    }

    /// Load all built-in workflows
    pub fn load_builtin(&mut self) {
        let builtin_workflows = super::builtin::all_workflows();
        for workflow in builtin_workflows {
            self.workflows.insert(workflow.id.clone(), workflow);
        }
    }

    /// Register a workflow
    pub fn register(&mut self, workflow: Workflow) -> Result<()> {
        if self.workflows.contains_key(&workflow.id) {
            return Err(DrivenError::Config(format!(
                "Workflow with id '{}' already registered",
                workflow.id
            )));
        }
        self.workflows.insert(workflow.id.clone(), workflow);
        Ok(())
    }

    /// Get a workflow by ID
    pub fn get(&self, id: &str) -> Option<&Workflow> {
        self.workflows.get(id)
    }

    /// List all workflows
    pub fn list(&self) -> Vec<&Workflow> {
        self.workflows.values().collect()
    }

    /// List workflows by phase
    pub fn list_by_phase(&self, phase: WorkflowPhase) -> Vec<&Workflow> {
        self.workflows.values().filter(|w| w.phase == phase).collect()
    }

    /// Start a new workflow session
    pub fn start(&mut self, workflow_id: &str) -> Result<WorkflowSession> {
        let workflow = self
            .workflows
            .get(workflow_id)
            .ok_or_else(|| DrivenError::Config(format!("Workflow '{}' not found", workflow_id)))?
            .clone();

        let session_id = uuid::Uuid::new_v4().to_string();
        let progress = WorkflowProgress::new(&session_id, &workflow);

        let session = WorkflowSession {
            id: session_id.clone(),
            workflow,
            progress,
        };

        self.sessions.insert(session_id.clone(), session.clone());
        self.persist_progress(&session)?;

        Ok(session)
    }

    /// Resume an existing workflow session
    pub fn resume(&mut self, session_id: &str) -> Result<WorkflowSession> {
        // Try to get from memory first
        if let Some(session) = self.sessions.get(session_id) {
            return Ok(session.clone());
        }

        // Try to load from disk
        if let Some(ref dir) = self.progress_dir {
            let path = dir.join(format!("{}.json", session_id));
            if path.exists() {
                let content = std::fs::read_to_string(&path)?;
                let progress: WorkflowProgress = serde_json::from_str(&content)
                    .map_err(|e| DrivenError::Config(format!("Failed to parse progress: {}", e)))?;

                let workflow = self
                    .workflows
                    .get(&progress.workflow_id)
                    .ok_or_else(|| {
                        DrivenError::Config(format!(
                            "Workflow '{}' not found",
                            progress.workflow_id
                        ))
                    })?
                    .clone();

                let session = WorkflowSession {
                    id: session_id.to_string(),
                    workflow,
                    progress,
                };

                self.sessions.insert(session_id.to_string(), session.clone());
                return Ok(session);
            }
        }

        Err(DrivenError::Config(format!("Session '{}' not found", session_id)))
    }

    /// Execute the current step in a session
    pub fn execute_step(&mut self, session: &mut WorkflowSession) -> Result<StepResult> {
        let current_step_id = session.progress.current_step.clone();
        let step = session
            .workflow
            .get_step(&current_step_id)
            .ok_or_else(|| DrivenError::Config(format!("Step '{}' not found", current_step_id)))?;

        // Check condition if present
        if let Some(ref condition) = step.condition {
            if !self.evaluate_condition(condition, &session.progress)? {
                // Skip this step, move to next
                if let Some(next) = session.workflow.next_step(&current_step_id) {
                    session.progress.set_current_step(&next.id);
                    self.persist_progress(session)?;
                    return Ok(StepResult::success(Some(
                        "Step skipped due to condition".to_string(),
                    ))
                    .with_next_step(&next.id));
                }
            }
        }

        // Mark step as completed
        session.progress.complete_step(&current_step_id);

        // Determine next step
        let next_step = if !step.branches.is_empty() {
            // Evaluate branches
            let mut next = None;
            for branch in &step.branches {
                if self.evaluate_condition(&branch.condition, &session.progress)? {
                    next = Some(branch.next_step.clone());
                    break;
                }
            }
            next.or_else(|| session.workflow.next_step(&current_step_id).map(|s| s.id.clone()))
        } else {
            session.workflow.next_step(&current_step_id).map(|s| s.id.clone())
        };

        // Update current step
        if let Some(ref next_id) = next_step {
            session.progress.set_current_step(next_id);
        }

        self.persist_progress(session)?;

        // Create result
        let mut result = StepResult::success(Some(format!("Completed step: {}", step.name)));
        if let Some(next_id) = next_step {
            result = result.with_next_step(next_id);
        }
        if step.is_checkpoint {
            result = result.with_pause();
        }

        Ok(result)
    }

    /// Get progress for a session
    pub fn get_progress(&self, session_id: &str) -> Option<&WorkflowProgress> {
        self.sessions.get(session_id).map(|s| &s.progress)
    }

    /// Substitute variables in a template string
    pub fn substitute_variables(&self, template: &str, progress: &WorkflowProgress) -> String {
        let mut result = template.to_string();
        for (key, value) in &progress.variables {
            result = result.replace(&format!("{{{}}}", key), value);
        }
        result
    }

    /// Evaluate a condition expression
    fn evaluate_condition(&self, condition: &str, progress: &WorkflowProgress) -> Result<bool> {
        // Simple condition evaluation
        // Format: "variable == value" or "variable != value"
        let condition = condition.trim();

        if condition.contains("==") {
            let parts: Vec<&str> = condition.split("==").collect();
            if parts.len() == 2 {
                let var_name = parts[0].trim();
                let expected = parts[1].trim().trim_matches('"');
                let actual = progress.get_variable(var_name).map(|s| s.as_str()).unwrap_or("");
                return Ok(actual == expected);
            }
        } else if condition.contains("!=") {
            let parts: Vec<&str> = condition.split("!=").collect();
            if parts.len() == 2 {
                let var_name = parts[0].trim();
                let expected = parts[1].trim().trim_matches('"');
                let actual = progress.get_variable(var_name).map(|s| s.as_str()).unwrap_or("");
                return Ok(actual != expected);
            }
        }

        // Default to true for unrecognized conditions
        Ok(true)
    }

    /// Persist progress to disk
    fn persist_progress(&self, session: &WorkflowSession) -> Result<()> {
        if let Some(ref dir) = self.progress_dir {
            std::fs::create_dir_all(dir)?;
            let path = dir.join(format!("{}.json", session.id));
            let content = serde_json::to_string_pretty(&session.progress)
                .map_err(|e| DrivenError::Config(format!("Failed to serialize progress: {}", e)))?;
            std::fs::write(path, content)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_new() {
        let engine = WorkflowEngine::new();
        assert!(engine.workflows.is_empty());
    }

    #[test]
    fn test_load_builtin() {
        let mut engine = WorkflowEngine::new();
        engine.load_builtin();
        assert!(!engine.workflows.is_empty());
        assert!(engine.workflows.len() >= 30);
    }

    #[test]
    fn test_start_workflow() {
        let mut engine = WorkflowEngine::new();
        engine.load_builtin();

        let session = engine.start("quick-bug-fix");
        assert!(session.is_ok());

        let session = session.unwrap();
        assert!(!session.id.is_empty());
        assert_eq!(session.workflow.id, "quick-bug-fix");
    }

    #[test]
    fn test_list_by_phase() {
        let mut engine = WorkflowEngine::new();
        engine.load_builtin();

        let analysis = engine.list_by_phase(WorkflowPhase::Analysis);
        assert!(!analysis.is_empty());

        let quick_flow = engine.list_by_phase(WorkflowPhase::QuickFlow);
        assert!(!quick_flow.is_empty());
    }

    #[test]
    fn test_variable_substitution() {
        let engine = WorkflowEngine::new();
        let workflow = Workflow::new("test", "Test", WorkflowPhase::Analysis, "Test workflow")
            .with_variable("project", "my-project")
            .with_variable("author", "John");

        let progress = WorkflowProgress::new("session-1", &workflow);

        let template = "Project: {project}, Author: {author}";
        let result = engine.substitute_variables(template, &progress);

        assert_eq!(result, "Project: my-project, Author: John");
    }
}
