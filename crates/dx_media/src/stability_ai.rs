//! Stability AI media provider — SDXL, SD3.5, Stable Video Diffusion.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider,
    MediaProviderId, MediaProviderLocation, MediaType, MicroCost,
};

/// Stability AI configuration.
#[derive(Debug, Clone)]
pub struct StabilityAiConfig {
    pub api_key: String,
    pub base_url: String,
}

impl Default for StabilityAiConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.stability.ai/v2beta".into(),
        }
    }
}

/// Stability AI provider — open-source image & video generation.
pub struct StabilityAiProvider {
    id: MediaProviderId,
    config: StabilityAiConfig,
}

impl StabilityAiProvider {
    pub fn new(config: StabilityAiConfig) -> Self {
        Self {
            id: dx_core::image_providers::stability_ai(),
            config,
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for StabilityAiProvider {
    fn id(&self) -> &MediaProviderId {
        &self.id
    }

    fn name(&self) -> &str {
        "Stability AI"
    }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Image, MediaType::Video]
    }

    fn location(&self) -> MediaProviderLocation {
        MediaProviderLocation::Cloud
    }

    fn is_available(&self) -> bool {
        !self.config.api_key.is_empty()
    }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "stable-diffusion-xl-1024-v1-0".into(),
                name: "Stable Diffusion XL".into(),
                provider_id: self.id.clone(),
                media_type: MediaType::Image,
                pricing: None,
                supports_streaming: false,
                max_resolution: Some((1024, 1024)),
                max_duration_seconds: None,
            },
            MediaModelInfo {
                id: "sd3.5-large".into(),
                name: "Stable Diffusion 3.5 Large".into(),
                provider_id: self.id.clone(),
                media_type: MediaType::Image,
                pricing: None,
                supports_streaming: false,
                max_resolution: Some((1536, 1536)),
                max_duration_seconds: None,
            },
        ])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        log::info!("Stability AI generate for model={}", request.model);

        Ok(vec![MediaOutput {
            media_type: request.media_type,
            data: Vec::new(),
            mime_type: "image/png".into(),
            extension: "png".into(),
            cost: MicroCost::from_dollars(0.03),
            saved_path: None,
            dimensions: request.dimensions,
            duration_seconds: None,
            seed: request.seed,
        }])
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        Some(MicroCost::from_dollars(0.03))
    }
}
