//! Request decomposer — breaks down natural language generation requests
//! into a plan of concrete sub-tasks.

use crate::plan::{DataFlow, GenerationPlan, GenerationTask, TaskDependency, TaskStatus, TaskType};
use crate::GenerateRequest;
use anyhow::Result;
use dx_core::cost::MicroCost;

/// Decomposes a complex generation request into a task plan.
pub struct RequestDecomposer;

impl RequestDecomposer {
    pub fn new() -> Self {
        Self
    }

    /// Decompose a generation request into a plan of tasks.
    ///
    /// Uses keyword analysis and task hints to determine what sub-tasks
    /// are needed. In a full implementation, this would use an LLM to
    /// understand the request and plan the pipeline.
    pub fn decompose(&self, request: &GenerateRequest) -> Result<GenerationPlan> {
        let mut plan = GenerationPlan::new();
        let prompt_lower = request.prompt.to_lowercase();

        // Detect needed task types from prompt keywords and hints
        let needs_text = true; // Always need text generation for copy/descriptions
        let needs_image = prompt_lower.contains("image")
            || prompt_lower.contains("picture")
            || prompt_lower.contains("hero")
            || prompt_lower.contains("photo")
            || prompt_lower.contains("illustration")
            || request.task_hints.iter().any(|h| h == "image");
        let needs_video = prompt_lower.contains("video")
            || prompt_lower.contains("animation")
            || request.task_hints.iter().any(|h| h == "video");
        let needs_audio = prompt_lower.contains("music")
            || prompt_lower.contains("audio")
            || prompt_lower.contains("sound")
            || prompt_lower.contains("background music")
            || request.task_hints.iter().any(|h| h == "audio");
        let needs_3d = prompt_lower.contains("3d")
            || prompt_lower.contains("mockup")
            || prompt_lower.contains("model")
            || request.task_hints.iter().any(|h| h == "3d");
        let needs_document = prompt_lower.contains("pdf")
            || prompt_lower.contains("document")
            || prompt_lower.contains("report")
            || prompt_lower.contains("landing page")
            || request.task_hints.iter().any(|h| h == "pdf");
        let needs_narration = prompt_lower.contains("narration")
            || prompt_lower.contains("read")
            || prompt_lower.contains("speak");

        // Create tasks based on detected needs
        let mut task_counter = 0;
        let mut next_id = || {
            task_counter += 1;
            format!("task_{}", task_counter)
        };

        // Text generation is always first (provides copy for other tasks)
        let text_task_id = if needs_text {
            let id = next_id();
            plan.add_task(GenerationTask {
                id: id.clone(),
                name: "Generate text content".to_string(),
                task_type: TaskType::Text,
                prompt: format!(
                    "Generate the text content for: {}",
                    request.prompt
                ),
                preferred_provider: None,
                estimated_cost: MicroCost(500), // ~$0.005 for text gen
                status: TaskStatus::Pending,
                priority: 100,
            });
            Some(id)
        } else {
            None
        };

        // Image generation
        if needs_image {
            let id = next_id();
            if let Some(text_id) = &text_task_id {
                plan.add_dependency(TaskDependency {
                    from_task_id: text_id.clone(),
                    to_task_id: id.clone(),
                    data_flow: DataFlow::TextToPrompt,
                });
            }
            plan.add_task(GenerationTask {
                id,
                name: "Generate image".to_string(),
                task_type: TaskType::Image,
                prompt: format!("Create an image for: {}", request.prompt),
                preferred_provider: None,
                estimated_cost: MicroCost(4000), // ~$0.04 for DALL-E
                status: TaskStatus::Pending,
                priority: 80,
            });
        }

        // Video generation
        if needs_video {
            let id = next_id();
            plan.add_task(GenerationTask {
                id,
                name: "Generate video".to_string(),
                task_type: TaskType::Video,
                prompt: format!("Create a video for: {}", request.prompt),
                preferred_provider: None,
                estimated_cost: MicroCost(50000), // ~$0.50 for video
                status: TaskStatus::Pending,
                priority: 60,
            });
        }

        // Audio/music generation
        if needs_audio {
            let id = next_id();
            plan.add_task(GenerationTask {
                id,
                name: "Generate audio/music".to_string(),
                task_type: TaskType::Audio,
                prompt: format!("Create audio for: {}", request.prompt),
                preferred_provider: None,
                estimated_cost: MicroCost(10000), // ~$0.10 for music
                status: TaskStatus::Pending,
                priority: 70,
            });
        }

        // 3D model generation
        if needs_3d {
            let id = next_id();
            plan.add_task(GenerationTask {
                id,
                name: "Generate 3D model".to_string(),
                task_type: TaskType::ThreeD,
                prompt: format!("Create a 3D model for: {}", request.prompt),
                preferred_provider: None,
                estimated_cost: MicroCost(20000), // ~$0.20 for 3D
                status: TaskStatus::Pending,
                priority: 50,
            });
        }

        // Narration (TTS of the generated text)
        if needs_narration {
            let id = next_id();
            if let Some(text_id) = &text_task_id {
                plan.add_dependency(TaskDependency {
                    from_task_id: text_id.clone(),
                    to_task_id: id.clone(),
                    data_flow: DataFlow::TextToPrompt,
                });
            }
            plan.add_task(GenerationTask {
                id,
                name: "Generate narration".to_string(),
                task_type: TaskType::Narration,
                prompt: "Read the generated text aloud".to_string(),
                preferred_provider: None,
                estimated_cost: MicroCost(2000), // ~$0.02 for TTS
                status: TaskStatus::Pending,
                priority: 30,
            });
        }

        // Document assembly (depends on all other outputs)
        if needs_document {
            let assembly_id = next_id();
            // Assembly depends on all previous tasks
            for task in &plan.tasks {
                plan.dependencies.push(TaskDependency {
                    from_task_id: task.id.clone(),
                    to_task_id: assembly_id.clone(),
                    data_flow: DataFlow::AssemblyInput,
                });
            }
            plan.add_task(GenerationTask {
                id: assembly_id,
                name: "Assemble document".to_string(),
                task_type: TaskType::Document,
                prompt: format!("Assemble all outputs into: {}", request.prompt),
                preferred_provider: None,
                estimated_cost: MicroCost(0), // Local rendering is free
                status: TaskStatus::Pending,
                priority: 10,
            });
        }

        log::info!(
            "Decomposed request into {} tasks, estimated total: ${:.4}",
            plan.tasks.len(),
            plan.estimated_total_cost.as_usd()
        );

        Ok(plan)
    }
}

impl Default for RequestDecomposer {
    fn default() -> Self {
        Self::new()
    }
}
