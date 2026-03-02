//! dx_orchestrator — Unified multi-media generation orchestrator.
//!
//! Handles complex requests like:
//! > "Generate a product landing page PDF with a hero image, 3D mockup, and background music"
//!
//! The orchestrator:
//! 1. Decomposes the request into sub-tasks (LLM copy, image, 3D, audio, PDF)
//! 2. Executes independent sub-tasks in parallel
//! 3. Assembles final output from all sub-task results
//! 4. Tracks unified cost across all providers used
//! 5. Reports progress for each concurrent task

pub mod cost_summary;
pub mod decomposer;
pub mod executor;
pub mod plan;

pub use cost_summary::CostSummary;
pub use decomposer::RequestDecomposer;
pub use executor::ParallelExecutor;
pub use plan::{GenerationPlan, GenerationTask, TaskDependency, TaskStatus};

use dx_core::cost::MicroCost;

/// A unified generation request.
#[derive(Debug, Clone)]
pub struct GenerateRequest {
    /// The user's natural language request.
    pub prompt: String,
    /// Optional explicit task hints (e.g., "image", "video", "pdf").
    pub task_hints: Vec<String>,
    /// Maximum budget for the entire request.
    pub max_budget: Option<MicroCost>,
    /// Whether to prefer local/free providers.
    pub prefer_local: bool,
}

/// Result of a unified generation request.
#[derive(Debug)]
pub struct GenerateResult {
    /// All generated outputs.
    pub outputs: Vec<GeneratedOutput>,
    /// Total cost across all providers.
    pub total_cost: MicroCost,
    /// Summary of costs per provider.
    pub cost_summary: CostSummary,
    /// How long the entire request took.
    pub total_duration_secs: f64,
}

/// A single generated output from the orchestrator.
#[derive(Debug)]
pub struct GeneratedOutput {
    /// The task that produced this output.
    pub task_type: String,
    /// The provider used.
    pub provider_id: String,
    /// Output data (varies by task type).
    pub data: OutputData,
    /// Cost for this specific output.
    pub cost: MicroCost,
}

/// Output data variants.
#[derive(Debug)]
pub enum OutputData {
    /// Text content (LLM output).
    Text(String),
    /// Image bytes (PNG/JPEG).
    Image(Vec<u8>),
    /// Video bytes (MP4).
    Video(Vec<u8>),
    /// Audio bytes (MP3/WAV).
    Audio(Vec<u8>),
    /// 3D model bytes (glTF/OBJ).
    ThreeD(Vec<u8>),
    /// Document bytes (PDF).
    Document(Vec<u8>),
    /// Assembled multi-format output.
    Composite(Vec<(String, Vec<u8>)>),
}

/// Initialize the orchestrator subsystem.
pub fn init() {
    log::info!("DX Orchestrator initialized");
}

/// The main entry point — decompose, execute, and assemble a generation request.
pub async fn generate(request: GenerateRequest) -> anyhow::Result<GenerateResult> {
    log::info!("Orchestrating generation: {:?}", request.prompt);

    // Step 1: Decompose the request into sub-tasks
    let decomposer = RequestDecomposer::new();
    let plan = decomposer.decompose(&request)?;
    log::info!(
        "Decomposed into {} tasks with {} dependencies",
        plan.tasks.len(),
        plan.dependencies.len()
    );

    // Step 2: Execute tasks in parallel where possible
    let executor = ParallelExecutor::new();
    let outputs = executor.execute(&plan).await?;

    // Step 3: Calculate total cost
    let total_cost = outputs.iter().map(|o| o.cost).fold(MicroCost::ZERO, |a, b| {
        MicroCost(a.0 + b.0)
    });

    let cost_summary = CostSummary::from_outputs(&outputs);

    Ok(GenerateResult {
        outputs,
        total_cost,
        cost_summary,
        total_duration_secs: 0.0, // would be measured in real implementation
    })
}
