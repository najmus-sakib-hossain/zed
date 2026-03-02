//! Udio music generation adapter — high-quality AI music.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Udio — high-quality AI music generation.
pub struct UdioProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl UdioProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::music_providers::udio(),
            api_key: std::env::var("UDIO_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for UdioProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Udio" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Music] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "udio-v2".to_string(),
            name: "Udio V2".to_string(),
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
        }])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Udio: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        Some(MicroCost::from_dollars(0.10))
    }
}
