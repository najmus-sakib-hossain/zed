//! SwiftXR — programmatic WebXR/AR/VR scene creation and publishing.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// SwiftXR — create and publish WebXR/AR/VR scenes programmatically.
///
/// Free plan: unlimited updates + 1k views batches.
/// API key from: home.swiftxr.io dashboard.
pub struct SwiftXrProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl SwiftXrProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::threed_providers::swiftxr(),
            api_key: std::env::var("SWIFTXR_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for SwiftXrProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "SwiftXR" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::ThreeD] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "swiftxr-scene".into(),
            name: "SwiftXR WebXR Scene".into(),
            provider_id: self.id.clone(),
            media_type: MediaType::ThreeD,
            pricing: Some(MediaPricing {
                per_request: MicroCost::from_dollars(0.10),
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
        Err(anyhow::anyhow!("SwiftXR: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        Some(MicroCost::from_dollars(0.10))
    }
}
