//! Hailuo AI video generation adapter — generous free tier.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Hailuo AI — text-to-video with one of the most generous free tiers.
///
/// Free tier: generous daily/monthly credits on freemium plan.
/// API key from: Hailuo developer portal.
pub struct HailuoAiProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl HailuoAiProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::video_providers::hailuo_ai(),
            api_key: std::env::var("HAILUO_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for HailuoAiProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Hailuo AI" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Video] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "hailuo-v1".into(),
            name: "Hailuo Text-to-Video".into(),
            provider_id: self.id.clone(),
            media_type: MediaType::Video,
            pricing: Some(MediaPricing {
                per_request: MicroCost::from_dollars(0.20),
                per_second: Some(MicroCost::from_dollars(0.02)),
                per_character: None,
            }),
            supports_streaming: false,
            max_resolution: Some((1280, 720)),
            max_duration_seconds: Some(10),
        }])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Hailuo AI: HTTP integration pending"))
    }

    fn estimate_cost(&self, request: &MediaGenerationRequest) -> Option<MicroCost> {
        let duration = request.duration_seconds.unwrap_or(5.0);
        Some(MicroCost::from_dollars(0.20 + duration * 0.02))
    }
}
