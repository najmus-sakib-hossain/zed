//! DeepBrain AI real-time conversation avatar adapter.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// DeepBrain AI — real-time conversation avatars with multi-language lip-sync.
///
/// Trial credits on signup.
/// API key from: aistudios.com dashboard.
pub struct DeepbrainAiProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl DeepbrainAiProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::live_providers::deepbrain_ai(),
            api_key: std::env::var("DEEPBRAIN_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for DeepbrainAiProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "DeepBrain AI" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Live, MediaType::Video] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "deepbrain-live".into(),
                name: "DeepBrain Live Avatar".into(),
                provider_id: self.id.clone(),
                media_type: MediaType::Live,
                pricing: Some(MediaPricing {
                    per_request: MicroCost::from_dollars(0.50),
                    per_second: Some(MicroCost::from_dollars(0.02)),
                    per_character: None,
                }),
                supports_streaming: true,
                max_resolution: Some((1920, 1080)),
                max_duration_seconds: Some(3600),
            },
            MediaModelInfo {
                id: "deepbrain-video".into(),
                name: "DeepBrain Video".into(),
                provider_id: self.id.clone(),
                media_type: MediaType::Video,
                pricing: Some(MediaPricing {
                    per_request: MicroCost::from_dollars(1.00),
                    per_second: Some(MicroCost::from_dollars(0.10)),
                    per_character: None,
                }),
                supports_streaming: false,
                max_resolution: Some((1920, 1080)),
                max_duration_seconds: Some(300),
            },
        ])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("DeepBrain AI: HTTP integration pending"))
    }

    fn estimate_cost(&self, request: &MediaGenerationRequest) -> Option<MicroCost> {
        let duration = request.duration_seconds.unwrap_or(60.0);
        Some(MicroCost::from_dollars(0.50 + duration * 0.02))
    }
}
