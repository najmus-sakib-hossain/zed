//! Expanded Workflow Library
//!
//! Provides 30+ guided workflows for common development tasks,
//! matching and exceeding BMAD-METHOD capabilities.

mod builtin;
mod engine;

#[cfg(test)]
mod property_tests;

pub use builtin::*;
pub use engine::WorkflowEngine;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A guided workflow for development tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Unique identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Development phase this workflow belongs to
    pub phase: WorkflowPhase,
    /// Description of the workflow
    pub description: String,
    /// Steps in the workflow
    pub steps: Vec<WorkflowStep>,
    /// Variables available in the workflow
    pub variables: HashMap<String, String>,
    /// Checkpoint step IDs
    pub checkpoints: Vec<String>,
    /// Whether this is a built-in workflow
    #[serde(default)]
    pub builtin: bool,
}

/// Development phases for workflows
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowPhase {
    /// Analysis phase - understanding the problem
    Analysis,
    /// Planning phase - defining the solution
    Planning,
    /// Solutioning phase - designing the architecture
    Solutioning,
    /// Implementation phase - building the solution
    Implementation,
    /// Quick Flow - rapid development
    QuickFlow,
    /// Testing phase - validating the solution
    Testing,
    /// Documentation phase - documenting the solution
    Documentation,
    /// DevOps phase - deployment and operations
    DevOps,
}

impl std::fmt::Display for WorkflowPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowPhase::Analysis => write!(f, "Analysis"),
            WorkflowPhase::Planning => write!(f, "Planning"),
            WorkflowPhase::Solutioning => write!(f, "Solutioning"),
            WorkflowPhase::Implementation => write!(f, "Implementation"),
            WorkflowPhase::QuickFlow => write!(f, "Quick Flow"),
            WorkflowPhase::Testing => write!(f, "Testing"),
            WorkflowPhase::Documentation => write!(f, "Documentation"),
            WorkflowPhase::DevOps => write!(f, "DevOps"),
        }
    }
}

/// A step in a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// Unique identifier within the workflow
    pub id: String,
    /// Display name
    pub name: String,
    /// Description of what this step does
    pub description: String,
    /// Agent responsible for this step
    pub agent: String,
    /// Actions to perform in this step
    pub actions: Vec<String>,
    /// Condition for executing this step
    pub condition: Option<String>,
    /// Branches from this step
    pub branches: Vec<WorkflowBranch>,
    /// Whether this is a checkpoint step
    pub is_checkpoint: bool,
}

/// A branch in a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowBranch {
    /// Condition for taking this branch
    pub condition: String,
    /// ID of the next step
    pub next_step: String,
}

/// Progress tracking for a workflow session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowProgress {
    /// Session ID
    pub session_id: String,
    /// Workflow ID
    pub workflow_id: String,
    /// Current step ID
    pub current_step: String,
    /// Completed step IDs
    pub completed_steps: Vec<String>,
    /// Variable values
    pub variables: HashMap<String, String>,
    /// Started timestamp
    pub started_at: String,
    /// Last updated timestamp
    pub updated_at: String,
}

/// A workflow session
#[derive(Debug, Clone)]
pub struct WorkflowSession {
    /// Session ID
    pub id: String,
    /// The workflow being executed
    pub workflow: Workflow,
    /// Current progress
    pub progress: WorkflowProgress,
}

/// Result of executing a workflow step
#[derive(Debug, Clone)]
pub struct StepResult {
    /// Whether the step succeeded
    pub success: bool,
    /// Output from the step
    pub output: Option<String>,
    /// Next step ID (if branching)
    pub next_step: Option<String>,
    /// Whether to pause for user review
    pub pause_for_review: bool,
}

impl Workflow {
    /// Create a new workflow
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        phase: WorkflowPhase,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            phase,
            description: description.into(),
            steps: Vec::new(),
            variables: HashMap::new(),
            checkpoints: Vec::new(),
            builtin: false,
        }
    }

    /// Add a step to the workflow
    pub fn with_step(mut self, step: WorkflowStep) -> Self {
        if step.is_checkpoint {
            self.checkpoints.push(step.id.clone());
        }
        self.steps.push(step);
        self
    }

    /// Add a variable to the workflow
    pub fn with_variable(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.variables.insert(key.into(), value.into());
        self
    }

    /// Mark as built-in workflow
    pub fn as_builtin(mut self) -> Self {
        self.builtin = true;
        self
    }

    /// Get a step by ID
    pub fn get_step(&self, id: &str) -> Option<&WorkflowStep> {
        self.steps.iter().find(|s| s.id == id)
    }

    /// Get the first step
    pub fn first_step(&self) -> Option<&WorkflowStep> {
        self.steps.first()
    }

    /// Get the next step after the given step
    pub fn next_step(&self, current_id: &str) -> Option<&WorkflowStep> {
        let current_idx = self.steps.iter().position(|s| s.id == current_id)?;
        self.steps.get(current_idx + 1)
    }
}

impl WorkflowStep {
    /// Create a new workflow step
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        agent: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            agent: agent.into(),
            actions: Vec::new(),
            condition: None,
            branches: Vec::new(),
            is_checkpoint: false,
        }
    }

    /// Add an action to this step
    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.actions.push(action.into());
        self
    }

    /// Add multiple actions
    pub fn with_actions(mut self, actions: Vec<String>) -> Self {
        self.actions.extend(actions);
        self
    }

    /// Set a condition for this step
    pub fn with_condition(mut self, condition: impl Into<String>) -> Self {
        self.condition = Some(condition.into());
        self
    }

    /// Add a branch
    pub fn with_branch(
        mut self,
        condition: impl Into<String>,
        next_step: impl Into<String>,
    ) -> Self {
        self.branches.push(WorkflowBranch {
            condition: condition.into(),
            next_step: next_step.into(),
        });
        self
    }

    /// Mark as checkpoint
    pub fn as_checkpoint(mut self) -> Self {
        self.is_checkpoint = true;
        self
    }
}

impl WorkflowProgress {
    /// Create new progress for a workflow
    pub fn new(session_id: impl Into<String>, workflow: &Workflow) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            session_id: session_id.into(),
            workflow_id: workflow.id.clone(),
            current_step: workflow.first_step().map(|s| s.id.clone()).unwrap_or_default(),
            completed_steps: Vec::new(),
            variables: workflow.variables.clone(),
            started_at: now.clone(),
            updated_at: now,
        }
    }

    /// Mark a step as completed
    pub fn complete_step(&mut self, step_id: &str) {
        if !self.completed_steps.contains(&step_id.to_string()) {
            self.completed_steps.push(step_id.to_string());
        }
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    /// Set the current step
    pub fn set_current_step(&mut self, step_id: impl Into<String>) {
        self.current_step = step_id.into();
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    /// Set a variable value
    pub fn set_variable(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.variables.insert(key.into(), value.into());
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    /// Get a variable value
    pub fn get_variable(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }
}

impl StepResult {
    /// Create a successful step result
    pub fn success(output: Option<String>) -> Self {
        Self {
            success: true,
            output,
            next_step: None,
            pause_for_review: false,
        }
    }

    /// Create a failed step result
    pub fn failure(output: Option<String>) -> Self {
        Self {
            success: false,
            output,
            next_step: None,
            pause_for_review: false,
        }
    }

    /// Set the next step
    pub fn with_next_step(mut self, step_id: impl Into<String>) -> Self {
        self.next_step = Some(step_id.into());
        self
    }

    /// Mark as requiring user review
    pub fn with_pause(mut self) -> Self {
        self.pause_for_review = true;
        self
    }
}
