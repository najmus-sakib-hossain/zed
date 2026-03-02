//! Synthesia AI avatar video adapter.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Synthesia — AI avatar video generation with lip-synced speech.
pub struct SynthesiaProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl SynthesiaProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::video_providers::synthesia(),
            api_key: std::env::var("SYNTHESIA_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for SynthesiaProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Synthesia" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Video] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "synthesia-avatar".to_string(),
            name: "AI Avatar Video".to_string(),
            provider_id: self.id.clone(),
            media_type: MediaType::Video,
            pricing: Some(MediaPricing {
                per_request: MicroCost::from_dollars(1.00),
                per_second: Some(MicroCost::from_dollars(0.10)),
                per_character: None,
            }),
            supports_streaming: false,
            max_resolution: Some((1920, 1080)),
            max_duration_seconds: Some(300),
        }])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Synthesia: HTTP integration pending"))
    }

    fn estimate_cost(&self, request: &MediaGenerationRequest) -> Option<MicroCost> {
        let duration = request.duration_seconds.unwrap_or(30.0);
        Some(MicroCost::from_dollars(1.00 + duration * 0.10))
    }
}
