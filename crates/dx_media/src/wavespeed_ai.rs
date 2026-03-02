//! WaveSpeed AI — unified gateway for Kling 2.0, Seedance, and other video models.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaType, MicroCost,
};

/// WaveSpeed AI — single gateway to multiple video/image generation models.
pub struct WaveSpeedAiProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl WaveSpeedAiProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::image_providers::wavespeed_ai(),
            api_key: std::env::var("WAVESPEED_API_KEY").ok(),
        }
    }
}

impl Default for WaveSpeedAiProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl MediaProvider for WaveSpeedAiProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "WaveSpeed AI" }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Image, MediaType::Video]
    }

    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "kling-2.0".to_string(),
                name: "Kling 2.0 (via WaveSpeed)".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Video,
                pricing: None,
                supports_streaming: false,
                max_resolution: Some((1920, 1080)),
                max_duration_seconds: Some(10),
            },
            MediaModelInfo {
                id: "seedance".to_string(),
                name: "Seedance".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Video,
                pricing: None,
                supports_streaming: false,
                max_resolution: Some((1280, 720)),
                max_duration_seconds: Some(5),
            },
            MediaModelInfo {
                id: "flux-pro".to_string(),
                name: "Flux Pro".to_string(),
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
        Err(anyhow::anyhow!("WaveSpeed AI: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        // Free testing credits + pay-per-use
        None
    }
}
