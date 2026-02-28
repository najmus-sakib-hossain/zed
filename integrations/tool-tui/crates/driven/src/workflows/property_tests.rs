//! Property-based tests for the Workflow system
//!
//! Tests Properties 20-23 from the design document.

use super::{Workflow, WorkflowEngine, WorkflowPhase, WorkflowProgress, WorkflowStep};
use proptest::prelude::*;

/// Generate a valid workflow ID
fn arb_workflow_id() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9-]{2,20}".prop_filter("no consecutive hyphens", |s| !s.contains("--"))
}

/// Generate a valid workflow name
fn arb_workflow_name() -> impl Strategy<Value = String> {
    "[A-Z][a-zA-Z ]{2,30}".prop_filter("no double spaces", |s| !s.contains("  "))
}

/// Generate a workflow phase
fn arb_phase() -> impl Strategy<Value = WorkflowPhase> {
    prop_oneof![
        Just(WorkflowPhase::Analysis),
        Just(WorkflowPhase::Planning),
        Just(WorkflowPhase::Solutioning),
        Just(WorkflowPhase::Implementation),
        Just(WorkflowPhase::QuickFlow),
        Just(WorkflowPhase::Testing),
        Just(WorkflowPhase::Documentation),
        Just(WorkflowPhase::DevOps),
    ]
}

/// Generate a valid step ID
fn arb_step_id() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9-]{2,15}"
}

/// Generate a valid agent ID
fn arb_agent_id() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("pm".to_string()),
        Just("architect".to_string()),
        Just("developer".to_string()),
        Just("analyst".to_string()),
        Just("test-architect".to_string()),
    ]
}

/// Generate a workflow step
fn arb_step(id: String) -> impl Strategy<Value = WorkflowStep> {
    (
        arb_workflow_name(),       // name
        "[A-Z][a-zA-Z .,]{10,50}", // description
        arb_agent_id(),
        prop::collection::vec("[A-Z][a-zA-Z ]{5,30}", 1..4), // actions
        prop::bool::ANY,                                     // is_checkpoint
    )
        .prop_map(move |(name, desc, agent, actions, is_checkpoint)| {
            let mut step = WorkflowStep::new(id.clone(), name, desc, agent).with_actions(actions);
            if is_checkpoint {
                step = step.as_checkpoint();
            }
            step
        })
}

/// Generate a workflow with steps
fn arb_workflow() -> impl Strategy<Value = Workflow> {
    (
        arb_workflow_id(),
        arb_workflow_name(),
        arb_phase(),
        "[A-Z][a-zA-Z .,]{10,100}",                 // description
        prop::collection::vec(arb_step_id(), 2..5), // step IDs
    )
        .prop_flat_map(|(id, name, phase, desc, step_ids)| {
            let steps_strategy =
                step_ids.into_iter().map(|step_id| arb_step(step_id)).collect::<Vec<_>>();

            (Just(id), Just(name), Just(phase), Just(desc), steps_strategy)
        })
        .prop_map(|(id, name, phase, desc, steps)| {
            let mut workflow = Workflow::new(id, name, phase, desc);
            for step in steps {
                workflow = workflow.with_step(step);
            }
            workflow
        })
}

/// Generate a variable name
fn arb_var_name() -> impl Strategy<Value = String> {
    "[a-z][a-z_]{2,15}"
}

/// Generate a variable value
fn arb_var_value() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 ]{1,30}"
}

proptest! {
    /// Property 20: Workflow Step Execution
    /// For any workflow, steps SHALL execute in order, respecting conditions and checkpoints.
    #[test]
    fn prop_workflow_step_execution(workflow in arb_workflow()) {
        // Verify steps are in order
        let step_ids: Vec<&str> = workflow.steps.iter().map(|s| s.id.as_str()).collect();

        // First step should be accessible
        let first = workflow.first_step();
        prop_assert!(first.is_some(), "Workflow should have a first step");
        prop_assert_eq!(&first.unwrap().id, &step_ids[0]);

        // Each step should have a next step (except the last)
        for i in 0..step_ids.len() - 1 {
            let next = workflow.next_step(step_ids[i]);
            prop_assert!(next.is_some(), "Step {} should have a next step", step_ids[i]);
            prop_assert_eq!(&next.unwrap().id, &step_ids[i + 1]);
        }

        // Last step should have no next
        let last_next = workflow.next_step(step_ids[step_ids.len() - 1]);
        prop_assert!(last_next.is_none(), "Last step should have no next step");
    }

    /// Property 22: Workflow Variable Substitution
    /// For any workflow template with variables, all `{variable}` placeholders
    /// SHALL be replaced with their values.
    #[test]
    fn prop_workflow_variable_substitution(
        workflow in arb_workflow(),
        vars in prop::collection::hash_map(arb_var_name(), arb_var_value(), 1..5),
    ) {
        let engine = WorkflowEngine::new();

        // Create workflow with variables
        let mut workflow_with_vars = workflow.clone();
        for (k, v) in &vars {
            workflow_with_vars = workflow_with_vars.with_variable(k.clone(), v.clone());
        }

        let progress = WorkflowProgress::new("test-session", &workflow_with_vars);

        // Create a template with all variables
        let template: String = vars.keys()
            .map(|k| format!("{{{}}}", k))
            .collect::<Vec<_>>()
            .join(" ");

        let result = engine.substitute_variables(&template, &progress);

        // All placeholders should be replaced
        for (key, value) in &vars {
            prop_assert!(!result.contains(&format!("{{{}}}", key)),
                "Placeholder {{{}}} should be replaced", key);
            prop_assert!(result.contains(value),
                "Value '{}' should be in result", value);
        }
    }

    /// Property 23: Workflow Progress Persistence
    /// For any workflow session, progress SHALL be persisted and resumable.
    #[test]
    fn prop_workflow_progress_persistence(workflow in arb_workflow()) {
        // Create progress
        let mut progress = WorkflowProgress::new("test-session", &workflow);

        // Complete some steps
        if let Some(first) = workflow.first_step() {
            progress.complete_step(&first.id);

            if let Some(second) = workflow.next_step(&first.id) {
                progress.set_current_step(&second.id);
            }
        }

        // Serialize to JSON
        let json = serde_json::to_string(&progress).expect("Failed to serialize progress");

        // Deserialize back
        let loaded: WorkflowProgress = serde_json::from_str(&json).expect("Failed to deserialize progress");

        // Verify all fields are preserved
        prop_assert_eq!(&progress.session_id, &loaded.session_id);
        prop_assert_eq!(&progress.workflow_id, &loaded.workflow_id);
        prop_assert_eq!(&progress.current_step, &loaded.current_step);
        prop_assert_eq!(&progress.completed_steps, &loaded.completed_steps);
        prop_assert_eq!(&progress.variables, &loaded.variables);
    }

    /// Property: Workflow checkpoints are tracked correctly
    #[test]
    fn prop_workflow_checkpoints_tracked(workflow in arb_workflow()) {
        // Count checkpoint steps
        let checkpoint_count = workflow.steps.iter().filter(|s| s.is_checkpoint).count();

        // Verify checkpoints list matches
        prop_assert_eq!(workflow.checkpoints.len(), checkpoint_count);

        // All checkpoint IDs should be in the steps
        for checkpoint_id in &workflow.checkpoints {
            let step = workflow.get_step(checkpoint_id);
            prop_assert!(step.is_some(), "Checkpoint {} should exist as a step", checkpoint_id);
            prop_assert!(step.unwrap().is_checkpoint, "Step {} should be marked as checkpoint", checkpoint_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_yaml_round_trip() {
        let workflow = Workflow::new(
            "test-workflow",
            "Test Workflow",
            WorkflowPhase::Analysis,
            "A test workflow",
        )
        .with_step(
            WorkflowStep::new("step-1", "Step 1", "First step", "analyst")
                .with_action("Do something".to_string()),
        )
        .with_step(
            WorkflowStep::new("step-2", "Step 2", "Second step", "developer")
                .with_action("Do something else".to_string())
                .as_checkpoint(),
        )
        .with_variable("project", "my-project");

        let yaml = serde_yaml::to_string(&workflow).unwrap();
        let loaded: Workflow = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(workflow.id, loaded.id);
        assert_eq!(workflow.name, loaded.name);
        assert_eq!(workflow.steps.len(), loaded.steps.len());
        assert_eq!(workflow.checkpoints.len(), loaded.checkpoints.len());
    }

    #[test]
    fn test_builtin_workflows_count() {
        let workflows = super::super::builtin::all_workflows();
        assert!(workflows.len() >= 30, "Expected at least 30 workflows, got {}", workflows.len());
    }

    #[test]
    fn test_workflow_engine_start() {
        let mut engine = WorkflowEngine::new();
        engine.load_builtin();

        let session = engine.start("quick-bug-fix");
        assert!(session.is_ok());

        let session = session.unwrap();
        assert!(!session.id.is_empty());
        assert_eq!(session.workflow.id, "quick-bug-fix");
        assert!(!session.progress.current_step.is_empty());
    }
}
