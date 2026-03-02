//! Google Cloud Text-to-Speech adapter — 300+ WaveNet/Neural voices.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// Google Cloud TTS — 300+ WaveNet/Neural voices, 50+ languages, SSML support.
///
/// Free tier: up to 5M chars/month depending on region.
/// API key from: console.cloud.google.com → APIs & Services → Credentials.
pub struct GoogleTtsProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl GoogleTtsProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::audio_providers::google_tts(),
            api_key: std::env::var("GOOGLE_AI_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for GoogleTtsProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "Google TTS" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Audio] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "wavenet".into(),
                name: "Google WaveNet".into(),
                provider_id: self.id.clone(),
                media_type: MediaType::Audio,
                pricing: Some(MediaPricing {
                    per_request: MicroCost::zero(),
                    per_second: None,
                    per_character: Some(MicroCost::from_microdollars(16)),
                }),
                supports_streaming: true,
                max_resolution: None,
                max_duration_seconds: None,
            },
            MediaModelInfo {
                id: "neural2".into(),
                name: "Google Neural2".into(),
                provider_id: self.id.clone(),
                media_type: MediaType::Audio,
                pricing: Some(MediaPricing {
                    per_request: MicroCost::zero(),
                    per_second: None,
                    per_character: Some(MicroCost::from_microdollars(16)),
                }),
                supports_streaming: true,
                max_resolution: None,
                max_duration_seconds: None,
            },
        ])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("Google TTS: HTTP integration pending"))
    }

    fn estimate_cost(&self, request: &MediaGenerationRequest) -> Option<MicroCost> {
        let chars = request.prompt.len() as u64;
        Some(MicroCost::from_microdollars(chars * 16))
    }
}
