//! D-ID — Real-time streaming avatars and talking heads.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaType, MicroCost,
};

/// D-ID — Real-time low-latency streaming avatars for live conversations.
pub struct DIdProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl DIdProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::live_providers::d_id(),
            api_key: std::env::var("D_ID_API_KEY").ok(),
        }
    }
}

impl Default for DIdProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl MediaProvider for DIdProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "D-ID" }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Video, MediaType::Live]
    }

    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "talks/stream".to_string(),
                name: "Real-time Streaming Avatar".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Live,
                pricing: None,
                supports_streaming: true,
                max_resolution: Some((1920, 1080)),
                max_duration_seconds: None, // Unlimited for live sessions
            },
            MediaModelInfo {
                id: "talks".to_string(),
                name: "Async Talking Head Video".to_string(),
                provider_id: self.id.clone(),
                media_type: MediaType::Video,
                pricing: None,
                supports_streaming: false,
                max_resolution: Some((1920, 1080)),
                max_duration_seconds: Some(300),
            },
        ])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("D-ID: HTTP/WebSocket integration pending"))
    }

    fn estimate_cost(&self, _request: &MediaGenerationRequest) -> Option<MicroCost> {
        // 14-day trial + limited minutes
        None
    }
}
