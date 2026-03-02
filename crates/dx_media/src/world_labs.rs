//! World Labs spatial AI — text/image to explorable 3D worlds.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// World Labs — text/image/video to fully explorable 3D worlds.
///
/// Trial credits on signup.
/// API key from: worldlabs.ai developer API.
pub struct WorldLabsProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl WorldLabsProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::threed_providers::world_labs(),
            api_key: std::env::var("WORLD_LABS_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for WorldLabsProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "World Labs" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::ThreeD] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "world-gen-v1".into(),
            name: "World Labs 3D Scene Generator".into(),
            provider_id: self.id.clone(),
            media_type: MediaType::ThreeD,
            pricing: Some(MediaPricing {
                per_request: MicroCost::from_dollars(0.50),
                per_second: None,
                per_character: None,
            }),
            supports_streaming: false,
            max_resolution: None,
            max_duration_seconds: None,
        }])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("World Labs: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        Some(MicroCost::from_dollars(0.50))
    }
}
