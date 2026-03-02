//! Parallel orchestration engine — executes media generation tasks concurrently.
//!
//! Builds a dependency graph from `OrchestratedRequest` tasks and runs
//! independent tasks in parallel while respecting dependencies.

use anyhow::Result;
use dx_core::{
    CostTracker, DxProviderRegistry, MediaGenerationRequest, MediaOutput,
    MediaType, MicroCost,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use crate::{OrchestratedRequest, OrchestratedTask, OrchestrationProgress, OrchestrationResult};

/// A task in the dependency graph with resolved dependencies.
#[derive(Debug)]
struct GraphNode {
    task: OrchestratedTask,
    dependents: Vec<String>,  // task IDs that depend on this node
}

/// Build a topological ordering from the orchestrated request.
fn topological_sort(tasks: &[OrchestratedTask]) -> Result<Vec<Vec<String>>> {
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut dependents: HashMap<String, Vec<String>> = HashMap::new();

    // Initialize
    for task in tasks {
        in_degree.entry(task.id.clone()).or_insert(0);
        for dep in &task.depends_on {
            dependents.entry(dep.clone()).or_default().push(task.id.clone());
            *in_degree.entry(task.id.clone()).or_insert(0) += 1;
        }
    }

    let mut levels: Vec<Vec<String>> = Vec::new();
    let mut queue: VecDeque<String> = VecDeque::new();

    // Start with tasks that have no dependencies
    for (id, &deg) in &in_degree {
        if deg == 0 {
            queue.push_back(id.clone());
        }
    }

    let mut visited = 0;
    while !queue.is_empty() {
        let level: Vec<String> = queue.drain(..).collect();
        for id in &level {
            visited += 1;
            if let Some(deps) = dependents.get(id) {
                for dep_id in deps {
                    if let Some(deg) = in_degree.get_mut(dep_id) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(dep_id.clone());
                        }
                    }
                }
            }
        }
        levels.push(level);
    }

    if visited != tasks.len() {
        anyhow::bail!("Circular dependency detected in orchestration tasks");
    }

    Ok(levels)
}

/// Execute an orchestrated request with parallel execution of independent tasks.
pub async fn execute_parallel(
    registry: &Arc<DxProviderRegistry>,
    request: &OrchestratedRequest,
    progress_cb: Option<Box<dyn Fn(OrchestrationProgress) + Send + Sync>>,
) -> Result<OrchestrationResult> {
    let levels = topological_sort(&request.tasks)?;

    let task_map: HashMap<String, &OrchestratedTask> =
        request.tasks.iter().map(|t| (t.id.clone(), t)).collect();

    let mut all_outputs: Vec<(String, MediaOutput)> = Vec::new();
    let mut all_errors: Vec<(String, String)> = Vec::new();
    let mut cost_tracker = CostTracker::new();
    let mut completed_count = 0;

    for level in &levels {
        // All tasks in this level can run in parallel
        let mut level_futures = Vec::new();

        for task_id in level {
            if let Some(task) = task_map.get(task_id) {
                let registry = registry.clone();
                let task_clone = (*task).clone();
                level_futures.push(async move {
                    execute_single_task(&registry, &task_clone).await
                });
            }
        }

        // Execute level in parallel using futures::join_all
        let results = futures::future::join_all(level_futures).await;

        for (i, result) in results.into_iter().enumerate() {
            let task_id = &level[i];
            match result {
                Ok(outputs) => {
                    for output in outputs {
                        cost_tracker.record(task_id, output.cost);
                        all_outputs.push((task_id.clone(), output));
                    }
                }
                Err(e) => {
                    all_errors.push((task_id.clone(), e.to_string()));
                }
            }
            completed_count += 1;

            if let Some(ref cb) = progress_cb {
                cb(OrchestrationProgress {
                    total_tasks: request.tasks.len(),
                    completed_tasks: completed_count,
                    current_task: Some(task_id.clone()),
                    total_cost: cost_tracker.total(),
                });
            }
        }
    }

    Ok(OrchestrationResult {
        outputs: all_outputs,
        cost_tracker,
        errors: all_errors,
    })
}

/// Execute a single media generation task.
async fn execute_single_task(
    registry: &DxProviderRegistry,
    task: &OrchestratedTask,
) -> Result<Vec<MediaOutput>> {
    let providers = registry.media_providers_for_type(task.media_type);

    if let Some(provider) = providers.first() {
        provider.generate(&task.request).await
    } else {
        anyhow::bail!("No provider available for {:?}", task.media_type)
    }
}

/// Estimate total cost for an orchestrated request without executing it.
pub fn estimate_cost(
    registry: &DxProviderRegistry,
    request: &OrchestratedRequest,
) -> MicroCost {
    let mut total = MicroCost::ZERO;

    for task in &request.tasks {
        let providers = registry.media_providers_for_type(task.media_type);
        if let Some(provider) = providers.first() {
            if let Some(cost) = provider.estimate_cost(&task.request) {
                total += cost;
            }
        }
    }

    total
}

/// Validate an orchestrated request for dependency correctness.
pub fn validate_request(request: &OrchestratedRequest) -> Result<()> {
    let task_ids: HashSet<&str> = request.tasks.iter().map(|t| t.id.as_str()).collect();

    for task in &request.tasks {
        for dep in &task.depends_on {
            if !task_ids.contains(dep.as_str()) {
                anyhow::bail!(
                    "Task '{}' depends on '{}' which does not exist",
                    task.id,
                    dep
                );
            }
        }
        if task.depends_on.contains(&task.id) {
            anyhow::bail!("Task '{}' depends on itself", task.id);
        }
    }

    // Check for cycles
    topological_sort(&request.tasks)?;

    Ok(())
}
