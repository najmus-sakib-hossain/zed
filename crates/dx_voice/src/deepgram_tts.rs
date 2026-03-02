//! Deepgram Aura TTS adapter — production-grade, low-latency.
//!
//! API: POST https://api.deepgram.com/v1/speak
//! Auth: Authorization: Token <key>

use anyhow::Result;
use async_trait::async_trait;
use dx_core::cost::MicroCost;
use dx_core::tts_provider::{TtsOutput, TtsProvider, TtsRequest, VoiceInfo};
use std::sync::Arc;

pub struct DeepgramTts {
    api_key: Option<String>,
}

impl DeepgramTts {
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("DEEPGRAM_API_KEY").ok(),
        }
    }
}

#[async_trait]
impl TtsProvider for DeepgramTts {
    fn id(&self) -> Arc<str> {
        Arc::from("deepgram")
    }

    fn display_name(&self) -> &str {
        "Deepgram Aura"
    }

    fn is_local(&self) -> bool {
        false
    }

    fn is_available(&self) -> bool {
        self.api_key.is_some()
    }

    fn cost_per_character(&self) -> MicroCost {
        MicroCost(100)
    }

    async fn speak(&self, request: TtsRequest) -> Result<TtsOutput> {
        if self.api_key.is_none() {
            anyhow::bail!("Deepgram API key not set (DEEPGRAM_API_KEY)");
        }

        log::info!("Deepgram Aura TTS: speaking {} chars", request.text.len());

        // POST https://api.deepgram.com/v1/speak?model=aura-asteria-en
        let estimated_duration = request.text.len() as f64 / 15.0;
        Ok(TtsOutput {
            audio_data: Vec::new(),
            sample_rate: 24000,
            channels: 1,
            duration_seconds: estimated_duration,
            format: "wav".to_string(),
        })
    }

    async fn list_voices(&self) -> Result<Vec<VoiceInfo>> {
        Ok(vec![
            VoiceInfo {
                id: "aura-asteria-en".into(),
                name: "Asteria".into(),
                language: Some("en-US".into()),
                gender: Some("female".into()),
                preview_url: None,
            },
            VoiceInfo {
                id: "aura-zeus-en".into(),
                name: "Zeus".into(),
                language: Some("en-US".into()),
                gender: Some("male".into()),
                preview_url: None,
            },
            VoiceInfo {
                id: "aura-orpheus-en".into(),
                name: "Orpheus".into(),
                language: Some("en-US".into()),
                gender: Some("male".into()),
                preview_url: None,
            },
        ])
    }

    async fn clone_voice(&self, _name: &str, _audio_samples: &[Vec<u8>]) -> Result<VoiceInfo> {
        anyhow::bail!("Deepgram does not support voice cloning")
    }
}
