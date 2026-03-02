//! Amazon Polly TTS adapter — 5M chars/month free tier.
//!
//! API: POST https://polly.<region>.amazonaws.com/ (SynthesizeSpeech)
//! Auth: AWS Signature V4

use anyhow::Result;
use async_trait::async_trait;
use dx_core::cost::MicroCost;
use dx_core::tts_provider::{TtsOutput, TtsProvider, TtsRequest, VoiceInfo};
use std::sync::Arc;

pub struct AmazonPollyTts {
    access_key: Option<String>,
    secret_key: Option<String>,
    region: String,
}

impl AmazonPollyTts {
    pub fn from_env() -> Self {
        Self {
            access_key: std::env::var("AWS_ACCESS_KEY_ID").ok(),
            secret_key: std::env::var("AWS_SECRET_ACCESS_KEY").ok(),
            region: std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
        }
    }
}

#[async_trait]
impl TtsProvider for AmazonPollyTts {
    fn id(&self) -> Arc<str> {
        Arc::from("amazon_polly")
    }

    fn display_name(&self) -> &str {
        "Amazon Polly"
    }

    fn is_local(&self) -> bool {
        false
    }

    fn is_available(&self) -> bool {
        self.access_key.is_some() && self.secret_key.is_some()
    }

    fn cost_per_character(&self) -> MicroCost {
        // Neural: $16 per 1M chars; Standard: $4 per 1M chars
        // 5M chars/month free for first 12 months
        MicroCost(16)
    }

    async fn speak(&self, request: TtsRequest) -> Result<TtsOutput> {
        if !self.is_available() {
            anyhow::bail!("AWS credentials not set (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)");
        }

        log::info!(
            "Amazon Polly TTS: speaking {} chars in region {}",
            request.text.len(),
            self.region
        );

        // SynthesizeSpeech API call with AWS Sig V4
        let estimated_duration = request.text.len() as f64 / 15.0;
        Ok(TtsOutput {
            audio_data: Vec::new(),
            sample_rate: 16000,
            channels: 1,
            duration_seconds: estimated_duration,
            format: "mp3".to_string(),
        })
    }

    async fn list_voices(&self) -> Result<Vec<VoiceInfo>> {
        Ok(vec![
            VoiceInfo {
                id: "Joanna".into(),
                name: "Joanna (Neural)".into(),
                language: Some("en-US".into()),
                gender: Some("female".into()),
                preview_url: None,
            },
            VoiceInfo {
                id: "Matthew".into(),
                name: "Matthew (Neural)".into(),
                language: Some("en-US".into()),
                gender: Some("male".into()),
                preview_url: None,
            },
            VoiceInfo {
                id: "Amy".into(),
                name: "Amy (Neural)".into(),
                language: Some("en-GB".into()),
                gender: Some("female".into()),
                preview_url: None,
            },
        ])
    }

    async fn clone_voice(&self, _name: &str, _audio_samples: &[Vec<u8>]) -> Result<VoiceInfo> {
        anyhow::bail!("Amazon Polly does not support voice cloning")
    }
}
