//! Together.ai — fast inference for open models (Flux, SD, etc.).

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaType, MicroCost,
};

/// Together.ai — run open-source image models with fast inference.
pub struct TogetherAiProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl TogetherAiProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::image_providers::together_ai(),
            api_key: std::env::var("TOGETHER_API_KEY").ok(),
        }
    }
}

impl Default for TogetherAiProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl MediaProvider for TogetherAiProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Together AI" }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Image]
    }

    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "black-forest-labs/FLUX.1-schnell".to_string(),
                name: "FLUX.1 Schnell".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Image,
                pricing: None,
                supports_streaming: false,
                max_resolution: Some((1024, 1024)),
                max_duration_seconds: None,
            },
            MediaModelInfo {
                id: "stabilityai/stable-diffusion-xl-base-1.0".to_string(),
                name: "SDXL 1.0".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Image,
                pricing: None,
                supports_streaming: false,
                max_resolution: Some((1024, 1024)),
                max_duration_seconds: None,
            },
        ])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Together AI: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        // Free credits + pay-per-use ($0.003-0.01 per image)
        None
    }
}
