//! Anam AI sub-second photorealistic real-time avatar adapter.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Anam AI — sub-second photorealistic real-time avatars.
///
/// Free dev tier available.
/// API key from: anam.ai portal.
pub struct AnamAiProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl AnamAiProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::live_providers::anam_ai(),
            api_key: std::env::var("ANAM_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for AnamAiProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Anam AI" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Live] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "anam-realtime-v1".into(),
            name: "Anam AI RT Avatar".into(),
            provider_id: self.id.clone(),
            media_type: MediaType::Live,
            pricing: Some(MediaPricing {
                per_request: MicroCost::from_dollars(0.25),
                per_second: Some(MicroCost::from_dollars(0.01)),
                per_character: None,
            }),
            supports_streaming: true,
            max_resolution: Some((1280, 720)),
            max_duration_seconds: Some(3600),
        }])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Anam AI: streaming integration pending"))
    }

    fn estimate_cost(&self, request: &MediaGenerationRequest) -> Option<MicroCost> {
        let duration = request.duration_seconds.unwrap_or(60.0);
        Some(MicroCost::from_dollars(0.25 + duration * 0.01))
    }
}
