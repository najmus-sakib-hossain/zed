//! Kling AI video generation adapter (by Kuaishou).

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Kling AI — fast, high-quality video generation.
pub struct KlingAiProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl KlingAiProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::video_providers::kling_ai(),
            api_key: std::env::var("KLING_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for KlingAiProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Kling AI" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Video] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "kling-v1.5".to_string(),
            name: "Kling 1.5 Pro".to_string(),
            provider_id: self.id.clone(),
            media_type: MediaType::Video,
            pricing: Some(MediaPricing {
                per_request: MicroCost::from_dollars(0.30),
                per_second: Some(MicroCost::from_dollars(0.03)),
                per_character: None,
            }),
            supports_streaming: false,
            max_resolution: Some((1920, 1080)),
            max_duration_seconds: Some(10),
        }])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Kling AI: HTTP integration pending"))
    }

    fn estimate_cost(&self, request: &MediaGenerationRequest) -> Option<MicroCost> {
        let duration = request.duration_seconds.unwrap_or(5.0);
        Some(MicroCost::from_dollars(0.30 + duration * 0.03))
    }
}
