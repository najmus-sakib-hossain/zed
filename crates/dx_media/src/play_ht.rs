//! Play.ht TTS adapter — emotional voices + voice cloning.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Play.ht — TTS with emotional voices and voice cloning.
///
/// Free tier: 5,000 words/month.
/// API key from: play.ht dashboard.
pub struct PlayHtProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl PlayHtProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::audio_providers::play_ht(),
            api_key: std::env::var("PLAYHT_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for PlayHtProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Play.ht" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Audio] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "playht-2.0".into(),
            name: "Play.ht 2.0 TTS".into(),
            provider_id: self.id.clone(),
            media_type: MediaType::Audio,
            pricing: Some(MediaPricing {
                per_request: MicroCost::zero(),
                per_second: None,
                per_character: Some(MicroCost::from_microdollars(20)),
            }),
            supports_streaming: true,
            max_resolution: None,
            max_duration_seconds: None,
        }])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Play.ht: HTTP integration pending"))
    }

    fn estimate_cost(&self, request: &MediaGenerationRequest) -> Option<MicroCost> {
        let chars = request.prompt.len() as u64;
        Some(MicroCost::from_microdollars(chars * 20))
    }
}
