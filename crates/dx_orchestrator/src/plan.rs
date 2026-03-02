//! Generation plan — the DAG of tasks to execute.

use dx_core::cost::MicroCost;
use serde::{Deserialize, Serialize};

/// Status of a generation task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Waiting for dependencies.
    Pending,
    /// Dependencies met, ready to execute.
    Ready,
    /// Currently executing.
    Running,
    /// Successfully completed.
    Completed,
    /// Failed with an error.
    Failed,
    /// Skipped (e.g., budget exceeded).
    Skipped,
}

/// A single generation task within a plan.
#[derive(Debug, Clone)]
pub struct GenerationTask {
    /// Unique task ID.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Type of generation (text, image, video, audio, 3d, document).
    pub task_type: TaskType,
    /// The prompt/instructions for this specific task.
    pub prompt: String,
    /// Preferred provider ID (if any).
    pub preferred_provider: Option<String>,
    /// Estimated cost.
    pub estimated_cost: MicroCost,
    /// Current status.
    pub status: TaskStatus,
    /// Priority (higher = execute first).
    pub priority: u32,
}

/// Types of generation tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    /// LLM text generation (copy, description, analysis).
    Text,
    /// Image generation.
    Image,
    /// Video generation.
    Video,
    /// Audio/music generation.
    Audio,
    /// 3D model generation.
    ThreeD,
    /// Document assembly (PDF, DOCX, etc.).
    Document,
    /// TTS narration of generated text.
    Narration,
    /// Assembly step (combine multiple outputs).
    Assembly,
}

/// A dependency relationship between tasks.
#[derive(Debug, Clone)]
pub struct TaskDependency {
    /// Task that must complete first.
    pub from_task_id: String,
    /// Task that depends on the output.
    pub to_task_id: String,
    /// What data flows between them.
    pub data_flow: DataFlow,
}

/// Type of data flowing between tasks.
#[derive(Debug, Clone)]
pub enum DataFlow {
    /// Text output feeds into another task's prompt.
    TextToPrompt,
    /// Image output becomes input for another task.
    ImageInput,
    /// Audio output becomes input for assembly.
    AudioInput,
    /// All outputs assemble into a document.
    AssemblyInput,
}

/// A complete generation plan — a DAG of tasks with dependencies.
#[derive(Debug)]
pub struct GenerationPlan {
    /// All tasks in the plan.
    pub tasks: Vec<GenerationTask>,
    /// Dependencies between tasks.
    pub dependencies: Vec<TaskDependency>,
    /// Total estimated cost.
    pub estimated_total_cost: MicroCost,
}

impl GenerationPlan {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            dependencies: Vec::new(),
            estimated_total_cost: MicroCost::ZERO,
        }
    }

    /// Add a task to the plan.
    pub fn add_task(&mut self, task: GenerationTask) {
        self.estimated_total_cost = MicroCost(
            self.estimated_total_cost.0 + task.estimated_cost.0,
        );
        self.tasks.push(task);
    }

    /// Add a dependency between tasks.
    pub fn add_dependency(&mut self, dep: TaskDependency) {
        self.dependencies.push(dep);
    }

    /// Get tasks that are ready to execute (all dependencies met).
    pub fn ready_tasks(&self) -> Vec<&GenerationTask> {
        self.tasks
            .iter()
            .filter(|task| {
                if task.status != TaskStatus::Pending && task.status != TaskStatus::Ready {
                    return false;
                }
                // Check all dependencies are completed
                let deps: Vec<_> = self
                    .dependencies
                    .iter()
                    .filter(|d| d.to_task_id == task.id)
                    .collect();

                deps.iter().all(|dep| {
                    self.tasks
                        .iter()
                        .find(|t| t.id == dep.from_task_id)
                        .map_or(false, |t| t.status == TaskStatus::Completed)
                })
            })
            .collect()
    }

    /// Mark a task as completed.
    pub fn complete_task(&mut self, task_id: &str) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            task.status = TaskStatus::Completed;
        }
    }

    /// Mark a task as failed.
    pub fn fail_task(&mut self, task_id: &str) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            task.status = TaskStatus::Failed;
        }
    }

    /// Check if all tasks are completed (or failed/skipped).
    pub fn is_complete(&self) -> bool {
        self.tasks.iter().all(|t| {
            matches!(
                t.status,
                TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Skipped
            )
        })
    }
}

impl Default for GenerationPlan {
    fn default() -> Self {
        Self::new()
    }
}
