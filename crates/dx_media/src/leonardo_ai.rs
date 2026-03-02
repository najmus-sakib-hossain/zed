//! Leonardo.ai image generation adapter — Phoenix, Vision XL.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Leonardo.ai — cinematic, product, and anime-style image generation.
///
/// Free plan: daily tokens + $5–10 API credit on signup.
/// API key from: leonardo.ai dashboard.
pub struct LeonardoAiProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl LeonardoAiProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::image_providers::leonardo_ai(),
            api_key: std::env::var("LEONARDO_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for LeonardoAiProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Leonardo.ai" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Image] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "phoenix-1.0".into(),
                name: "Leonardo Phoenix".into(),
                provider_id: self.id.clone(),
                media_type: MediaType::Image,
                pricing: Some(MediaPricing {
                    per_request: MicroCost::from_dollars(0.05),
                    per_second: None,
                    per_character: None,
                }),
                supports_streaming: false,
                max_resolution: Some((2048, 2048)),
                max_duration_seconds: None,
            },
            MediaModelInfo {
                id: "vision-xl".into(),
                name: "Leonardo Vision XL".into(),
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
            },
        ])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Leonardo.ai: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        Some(MicroCost::from_dollars(0.05))
    }
}
