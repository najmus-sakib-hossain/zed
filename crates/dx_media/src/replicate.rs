//! Replicate adapter — access 200+ community media models.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaType, MicroCost,
};

/// Replicate — run 200+ open-source media generation models in the cloud.
pub struct ReplicateProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl ReplicateProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::image_providers::replicate(),
            api_key: std::env::var("REPLICATE_API_TOKEN").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for ReplicateProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Replicate" }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Image, MediaType::Video, MediaType::Audio, MediaType::Music, MediaType::ThreeD]
    }

    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        // Replicate has 200+ models — return notable ones
        Ok(vec![
            MediaModelInfo {
                id: "stability-ai/sdxl".to_string(),
                name: "SDXL (via Replicate)".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Image,
                pricing: None,
                supports_streaming: false,
                max_resolution: Some((1024, 1024)),
                max_duration_seconds: None,
            },
            MediaModelInfo {
                id: "facebookresearch/musicgen".to_string(),
                name: "MusicGen (via Replicate)".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Music,
                pricing: None,
                supports_streaming: false,
                max_resolution: None,
                max_duration_seconds: Some(30),
            },
        ])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Replicate: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        // Replicate pricing varies by model — per-second GPU billing
        None
    }
}
