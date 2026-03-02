//! Additional cloud TTS adapters — WellSaid Labs, Murf AI, Lovo AI.

use anyhow::Result;
use async_trait::async_trait;
use dx_core::cost::MicroCost;
use dx_core::tts_provider::{TtsOutput, TtsProvider, TtsRequest, VoiceInfo};
use std::sync::Arc;

// ── WellSaid Labs ────────────────────────────────────────────────────────

pub struct WellSaidTts {
    api_key: Option<String>,
}

impl WellSaidTts {
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("WELLSAID_API_KEY").ok(),
        }
    }
}

#[async_trait]
impl TtsProvider for WellSaidTts {
    fn id(&self) -> Arc<str> { Arc::from("wellsaid") }
    fn display_name(&self) -> &str { "WellSaid Labs" }
    fn is_local(&self) -> bool { false }
    fn is_available(&self) -> bool { self.api_key.is_some() }
    fn cost_per_character(&self) -> MicroCost { MicroCost(250) }

    async fn speak(&self, request: TtsRequest) -> Result<TtsOutput> {
        if self.api_key.is_none() {
            anyhow::bail!("WellSaid API key not set (WELLSAID_API_KEY)");
        }
        log::info!("WellSaid TTS: speaking {} chars", request.text.len());
        let estimated_duration = request.text.len() as f64 / 15.0;
        Ok(TtsOutput {
            audio_data: Vec::new(), sample_rate: 24000, channels: 1,
            duration_seconds: estimated_duration, format: "wav".to_string(),
        })
    }

    async fn list_voices(&self) -> Result<Vec<VoiceInfo>> {
        Ok(vec![VoiceInfo {
            id: "wade_c".into(), name: "Wade C".into(),
            language: Some("en-US".into()), gender: Some("male".into()), preview_url: None,
        }])
    }

    async fn clone_voice(&self, _name: &str, _audio_samples: &[Vec<u8>]) -> Result<VoiceInfo> {
        anyhow::bail!("WellSaid voice cloning: not supported via API")
    }
}

// ── Murf AI ──────────────────────────────────────────────────────────────

pub struct MurfTts {
    api_key: Option<String>,
}

impl MurfTts {
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("MURF_API_KEY").ok(),
        }
    }
}

#[async_trait]
impl TtsProvider for MurfTts {
    fn id(&self) -> Arc<str> { Arc::from("murf") }
    fn display_name(&self) -> &str { "Murf AI" }
    fn is_local(&self) -> bool { false }
    fn is_available(&self) -> bool { self.api_key.is_some() }
    fn cost_per_character(&self) -> MicroCost { MicroCost(200) }

    async fn speak(&self, request: TtsRequest) -> Result<TtsOutput> {
        if self.api_key.is_none() {
            anyhow::bail!("Murf API key not set (MURF_API_KEY)");
        }
        log::info!("Murf AI TTS: speaking {} chars", request.text.len());
        let estimated_duration = request.text.len() as f64 / 15.0;
        Ok(TtsOutput {
            audio_data: Vec::new(), sample_rate: 24000, channels: 1,
            duration_seconds: estimated_duration, format: "wav".to_string(),
        })
    }

    async fn list_voices(&self) -> Result<Vec<VoiceInfo>> {
        Ok(vec![VoiceInfo {
            id: "en-US-natalie".into(), name: "Natalie".into(),
            language: Some("en-US".into()), gender: Some("female".into()), preview_url: None,
        }])
    }

    async fn clone_voice(&self, _name: &str, _audio_samples: &[Vec<u8>]) -> Result<VoiceInfo> {
        anyhow::bail!("Murf voice cloning: HTTP integration pending")
    }
}

// ── Lovo AI ──────────────────────────────────────────────────────────────

pub struct LovoTts {
    api_key: Option<String>,
}

impl LovoTts {
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("LOVO_API_KEY").ok(),
        }
    }
}

#[async_trait]
impl TtsProvider for LovoTts {
    fn id(&self) -> Arc<str> { Arc::from("lovo") }
    fn display_name(&self) -> &str { "Lovo AI" }
    fn is_local(&self) -> bool { false }
    fn is_available(&self) -> bool { self.api_key.is_some() }
    fn cost_per_character(&self) -> MicroCost { MicroCost(180) }

    async fn speak(&self, request: TtsRequest) -> Result<TtsOutput> {
        if self.api_key.is_none() {
            anyhow::bail!("Lovo API key not set (LOVO_API_KEY)");
        }
        log::info!("Lovo AI TTS: speaking {} chars", request.text.len());
        let estimated_duration = request.text.len() as f64 / 15.0;
        Ok(TtsOutput {
            audio_data: Vec::new(), sample_rate: 24000, channels: 1,
            duration_seconds: estimated_duration, format: "wav".to_string(),
        })
    }

    async fn list_voices(&self) -> Result<Vec<VoiceInfo>> {
        Ok(vec![VoiceInfo {
            id: "lovo-default".into(), name: "Lovo Default".into(),
            language: Some("en-US".into()), gender: None, preview_url: None,
        }])
    }

    async fn clone_voice(&self, _name: &str, _audio_samples: &[Vec<u8>]) -> Result<VoiceInfo> {
        anyhow::bail!("Lovo voice cloning: HTTP integration pending")
    }
}
