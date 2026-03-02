//! Deepgram Aura-2 TTS adapter — high-quality conversational TTS.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Deepgram Aura-2 — high-quality TTS optimized for agents.
///
/// Free tier: $200 free credit on signup.
/// API key from: deepgram.com.
pub struct DeepgramProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl DeepgramProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::audio_providers::deepgram(),
            api_key: std::env::var("DEEPGRAM_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for DeepgramProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Deepgram" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Audio] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![MediaModelInfo {
            id: "aura-2".into(),
            name: "Deepgram Aura-2".into(),
            provider_id: self.id.clone(),
            media_type: MediaType::Audio,
            pricing: Some(MediaPricing {
                per_request: MicroCost::zero(),
                per_second: None,
                per_character: Some(MicroCost::from_microdollars(10)),
            }),
            supports_streaming: true,
            max_resolution: None,
            max_duration_seconds: None,
        }])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Deepgram: HTTP integration pending"))
    }

    fn estimate_cost(&self, request: &MediaGenerationRequest) -> Option<MicroCost> {
        let chars = request.prompt.len() as u64;
        Some(MicroCost::from_microdollars(chars * 10))
    }
}
