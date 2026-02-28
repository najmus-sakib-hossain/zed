//! ElevenLabs TTS provider

use async_trait::async_trait;
use bytes::Bytes;
use reqwest::Client;
use serde::Deserialize;

use super::{AudioOutputFormat, TtsProvider, VoiceConfig, VoiceError, VoiceInfo};

const ELEVENLABS_API_BASE: &str = "https://api.elevenlabs.io/v1";

/// ElevenLabs TTS provider
pub struct ElevenLabsProvider {
    client: Client,
    api_key: String,
}

impl ElevenLabsProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    pub fn from_env() -> Result<Self, VoiceError> {
        let api_key = std::env::var("ELEVENLABS_API_KEY").map_err(|_| VoiceError::AuthError)?;
        Ok(Self::new(api_key))
    }
}

#[derive(Debug, Deserialize)]
struct ElevenLabsVoicesResponse {
    voices: Vec<ElevenLabsVoice>,
}

#[derive(Debug, Deserialize)]
struct ElevenLabsVoice {
    voice_id: String,
    name: String,
    #[serde(default)]
    labels: serde_json::Value,
}

#[async_trait]
impl TtsProvider for ElevenLabsProvider {
    fn name(&self) -> &str {
        "elevenlabs"
    }

    async fn synthesize(&self, text: &str, config: &VoiceConfig) -> Result<Bytes, VoiceError> {
        let output_format = match config.output_format {
            AudioOutputFormat::Mp3 => "mp3_44100_128",
            AudioOutputFormat::Pcm => "pcm_16000",
            AudioOutputFormat::Wav => "pcm_44100",
            AudioOutputFormat::Ogg => "mp3_44100_128", // Fallback to mp3
        };

        let url = format!(
            "{}/text-to-speech/{}?output_format={}",
            ELEVENLABS_API_BASE, config.voice_id, output_format
        );

        let body = serde_json::json!({
            "text": text,
            "model_id": config.model_id.as_deref().unwrap_or("eleven_multilingual_v2"),
            "voice_settings": {
                "stability": config.stability,
                "similarity_boost": config.similarity_boost,
            }
        });

        let resp = self
            .client
            .post(&url)
            .header("xi-api-key", &self.api_key)
            .json(&body)
            .send()
            .await?;

        if resp.status().as_u16() == 429 {
            return Err(VoiceError::RateLimited);
        }

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(VoiceError::Provider(format!("ElevenLabs error: {}", text)));
        }

        let bytes = resp.bytes().await?;
        Ok(bytes)
    }

    async fn list_voices(&self) -> Result<Vec<VoiceInfo>, VoiceError> {
        let resp = self
            .client
            .get(format!("{}/voices", ELEVENLABS_API_BASE))
            .header("xi-api-key", &self.api_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(VoiceError::Provider("Failed to list voices".into()));
        }

        let data: ElevenLabsVoicesResponse = resp
            .json()
            .await
            .map_err(|e| VoiceError::Provider(format!("Parse error: {}", e)))?;

        Ok(data
            .voices
            .into_iter()
            .map(|v| {
                let gender = v.labels.get("gender").and_then(|g| g.as_str()).map(String::from);
                VoiceInfo {
                    id: v.voice_id,
                    name: v.name,
                    language: None,
                    gender,
                    preview_url: None,
                }
            })
            .collect())
    }

    async fn health_check(&self) -> Result<bool, VoiceError> {
        let resp = self
            .client
            .get(format!("{}/voices", ELEVENLABS_API_BASE))
            .header("xi-api-key", &self.api_key)
            .send()
            .await?;
        Ok(resp.status().is_success())
    }
}
