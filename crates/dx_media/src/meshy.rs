//! Meshy 3D asset generation adapter — text/image to 3D with PBR textures.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Meshy — text-to-3D and image-to-3D with PBR textures.
pub struct MeshyProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl MeshyProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::threed_providers::meshy(),
            api_key: std::env::var("MESHY_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for MeshyProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Meshy" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::ThreeD] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "meshy-v4".to_string(),
                name: "Meshy V4".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::ThreeD,
                pricing: Some(MediaPricing {
                    per_request: MicroCost::from_dollars(0.20),
                    per_second: None,
                    per_character: None,
                }),
                supports_streaming: false,
                max_resolution: None,
                max_duration_seconds: None,
            },
            MediaModelInfo {
                id: "meshy-img-to-3d".to_string(),
                name: "Meshy Image-to-3D".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::ThreeD,
                pricing: Some(MediaPricing {
                    per_request: MicroCost::from_dollars(0.15),
                    per_second: None,
                    per_character: None,
                }),
                supports_streaming: false,
                max_resolution: None,
                max_duration_seconds: None,
            },
        ])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Meshy: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        Some(MicroCost::from_dollars(0.20))
    }
}
