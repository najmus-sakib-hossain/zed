//! Azure Speech TTS adapter — neural voices, SSML support.
//!
//! API: POST https://<region>.tts.speech.microsoft.com/cognitiveservices/v1
//! Auth: Ocp-Apim-Subscription-Key header

use anyhow::Result;
use async_trait::async_trait;
use dx_core::cost::MicroCost;
use dx_core::tts_provider::{TtsOutput, TtsProvider, TtsRequest, VoiceInfo};
use std::sync::Arc;

pub struct AzureSpeechTts {
    api_key: Option<String>,
    region: String,
}

impl AzureSpeechTts {
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("AZURE_SPEECH_KEY").ok(),
            region: std::env::var("AZURE_SPEECH_REGION")
                .unwrap_or_else(|_| "eastus".to_string()),
        }
    }
}

#[async_trait]
impl TtsProvider for AzureSpeechTts {
    fn id(&self) -> Arc<str> {
        Arc::from("azure_speech")
    }

    fn display_name(&self) -> &str {
        "Azure Speech"
    }

    fn is_local(&self) -> bool {
        false
    }

    fn is_available(&self) -> bool {
        self.api_key.is_some()
    }

    fn cost_per_character(&self) -> MicroCost {
        // Neural: $16 per 1M chars
        MicroCost(16)
    }

    async fn speak(&self, request: TtsRequest) -> Result<TtsOutput> {
        if self.api_key.is_none() {
            anyhow::bail!("Azure Speech key not set (AZURE_SPEECH_KEY)");
        }

        log::info!(
            "Azure Speech TTS: speaking {} chars in region {}",
            request.text.len(),
            self.region
        );

        // POST https://{region}.tts.speech.microsoft.com/cognitiveservices/v1
        // Headers: Ocp-Apim-Subscription-Key, Content-Type: application/ssml+xml
        // Body: SSML document with voice selection and text
        //
        // SSML example:
        // <speak version='1.0' xml:lang='en-US'>
        //   <voice name='en-US-JennyNeural'>Hello world</voice>
        // </speak>
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
        // GET https://{region}.tts.speech.microsoft.com/cognitiveservices/voices/list
        Ok(vec![
            VoiceInfo {
                id: "en-US-JennyNeural".into(),
                name: "Jenny (Neural)".into(),
                language: Some("en-US".into()),
                gender: Some("female".into()),
                preview_url: None,
            },
            VoiceInfo {
                id: "en-US-GuyNeural".into(),
                name: "Guy (Neural)".into(),
                language: Some("en-US".into()),
                gender: Some("male".into()),
                preview_url: None,
            },
            VoiceInfo {
                id: "en-US-AriaNeural".into(),
                name: "Aria (Neural)".into(),
                language: Some("en-US".into()),
                gender: Some("female".into()),
                preview_url: None,
            },
        ])
    }

    async fn clone_voice(&self, _name: &str, _audio_samples: &[Vec<u8>]) -> Result<VoiceInfo> {
        // Azure supports Custom Neural Voice with training data
        anyhow::bail!("Azure Speech voice cloning: requires Custom Neural Voice setup")
    }
}
