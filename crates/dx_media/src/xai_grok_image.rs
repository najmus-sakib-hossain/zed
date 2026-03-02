//! xAI Grok image generation adapter — Flux-based, fast image generation.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// xAI Grok Image — OpenAI-compatible image generation via x.ai.
///
/// Pay-per-image (~$0.07); new users often get test credits.
/// API key from: console.x.ai.
pub struct XaiGrokImageProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl XaiGrokImageProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::image_providers::xai_grok(),
            api_key: std::env::var("XAI_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for XaiGrokImageProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "xAI Grok Image" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Image] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "grok-2-image-1212".into(),
            name: "Grok 2 Image".into(),
            provider_id: self.id.clone(),
            media_type: MediaType::Image,
            pricing: Some(MediaPricing {
                per_request: MicroCost::from_dollars(0.07),
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
        Err(anyhow::anyhow!("xAI Grok Image: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        Some(MicroCost::from_dollars(0.07))
    }
}
