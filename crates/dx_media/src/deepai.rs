//! DeepAI image generation adapter — simple, affordable AI image generation.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// DeepAI — text-to-image, image-to-image, and inpainting.
///
/// Free tier: 100 free credits on signup.
/// API key from: api.deepai.org.
pub struct DeepAiProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl DeepAiProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::image_providers::deepai(),
            api_key: std::env::var("DEEPAI_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for DeepAiProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "DeepAI" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Image] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "text2img".into(),
            name: "DeepAI Text-to-Image".into(),
            provider_id: self.id.clone(),
            media_type: MediaType::Image,
            pricing: Some(MediaPricing {
                per_request: MicroCost::from_dollars(0.05),
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
        Err(anyhow::anyhow!("DeepAI: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        Some(MicroCost::from_dollars(0.05))
    }
}
