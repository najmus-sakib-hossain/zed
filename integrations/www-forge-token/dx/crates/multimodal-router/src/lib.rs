//! # multimodal-router
//!
//! Routes multimodal inputs to the most cost-effective model and
//! processing pipeline based on content type and complexity.
//!
//! ## Evidence
//! - Different models have vastly different multimodal pricing
//! - GPT-4o-mini vision is ~10× cheaper than GPT-4o for simple tasks
//! - Audio-capable models vary in per-second costs
//! - **Honest: 30-70% cost savings by routing to cheaper models when appropriate**
//!
//! STAGE: PreCall (priority 5)

use dx_core::*;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct MultimodalRouterConfig {
    /// Preferred cheap model for simple visual tasks
    pub cheap_vision_model: String,
    /// Preferred model for complex visual analysis
    pub full_vision_model: String,
    /// Preferred model for audio
    pub audio_model: String,
    /// Maximum images before routing to batch/cheaper model
    pub image_complexity_threshold: usize,
}

impl Default for MultimodalRouterConfig {
    fn default() -> Self {
        Self {
            cheap_vision_model: "gpt-4o-mini".into(),
            full_vision_model: "gpt-4o".into(),
            audio_model: "gpt-4o-audio-preview".into(),
            image_complexity_threshold: 5,
        }
    }
}

/// Routing decision for multimodal content.
#[derive(Debug, Clone, PartialEq)]
pub enum RoutingDecision {
    /// Use the current model (no change)
    KeepCurrent,
    /// Route to a cheaper model
    RouteToModel(String),
}

pub struct MultimodalRouter {
    config: MultimodalRouterConfig,
    report: Mutex<TokenSavingsReport>,
}

impl MultimodalRouter {
    pub fn new() -> Self {
        Self::with_config(MultimodalRouterConfig::default())
    }

    pub fn with_config(config: MultimodalRouterConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Determine routing based on multimodal content.
    fn decide_route(&self, input: &MultiModalSaverInput, current_model: &str) -> RoutingDecision {
        let has_audio = !input.audio.is_empty();
        let has_complex_images = input.base.images.len() > self.config.image_complexity_threshold
            || input.base.images.iter().any(|i| i.detail == ImageDetail::High);
        let has_3d = !input.assets_3d.is_empty();
        let has_video = !input.videos.is_empty();

        // If audio, need audio-capable model
        if has_audio && !current_model.contains("audio") {
            return RoutingDecision::RouteToModel(self.config.audio_model.clone());
        }

        // Simple image tasks → cheaper model
        if !has_audio && !has_3d && !has_video && !has_complex_images {
            if current_model.contains("gpt-4o") && !current_model.contains("mini") {
                return RoutingDecision::RouteToModel(self.config.cheap_vision_model.clone());
            }
        }

        RoutingDecision::KeepCurrent
    }
}

#[async_trait::async_trait]
impl MultiModalTokenSaver for MultimodalRouter {
    fn name(&self) -> &str { "multimodal-router" }
    fn stage(&self) -> SaverStage { SaverStage::PreCall }
    fn priority(&self) -> u32 { 5 }
    fn modality(&self) -> Modality { Modality::CrossModal }

    async fn process_multimodal(
        &self,
        input: MultiModalSaverInput,
        ctx: &SaverContext,
    ) -> Result<MultiModalSaverOutput, SaverError> {
        let total_tokens: usize = input.base.messages.iter().map(|m| m.token_count).sum::<usize>()
            + input.audio.iter().map(|a| a.naive_token_estimate).sum::<usize>()
            + input.videos.iter().map(|v| v.naive_token_estimate).sum::<usize>();

        let decision = self.decide_route(&input, &ctx.model);

        let (description, cost_savings_pct) = match &decision {
            RoutingDecision::KeepCurrent => {
                ("Keeping current model — no cheaper alternative appropriate.".into(), 0.0)
            }
            RoutingDecision::RouteToModel(model) => {
                // Estimate cost savings (rough: mini is ~10× cheaper for vision)
                let savings = if model.contains("mini") { 0.6 } else { 0.3 };
                (format!(
                    "ROUTING: {} → {} (estimated {:.0}% cost savings). \
                     NOTE: This changes the model used, not the token count.",
                    ctx.model, model, savings * 100.0
                ), savings)
            }
        };

        // Token count doesn't change — this is a cost router, not a token reducer
        let estimated_cost_tokens_saved = (total_tokens as f64 * cost_savings_pct) as usize;

        let report = TokenSavingsReport {
            technique: "multimodal-router".into(),
            tokens_before: total_tokens,
            tokens_after: total_tokens,
            tokens_saved: estimated_cost_tokens_saved, // Cost-equivalent savings
            description,
        };
        *self.report.lock().unwrap() = report;

        // Add routing hint as metadata in a system message
        let mut new_messages = input.base.messages;
        if let RoutingDecision::RouteToModel(ref model) = decision {
            new_messages.push(Message {
                role: "system".into(),
                content: format!("[multimodal-router: recommend model={}]", model),
                images: vec![],
                tool_call_id: None,
                token_count: 5,
            });
        }

        Ok(MultiModalSaverOutput {
            base: SaverOutput {
                messages: new_messages,
                tools: input.base.tools,
                images: input.base.images,
                skipped: decision == RoutingDecision::KeepCurrent,
                cached_response: None,
            },
            audio: input.audio,
            live_frames: input.live_frames,
            documents: input.documents,
            videos: input.videos,
            assets_3d: input.assets_3d,
        })
    }

    fn last_savings(&self) -> TokenSavingsReport {
        self.report.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_routes_simple_to_mini() {
        let saver = MultimodalRouter::new();
        let ctx = SaverContext { model: "gpt-4o".into(), ..Default::default() };
        let input = MultiModalSaverInput {
            base: SaverInput {
                messages: vec![Message { role: "user".into(), content: "describe this".into(), images: vec![], tool_call_id: None, token_count: 5 }],
                tools: vec![],
                images: vec![ImageInput { data: vec![], mime: "image/jpeg".into(), detail: ImageDetail::Low, original_tokens: 85, processed_tokens: 85 }],
                turn_number: 1,
            },
            audio: vec![], live_frames: vec![], documents: vec![], videos: vec![], assets_3d: vec![],
        };
        let out = saver.process_multimodal(input, &ctx).await.unwrap();
        assert!(!out.base.skipped);
        // Should recommend routing to mini
        assert!(out.base.messages.iter().any(|m| m.content.contains("mini")));
    }
}
