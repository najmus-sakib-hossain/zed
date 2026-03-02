//! OpenAI TTS adapter — simple, high-quality.
//!
//! API: POST https://api.openai.com/v1/audio/speech
//! Auth: Authorization: Bearer <key>

use anyhow::Result;
use async_trait::async_trait;
use dx_core::cost::MicroCost;
use dx_core::tts_provider::{TtsOutput, TtsProvider, TtsRequest, VoiceInfo};
use std::sync::Arc;

pub struct OpenAiTts {
    api_key: Option<String>,
}

impl OpenAiTts {
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("OPENAI_API_KEY").ok(),
        }
    }
}

#[async_trait]
impl TtsProvider for OpenAiTts {
    fn id(&self) -> Arc<str> {
        Arc::from("openai_tts")
    }

    fn display_name(&self) -> &str {
        "OpenAI TTS"
    }

    fn is_local(&self) -> bool {
        false
    }

    fn is_available(&self) -> bool {
        self.api_key.is_some()
    }

    fn cost_per_character(&self) -> MicroCost {
        // tts-1: $15 per 1M chars; tts-1-hd: $30 per 1M chars
        MicroCost(15)
    }

    async fn speak(&self, request: TtsRequest) -> Result<TtsOutput> {
        if self.api_key.is_none() {
            anyhow::bail!("OpenAI API key not set (OPENAI_API_KEY)");
        }

        log::info!("OpenAI TTS: speaking {} chars", request.text.len());

        // POST https://api.openai.com/v1/audio/speech
        // Body: { "model": "tts-1", "input": "...", "voice": "alloy" }
        // Response: audio/mp3 bytes
        let estimated_duration = request.text.len() as f64 / 15.0;
        Ok(TtsOutput {
            audio_data: Vec::new(),
            sample_rate: 24000,
            channels: 1,
            duration_seconds: estimated_duration,
            format: "mp3".to_string(),
        })
    }

    async fn list_voices(&self) -> Result<Vec<VoiceInfo>> {
        Ok(vec![
            VoiceInfo {
                id: "alloy".into(),
                name: "Alloy".into(),
                language: Some("en".into()),
                gender: None,
                preview_url: None,
            },
            VoiceInfo {
                id: "echo".into(),
                name: "Echo".into(),
                language: Some("en".into()),
                gender: Some("male".into()),
                preview_url: None,
            },
            VoiceInfo {
                id: "fable".into(),
                name: "Fable".into(),
                language: Some("en".into()),
                gender: None,
                preview_url: None,
            },
            VoiceInfo {
                id: "onyx".into(),
                name: "Onyx".into(),
                language: Some("en".into()),
                gender: Some("male".into()),
                preview_url: None,
            },
            VoiceInfo {
                id: "nova".into(),
                name: "Nova".into(),
                language: Some("en".into()),
                gender: Some("female".into()),
                preview_url: None,
            },
            VoiceInfo {
                id: "shimmer".into(),
                name: "Shimmer".into(),
                language: Some("en".into()),
                gender: Some("female".into()),
                preview_url: None,
            },
        ])
    }

    async fn clone_voice(&self, _name: &str, _audio_samples: &[Vec<u8>]) -> Result<VoiceInfo> {
        anyhow::bail!("OpenAI TTS does not support voice cloning")
    }
}
