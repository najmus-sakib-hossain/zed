//! Pika video generation adapter.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Pika — AI video generation with cinematic effects.
pub struct PikaProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl PikaProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::video_providers::pika(),
            api_key: std::env::var("PIKA_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for PikaProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Pika" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Video] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "pika-2.0".to_string(),
            name: "Pika 2.0".to_string(),
            provider_id: self.id.clone(),
            media_type: MediaType::Video,
            pricing: Some(MediaPricing {
                per_request: MicroCost::from_dollars(0.25),
                per_second: None,
                per_character: None,
            }),
            supports_streaming: false,
            max_resolution: Some((1280, 720)),
            max_duration_seconds: Some(4),
        }])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Pika: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        Some(MicroCost::from_dollars(0.25))
    }
}
