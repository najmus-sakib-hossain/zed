//! Parallel executor — runs independent tasks concurrently.

use crate::plan::{GenerationPlan, TaskStatus};
use crate::{GeneratedOutput, OutputData};
use anyhow::Result;
use dx_core::cost::MicroCost;

/// Executes a generation plan, running independent tasks in parallel.
pub struct ParallelExecutor {
    /// Maximum number of concurrent tasks.
    max_concurrency: usize,
}

impl ParallelExecutor {
    pub fn new() -> Self {
        Self {
            max_concurrency: 4,
        }
    }

    /// Set the maximum number of concurrent tasks.
    pub fn with_concurrency(mut self, max: usize) -> Self {
        self.max_concurrency = max;
        self
    }

    /// Execute all tasks in the plan, respecting dependencies.
    ///
    /// Tasks without dependencies are executed in parallel.
    /// Tasks with dependencies wait until their dependencies complete.
    pub async fn execute(&self, plan: &GenerationPlan) -> Result<Vec<GeneratedOutput>> {
        let mut outputs = Vec::new();
        let mut plan_state = plan
            .tasks
            .iter()
            .map(|t| (t.id.clone(), t.status))
            .collect::<std::collections::HashMap<_, _>>();

        // Simple topological execution: process ready tasks in rounds
        loop {
            // Find tasks that are ready to execute
            let ready: Vec<_> = plan
                .tasks
                .iter()
                .filter(|task| {
                    let status = plan_state.get(&task.id).copied().unwrap_or(TaskStatus::Pending);
                    if status != TaskStatus::Pending {
                        return false;
                    }

                    // Check all dependencies are completed
                    plan.dependencies
                        .iter()
                        .filter(|d| d.to_task_id == task.id)
                        .all(|dep| {
                            plan_state
                                .get(&dep.from_task_id)
                                .copied()
                                .unwrap_or(TaskStatus::Pending)
                                == TaskStatus::Completed
                        })
                })
                .take(self.max_concurrency)
                .collect();

            if ready.is_empty() {
                break;
            }

            // Execute ready tasks (in a real impl, these would be spawned concurrently)
            for task in &ready {
                log::info!(
                    "Executing task '{}' ({:?})",
                    task.name,
                    task.task_type
                );

                plan_state.insert(task.id.clone(), TaskStatus::Running);

                // Placeholder execution — in real implementation, this would
                // dispatch to the appropriate provider (LLM, image, video, etc.)
                let output = GeneratedOutput {
                    task_type: format!("{:?}", task.task_type),
                    provider_id: task
                        .preferred_provider
                        .clone()
                        .unwrap_or_else(|| "auto".to_string()),
                    data: match task.task_type {
                        crate::plan::TaskType::Text => {
                            OutputData::Text(format!("Generated text for: {}", task.prompt))
                        }
                        crate::plan::TaskType::Image => OutputData::Image(Vec::new()),
                        crate::plan::TaskType::Video => OutputData::Video(Vec::new()),
                        crate::plan::TaskType::Audio => OutputData::Audio(Vec::new()),
                        crate::plan::TaskType::ThreeD => OutputData::ThreeD(Vec::new()),
                        crate::plan::TaskType::Document => OutputData::Document(Vec::new()),
                        crate::plan::TaskType::Narration => OutputData::Audio(Vec::new()),
                        crate::plan::TaskType::Assembly => OutputData::Composite(Vec::new()),
                    },
                    cost: task.estimated_cost,
                };

                outputs.push(output);
                plan_state.insert(task.id.clone(), TaskStatus::Completed);

                log::info!("Task '{}' completed", task.name);
            }
        }

        // Check for failed/skipped tasks
        let incomplete: Vec<_> = plan_state
            .iter()
            .filter(|(_, status)| **status == TaskStatus::Pending)
            .map(|(id, _)| id.clone())
            .collect();

        if !incomplete.is_empty() {
            log::warn!(
                "Orchestrator: {} tasks could not be executed (dependency cycle or failure): {:?}",
                incomplete.len(),
                incomplete
            );
        }

        Ok(outputs)
    }
}

impl Default for ParallelExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Progress callback for tracking task execution.
pub trait ProgressCallback: Send + Sync {
    fn on_task_start(&self, task_id: &str, task_name: &str);
    fn on_task_progress(&self, task_id: &str, progress: f32);
    fn on_task_complete(&self, task_id: &str, cost: MicroCost);
    fn on_task_failed(&self, task_id: &str, error: &str);
    fn on_all_complete(&self, total_cost: MicroCost);
}
