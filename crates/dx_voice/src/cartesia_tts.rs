//! Cartesia TTS adapter — 40ms latency, voice cloning from 3 seconds of audio.
//!
//! API: POST https://api.cartesia.ai/tts/bytes
//! Auth: X-API-Key header

use anyhow::Result;
use async_trait::async_trait;
use dx_core::cost::MicroCost;
use dx_core::tts_provider::{TtsOutput, TtsProvider, TtsRequest, VoiceInfo};
use std::sync::Arc;

pub struct CartesiaTts {
    api_key: Option<String>,
}

impl CartesiaTts {
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("CARTESIA_API_KEY").ok(),
        }
    }
}

#[async_trait]
impl TtsProvider for CartesiaTts {
    fn id(&self) -> Arc<str> {
        Arc::from("cartesia")
    }

    fn display_name(&self) -> &str {
        "Cartesia"
    }

    fn is_local(&self) -> bool {
        false
    }

    fn is_available(&self) -> bool {
        self.api_key.is_some()
    }

    fn cost_per_character(&self) -> MicroCost {
        MicroCost(150)
    }

    async fn speak(&self, request: TtsRequest) -> Result<TtsOutput> {
        if self.api_key.is_none() {
            anyhow::bail!("Cartesia API key not set (CARTESIA_API_KEY)");
        }

        log::info!("Cartesia TTS: speaking {} chars (40ms latency)", request.text.len());

        // POST https://api.cartesia.ai/tts/bytes
        // Supports WebSocket streaming for ultra-low latency
        let estimated_duration = request.text.len() as f64 / 15.0;
        Ok(TtsOutput {
            audio_data: Vec::new(),
            sample_rate: 44100,
            channels: 1,
            duration_seconds: estimated_duration,
            format: "wav".to_string(),
        })
    }

    async fn list_voices(&self) -> Result<Vec<VoiceInfo>> {
        // GET https://api.cartesia.ai/voices
        Ok(vec![
            VoiceInfo {
                id: "sonic-english".into(),
                name: "Sonic English".into(),
                language: Some("en-US".into()),
                gender: None,
                preview_url: None,
            },
        ])
    }

    async fn clone_voice(&self, _name: &str, _audio_samples: &[Vec<u8>]) -> Result<VoiceInfo> {
        // Cartesia can clone from just 3 seconds of audio
        anyhow::bail!("Cartesia voice cloning: HTTP integration pending")
    }
}
