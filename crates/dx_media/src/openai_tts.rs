//! OpenAI TTS adapter — TTS-1 / TTS-1-HD / gpt-4o-mini-tts.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// OpenAI TTS — natural dialogue voices via OpenAI API.
///
/// New-user credits (~$5–18); then ~$15/M chars.
/// API key from: platform.openai.com.
pub struct OpenAiTtsProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl OpenAiTtsProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::audio_providers::openai_tts(),
            api_key: std::env::var("OPENAI_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for OpenAiTtsProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "OpenAI TTS" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Audio] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "tts-1".into(),
                name: "TTS-1".into(),
                provider_id: self.id.clone(),
                media_type: MediaType::Audio,
                pricing: Some(MediaPricing {
                    per_request: MicroCost::zero(),
                    per_second: None,
                    per_character: Some(MicroCost::from_microdollars(15)),
                }),
                supports_streaming: true,
                max_resolution: None,
                max_duration_seconds: None,
            },
            MediaModelInfo {
                id: "tts-1-hd".into(),
                name: "TTS-1 HD".into(),
                provider_id: self.id.clone(),
                media_type: MediaType::Audio,
                pricing: Some(MediaPricing {
                    per_request: MicroCost::zero(),
                    per_second: None,
                    per_character: Some(MicroCost::from_microdollars(30)),
                }),
                supports_streaming: true,
                max_resolution: None,
                max_duration_seconds: None,
            },
        ])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("OpenAI TTS: HTTP integration pending"))
    }

    fn estimate_cost(&self, request: &MediaGenerationRequest) -> Option<MicroCost> {
        let chars = request.prompt.len() as u64;
        Some(MicroCost::from_microdollars(chars * 15))
    }
}
