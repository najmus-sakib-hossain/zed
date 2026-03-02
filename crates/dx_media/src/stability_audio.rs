//! Stability Audio adapter — high-quality sound and music generation.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Stability Audio — sound effects and music generation by Stability AI.
pub struct StabilityAudioProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl StabilityAudioProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::music_providers::stability_audio(),
            api_key: std::env::var("STABILITY_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for StabilityAudioProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Stability Audio" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Audio, MediaType::Music] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "stable-audio-2.0".to_string(),
                name: "Stable Audio 2.0".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Music,
                pricing: Some(MediaPricing {
                    per_request: MicroCost::from_dollars(0.04),
                    per_second: None,
                    per_character: None,
                }),
                supports_streaming: false,
                max_resolution: None,
                max_duration_seconds: Some(180),
            },
            MediaModelInfo {
                id: "stable-audio-sfx".to_string(),
                name: "Stable Audio SFX".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Audio,
                pricing: Some(MediaPricing {
                    per_request: MicroCost::from_dollars(0.02),
                    per_second: None,
                    per_character: None,
                }),
                supports_streaming: false,
                max_resolution: None,
                max_duration_seconds: Some(30),
            },
        ])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Stability Audio: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        Some(MicroCost::from_dollars(0.04))
    }
}
