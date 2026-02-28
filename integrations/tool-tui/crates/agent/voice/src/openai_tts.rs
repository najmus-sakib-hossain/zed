//! OpenAI TTS provider

use async_trait::async_trait;
use bytes::Bytes;
use reqwest::Client;

use super::{AudioOutputFormat, TtsProvider, VoiceConfig, VoiceError, VoiceInfo};

const OPENAI_TTS_URL: &str = "https://api.openai.com/v1/audio/speech";

/// OpenAI TTS provider
pub struct OpenAiTtsProvider {
    client: Client,
    api_key: String,
}

impl OpenAiTtsProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    pub fn from_env() -> Result<Self, VoiceError> {
        let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| VoiceError::AuthError)?;
        Ok(Self::new(api_key))
    }
}

#[async_trait]
impl TtsProvider for OpenAiTtsProvider {
    fn name(&self) -> &str {
        "openai"
    }

    async fn synthesize(&self, text: &str, config: &VoiceConfig) -> Result<Bytes, VoiceError> {
        let format = match config.output_format {
            AudioOutputFormat::Mp3 => "mp3",
            AudioOutputFormat::Wav => "wav",
            AudioOutputFormat::Ogg => "opus",
            AudioOutputFormat::Pcm => "pcm",
        };

        let voice = if config.voice_id == "default" {
            "alloy"
        } else {
            &config.voice_id
        };

        let body = serde_json::json!({
            "model": config.model_id.as_deref().unwrap_or("tts-1"),
            "input": text,
            "voice": voice,
            "response_format": format,
        });

        let resp = self
            .client
            .post(OPENAI_TTS_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        if resp.status().as_u16() == 429 {
            return Err(VoiceError::RateLimited);
        }

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(VoiceError::Provider(format!("OpenAI TTS error: {}", text)));
        }

        let bytes = resp.bytes().await?;
        Ok(bytes)
    }

    async fn list_voices(&self) -> Result<Vec<VoiceInfo>, VoiceError> {
        // OpenAI has fixed voices
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

    async fn health_check(&self) -> Result<bool, VoiceError> {
        let resp = self
            .client
            .get("https://api.openai.com/v1/models")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;
        Ok(resp.status().is_success())
    }
}
