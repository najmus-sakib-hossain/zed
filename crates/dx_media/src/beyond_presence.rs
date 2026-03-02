//! Beyond Presence real-time audio-to-video avatar adapter.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Beyond Presence — real-time audio-to-video API with expressive avatars.
///
/// Direct LiveKit integration for agents. Free testing credits.
/// API key from: bey.dev.
pub struct BeyondPresenceProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl BeyondPresenceProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::live_providers::beyond_presence(),
            api_key: std::env::var("BEYOND_PRESENCE_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for BeyondPresenceProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Beyond Presence" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Live] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "bey-realtime-v1".into(),
            name: "Beyond Presence RT Avatar".into(),
            provider_id: self.id.clone(),
            media_type: MediaType::Live,
            pricing: Some(MediaPricing {
                per_request: MicroCost::from_dollars(0.30),
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
        Err(anyhow::anyhow!("Beyond Presence: WebSocket integration pending"))
    }

    fn estimate_cost(&self, request: &MediaGenerationRequest) -> Option<MicroCost> {
        let duration = request.duration_seconds.unwrap_or(60.0);
        Some(MicroCost::from_dollars(0.30 + duration * 0.01))
    }
}
