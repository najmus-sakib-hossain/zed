//! OpenAI Sora video generation adapter — narrative, physics-aware video.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// OpenAI Sora — high-quality text-to-video with physics simulation.
///
/// No broad free tier (paid credits); some Pro access via ChatGPT+.
/// API key from: platform.openai.com.
pub struct OpenAiSoraProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl OpenAiSoraProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::video_providers::openai_sora(),
            api_key: std::env::var("OPENAI_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for OpenAiSoraProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "OpenAI Sora" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Video] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "sora-2".into(),
            name: "Sora 2".into(),
            provider_id: self.id.clone(),
            media_type: MediaType::Video,
            pricing: Some(MediaPricing {
                per_request: MicroCost::from_dollars(1.00),
                per_second: Some(MicroCost::from_dollars(0.10)),
                per_character: None,
            }),
            supports_streaming: false,
            max_resolution: Some((1920, 1080)),
            max_duration_seconds: Some(60),
        }])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("OpenAI Sora: HTTP integration pending"))
    }

    fn estimate_cost(&self, request: &MediaGenerationRequest) -> Option<MicroCost> {
        let duration = request.duration_seconds.unwrap_or(10.0);
        Some(MicroCost::from_dollars(1.00 + duration * 0.10))
    }
}
