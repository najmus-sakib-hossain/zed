//! Fish Audio multilingual TTS adapter.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Fish Audio — realistic multilingual TTS with generous free tier.
///
/// Free tier: 1M+ chars/month.
/// API key from: fish.audio developer API.
pub struct FishAudioProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl FishAudioProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::audio_providers::fish_audio(),
            api_key: std::env::var("FISH_AUDIO_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for FishAudioProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Fish Audio" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Audio] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "fish-speech-v1".into(),
            name: "Fish Speech V1".into(),
            provider_id: self.id.clone(),
            media_type: MediaType::Audio,
            pricing: Some(MediaPricing {
                per_request: MicroCost::zero(),
                per_second: None,
                per_character: Some(MicroCost::from_microdollars(5)),
            }),
            supports_streaming: true,
            max_resolution: None,
            max_duration_seconds: None,
        }])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Fish Audio: HTTP integration pending"))
    }

    fn estimate_cost(&self, request: &MediaGenerationRequest) -> Option<MicroCost> {
        let chars = request.prompt.len() as u64;
        Some(MicroCost::from_microdollars(chars * 5))
    }
}
