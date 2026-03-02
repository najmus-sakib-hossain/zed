//! Bria.ai image generation adapter — commercial-safe AI images.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Bria.ai — enterprise-grade, commercially safe image generation.
///
/// Free tier: 1,000 free API calls on signup.
/// API key from: bria.ai/api.
pub struct BriaProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl BriaProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::image_providers::bria(),
            api_key: std::env::var("BRIA_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for BriaProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Bria" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Image] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "bria-2.0".into(),
            name: "Bria 2.0".into(),
            provider_id: self.id.clone(),
            media_type: MediaType::Image,
            pricing: Some(MediaPricing {
                per_request: MicroCost::from_dollars(0.03),
                per_second: None,
                per_character: None,
            }),
            supports_streaming: false,
            max_resolution: Some((1024, 1024)),
            max_duration_seconds: None,
        }])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Bria: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        Some(MicroCost::from_dollars(0.03))
    }
}
