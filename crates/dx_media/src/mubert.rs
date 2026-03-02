//! Mubert generative royalty-free background music adapter.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Mubert — AI-generated royalty-free background music (infinite streams).
///
/// Free Ambassador plan: 25 tracks/month + API.
/// API key from: mubert.com.
pub struct MubertProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl MubertProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::music_providers::mubert(),
            api_key: std::env::var("MUBERT_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for MubertProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Mubert" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Music] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "mubert-gen".into(),
            name: "Mubert Generative".into(),
            provider_id: self.id.clone(),
            media_type: MediaType::Music,
            pricing: Some(MediaPricing {
                per_request: MicroCost::from_dollars(0.05),
                per_second: None,
                per_character: None,
            }),
            supports_streaming: true,
            max_resolution: None,
            max_duration_seconds: Some(600),
        }])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Mubert: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        Some(MicroCost::from_dollars(0.05))
    }
}
