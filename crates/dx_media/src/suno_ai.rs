//! Suno AI music generation adapter — full song generation with vocals + instruments.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Suno AI — AI-powered music generation with vocals, instruments, and lyrics.
pub struct SunoAiProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl SunoAiProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::music_providers::suno_ai(),
            api_key: std::env::var("SUNO_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for SunoAiProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Suno AI" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Music] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "suno-v4".to_string(),
                name: "Suno V4".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Music,
                pricing: Some(MediaPricing {
                    per_request: MicroCost::from_dollars(0.10),
                    per_second: None,
                    per_character: None,
                }),
                supports_streaming: false,
                max_resolution: None,
                max_duration_seconds: Some(240),
            },
        ])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Suno AI: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        Some(MicroCost::from_dollars(0.10))
    }
}
