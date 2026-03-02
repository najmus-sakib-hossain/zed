//! Simli — Real-time avatar streaming with sub-second latency.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaType, MicroCost,
};

/// Simli — Sub-second latency real-time avatar streaming.
pub struct SimliProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl SimliProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::live_providers::simli(),
            api_key: std::env::var("SIMLI_API_KEY").ok(),
        }
    }
}

impl Default for SimliProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl MediaProvider for SimliProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Simli" }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Live]
    }

    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "avatar-stream".to_string(),
                name: "Real-time Avatar Stream".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Live,
                pricing: None,
                supports_streaming: true,
                max_resolution: Some((1280, 720)),
                max_duration_seconds: None,
            },
        ])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Simli: WebSocket integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        // $10 free credit + ~50 min/month on free plan
        None
    }
}
