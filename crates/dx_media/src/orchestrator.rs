//! Multi-media orchestrator — the unified `generate()` call.
//!
//! Decomposes complex multi-media requests into parallel generation tasks:
//! LLM writes copy + image provider generates hero + 3D generates mockup +
//! music generates audio + Rust renders PDF. All in parallel.

use anyhow::Result;
use dx_core::{
    CostTracker, DxProviderRegistry, MediaGenerationRequest, MediaOutput,
    MediaType, MicroCost,
};
use std::sync::Arc;

/// A multi-media generation request that may involve multiple provider types.
#[derive(Debug, Clone)]
pub struct OrchestratedRequest {
    /// Human description of what to generate.
    pub description: String,
    /// Individual media generation sub-tasks.
    pub tasks: Vec<OrchestratedTask>,
}

/// A single sub-task within an orchestrated generation.
#[derive(Debug, Clone)]
pub struct OrchestratedTask {
    pub id: String,
    pub media_type: MediaType,
    pub request: MediaGenerationRequest,
    /// Dependencies — task IDs that must complete before this one.
    pub depends_on: Vec<String>,
}

/// Progress of the overall orchestrated generation.
#[derive(Debug, Clone)]
pub struct OrchestrationProgress {
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub current_task: Option<String>,
    pub total_cost: MicroCost,
}

/// Result of an orchestrated generation.
#[derive(Debug)]
pub struct OrchestrationResult {
    pub outputs: Vec<(String, MediaOutput)>,
    pub cost_tracker: CostTracker,
    pub errors: Vec<(String, String)>,
}

/// The multi-media orchestrator.
pub struct MediaOrchestrator {
    registry: Arc<DxProviderRegistry>,
}

impl MediaOrchestrator {
    pub fn new(registry: Arc<DxProviderRegistry>) -> Self {
        Self { registry }
    }

    /// Execute an orchestrated multi-media generation.
    ///
    /// Independent tasks run in parallel; dependent tasks wait for their prerequisites.
    pub async fn execute(&self, request: &OrchestratedRequest) -> Result<OrchestrationResult> {
        let mut outputs = Vec::new();
        let mut cost_tracker = CostTracker::new();
        let mut errors = Vec::new();

        // Simple sequential execution for now.
        // TODO: Implement parallel execution for independent tasks.
        for task in &request.tasks {
            let providers = self.registry.media_providers_for_type(task.media_type);

            if let Some(provider) = providers.first() {
                match provider.generate(&task.request).await {
                    Ok(media_outputs) => {
                        for output in media_outputs {
                            cost_tracker.record(&provider.id().to_string(), output.cost);
                            outputs.push((task.id.clone(), output));
                        }
                    }
                    Err(e) => {
                        errors.push((task.id.clone(), e.to_string()));
                    }
                }
            } else {
                errors.push((
                    task.id.clone(),
                    format!("No available provider for {:?}", task.media_type),
                ));
            }
        }

        Ok(OrchestrationResult {
            outputs,
            cost_tracker,
            errors,
        })
    }
}
