//! Tripo AI 3D generation adapter — fast 3D model generation.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Tripo AI — fast 3D generation from text and images.
pub struct TripoAiProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl TripoAiProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::threed_providers::tripo_ai(),
            api_key: std::env::var("TRIPO_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for TripoAiProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Tripo AI" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::ThreeD] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "tripo-v2".to_string(),
            name: "Tripo V2".to_string(),
            provider_id: self.id.clone(),
            media_type: MediaType::ThreeD,
            pricing: Some(MediaPricing {
                per_request: MicroCost::from_dollars(0.15),
                per_second: None,
                per_character: None,
            }),
            supports_streaming: false,
            max_resolution: None,
            max_duration_seconds: None,
        }])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Tripo AI: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        Some(MicroCost::from_dollars(0.15))
    }
}
