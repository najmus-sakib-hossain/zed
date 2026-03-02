//! Hedra — Expressive real-time talking heads with LiveKit integration.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaType, MicroCost,
};

/// Hedra — Expressive character-driven real-time conversations.
pub struct HedraProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl HedraProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::live_providers::hedra(),
            api_key: std::env::var("HEDRA_API_KEY").ok(),
        }
    }
}

impl Default for HedraProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl MediaProvider for HedraProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Hedra" }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Live, MediaType::Video]
    }

    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "character-streaming".to_string(),
                name: "Character Streaming (LiveKit)".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Live,
                pricing: None,
                supports_streaming: true,
                max_resolution: Some((1280, 720)),
                max_duration_seconds: None,
            },
        ])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Hedra: WebSocket integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        // Free tier + credits
        None
    }
}
