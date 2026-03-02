//! Runway Gen-3 Alpha video generation adapter.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Runway Gen-3 Alpha — high-quality AI video generation.
pub struct RunwayProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl RunwayProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::video_providers::runway(),
            api_key: std::env::var("RUNWAY_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for RunwayProvider {
    fn id(&self) -> &MediaProviderId {
        &self.id
    }

    fn name(&self) -> &str {
        "Runway"
    }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Video]
    }

    fn location(&self) -> MediaProviderLocation {
        MediaProviderLocation::Cloud
    }

    fn is_available(&self) -> bool {
        self.api_key.is_some()
    }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "gen3a_turbo".to_string(),
                name: "Gen-3 Alpha Turbo".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Video,
                pricing: Some(MediaPricing {
                    per_request: MicroCost::from_dollars(0.50),
                    per_second: Some(MicroCost::from_dollars(0.05)),
                    per_character: None,
                }),
                supports_streaming: false,
                max_resolution: Some((1280, 768)),
                max_duration_seconds: Some(10),
            },
        ])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Runway: HTTP integration pending"))
    }

    fn estimate_cost(&self, request: &MediaGenerationRequest) -> Option<MicroCost> {
        let duration = request.duration_seconds.unwrap_or(5.0);
        Some(MicroCost::from_dollars(0.50 + duration * 0.05))
    }
}
