//! Kaedim image-to-3D adapter — production 3D from photos/sketches.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Kaedim — high-quality image-to-3D for games and AR.
///
/// Free tier + trial credits.
/// API key from: app.kaedim3d.com → Settings.
pub struct KaedimProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl KaedimProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::threed_providers::kaedim(),
            api_key: std::env::var("KAEDIM_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for KaedimProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Kaedim" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::ThreeD] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "kaedim-img2mesh".into(),
            name: "Kaedim Image-to-3D".into(),
            provider_id: self.id.clone(),
            media_type: MediaType::ThreeD,
            pricing: Some(MediaPricing {
                per_request: MicroCost::from_dollars(1.00),
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
        Err(anyhow::anyhow!("Kaedim: HTTP integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        Some(MicroCost::from_dollars(1.00))
    }
}
