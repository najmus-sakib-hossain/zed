//! ElevenLabs audio generation adapter — TTS, voice cloning, sound effects, music.

use anyhow::Result;
use dx_core::{
    MediaGenerationRequest, MediaModelInfo, MediaOutput, MediaProvider, MediaProviderId,
    MediaProviderLocation, MediaPricing, MediaType, MicroCost,
};

/// ElevenLabs — ultra-realistic TTS, voice cloning, sound effects, and music.
///
/// Free tier: 10,000 chars/month (recurring) + extra on signup.
/// API key from: elevenlabs.io dashboard.
pub struct ElevenLabsProvider {
    id: MediaProviderId,
    api_key: Option<String>,
}

impl ElevenLabsProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::audio_providers::elevenlabs(),
            api_key: std::env::var("ELEVENLABS_API_KEY").ok(),
        }
    }
}

#[async_trait::async_trait]
impl MediaProvider for ElevenLabsProvider {
    fn id(&self) -> &MediaProviderId { &self.id }
    fn name(&self) -> &str { "ElevenLabs" }
    fn supported_media_types(&self) -> &[MediaType] { &[MediaType::Audio, MediaType::Music] }
    fn location(&self) -> MediaProviderLocation { MediaProviderLocation::Cloud }
    fn is_available(&self) -> bool { self.api_key.is_some() }

    async fn list_models(&self) -> Result<Vec<MediaModelInfo>> {
        Ok(vec![
            MediaModelInfo {
                id: "eleven_turbo_v2".into(),
                name: "ElevenLabs Turbo V2 (TTS)".into(),
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
            MediaModelInfo {
                id: "eleven_music_v1".into(),
                name: "ElevenLabs Music V1".into(),
                provider_id: self.id.clone(),
                media_type: MediaType::Music,
                pricing: Some(MediaPricing {
                    per_request: MicroCost::from_dollars(0.10),
                    per_second: None,
                    per_character: None,
                }),
                supports_streaming: false,
                max_resolution: None,
                max_duration_seconds: Some(300),
            },
            MediaModelInfo {
                id: "eleven_sfx_v1".into(),
                name: "ElevenLabs Sound Effects".into(),
                provider_id: self.id.clone(),
                media_type: MediaType::Audio,
                pricing: Some(MediaPricing {
                    per_request: MicroCost::from_dollars(0.05),
                    per_second: None,
                    per_character: None,
                }),
                supports_streaming: false,
                max_resolution: None,
                max_duration_seconds: Some(30),
            },
        ])
    }

    async fn generate(&self, request: &MediaGenerationRequest) -> Result<Vec<MediaOutput>> {
        let _ = request;
        Err(anyhow::anyhow!("ElevenLabs: HTTP integration pending"))
    }

    fn estimate_cost(&self, request: &MediaGenerationRequest) -> Option<MicroCost> {
        let chars = request.prompt.len() as u64;
        Some(MicroCost::from_microdollars(chars * 30))
    }
}
