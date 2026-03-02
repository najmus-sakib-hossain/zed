//! AIMLAPI.com — unified API for 200+ AI models including image generation.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaType, MicroCost,
};

/// AIML API — access to Flux, GPT-Image, and 200+ other models.
pub struct AimlApiProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl AimlApiProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::image_providers::aiml_api(),
            api_key: std::env::var("AIMLAPI_KEY").ok(),
        }
    }
}

impl Default for AimlApiProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl MediaProvider for AimlApiProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "AIML API" }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Image, MediaType::Audio]
    }

    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "flux-1.1".to_string(),
                name: "Flux 1.1".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Image,
                pricing: None,
                supports_streaming: false,
                max_resolution: Some((1024, 1024)),
                max_duration_seconds: None,
            },
            MediaModelInfo {
                id: "gpt-image-mini".to_string(),
                name: "GPT-Image Mini".to_string(),
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
        Err(anyhow::anyhow!("AIML API: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        // Explicit free image generation tier
        None
    }
}
