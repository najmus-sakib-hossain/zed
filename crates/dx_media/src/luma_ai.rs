//! Luma AI Dream Machine video generation adapter.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Luma AI Dream Machine — realistic video generation from text/image.
pub struct LumaAiVideoProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl LumaAiVideoProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::video_providers::luma_ai(),
            api_key: std::env::var("LUMA_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for LumaAiVideoProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Luma AI" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Video] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "dream-machine".to_string(),
            name: "Dream Machine".to_string(),
            provider_id: self.id.clone(),
            media_type: MediaType::Video,
            pricing: Some(MediaPricing {
                per_request: MicroCost::from_dollars(0.40),
                per_second: Some(MicroCost::from_dollars(0.04)),
                per_character: None,
            }),
            supports_streaming: false,
            max_resolution: Some((1360, 752)),
            max_duration_seconds: Some(5),
        }])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Luma AI: HTTP integration pending"))
    }

    fn estimate_cost(&self, request: &MediaGenerationRequest) -> Option<MicroCost> {
        let duration = request.duration_seconds.unwrap_or(5.0);
        Some(MicroCost::from_dollars(0.40 + duration * 0.04))
    }
}
