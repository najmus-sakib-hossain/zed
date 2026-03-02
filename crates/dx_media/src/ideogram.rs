//! Ideogram 2.0 image generation adapter — best text rendering in images.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Ideogram — AI image generation with best-in-class text rendering.
///
/// Free daily generations + API credits on signup.
/// API key from: ideogram.ai developer portal.
pub struct IdeogramProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl IdeogramProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::image_providers::ideogram(),
            api_key: std::env::var("IDEOGRAM_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for IdeogramProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Ideogram" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Image] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "ideogram-v2".into(),
            name: "Ideogram 2.0".into(),
            provider_id: self.id.clone(),
            media_type: MediaType::Image,
            pricing: Some(MediaPricing {
                per_request: MicroCost::from_dollars(0.08),
                per_second: None,
                per_character: None,
            }),
            supports_streaming: false,
            max_resolution: Some((2048, 2048)),
            max_duration_seconds: None,
        }])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Ideogram: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        Some(MicroCost::from_dollars(0.08))
    }
}
