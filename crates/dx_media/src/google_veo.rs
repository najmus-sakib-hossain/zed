//! Google Veo 3 video generation adapter via Vertex AI / Gemini API.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Google Veo — high-quality text-to-video via Vertex AI or AI Studio.
///
/// Free tier: 100+ free credits/month; $300 new-user Cloud credits.
/// API key from: ai.google.dev or cloud.google.com/vertex-ai.
pub struct GoogleVeoProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl GoogleVeoProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::video_providers::google_veo(),
            api_key: std::env::var("GOOGLE_AI_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for GoogleVeoProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Google Veo" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Video] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "veo-3".into(),
            name: "Veo 3".into(),
            provider_id: self.id.clone(),
            media_type: MediaType::Video,
            pricing: Some(MediaPricing {
                per_request: MicroCost::from_dollars(0.50),
                per_second: Some(MicroCost::from_dollars(0.05)),
                per_character: None,
            }),
            supports_streaming: false,
            max_resolution: Some((1920, 1080)),
            max_duration_seconds: Some(30),
        }])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Google Veo: HTTP integration pending"))
    }

    fn estimate_cost(&self, request: &MediaGenerationRequest) -> Option<MicroCost> {
        let duration = request.duration_seconds.unwrap_or(8.0);
        Some(MicroCost::from_dollars(0.50 + duration * 0.05))
    }
}
