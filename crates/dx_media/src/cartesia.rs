//! Cartesia sub-200ms low-latency TTS adapter.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Cartesia — sub-200ms conversational TTS, perfect for live agents.
///
/// Free tier + credits on signup.
/// API key from: cartesia.ai.
pub struct CartesiaProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl CartesiaProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::audio_providers::cartesia(),
            api_key: std::env::var("CARTESIA_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for CartesiaProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Cartesia" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Audio] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "sonic-2".into(),
            name: "Cartesia Sonic 2".into(),
            provider_id: self.id.clone(),
            media_type: MediaType::Audio,
            pricing: Some(MediaPricing {
                per_request: MicroCost::zero(),
                per_second: None,
                per_character: Some(MicroCost::from_microdollars(15)),
            }),
            supports_streaming: true,
            max_resolution: None,
            max_duration_seconds: None,
        }])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Cartesia: HTTP integration pending"))
    }

    fn estimate_cost(&self, request: &MediaGenerationRequest) -> Option<MicroCost> {
        let chars = request.prompt.len() as u64;
        Some(MicroCost::from_microdollars(chars * 15))
    }
}
