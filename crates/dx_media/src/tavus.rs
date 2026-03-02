//! Tavus — Conversational Video Interface (CVI) for real-time face-to-face AI.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaType, MicroCost,
};

/// Tavus — Real-time see/hear/respond photorealistic digital twins.
pub struct TavusProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl TavusProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::live_providers::tavus(),
            api_key: std::env::var("TAVUS_API_KEY").ok(),
        }
    }
}

impl Default for TavusProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl MediaProvider for TavusProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Tavus" }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Live]
    }

    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "phoenix-3".to_string(),
                name: "Phoenix-3 (CVI Real-time)".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Live,
                pricing: None,
                supports_streaming: true,
                max_resolution: Some((1920, 1080)),
                max_duration_seconds: None, // Unlimited for live sessions
            },
        ])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Tavus: WebRTC integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        // Free developer testing tier
        None
    }
}
