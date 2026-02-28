//! ElevenLabs TTS Client
//!
//! Premium text-to-speech using ElevenLabs API.

use super::{AudioFormat, ElevenLabsConfig, SynthesizedAudio, TtsVoice, VoiceGender};
use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};

/// ElevenLabs API client
#[derive(Debug, Clone)]
pub struct ElevenLabsClient {
    config: ElevenLabsConfig,
    base_url: String,
}

/// ElevenLabs voice settings
#[derive(Debug, Clone, Serialize)]
struct VoiceSettings {
    stability: f32,
    similarity_boost: f32,
    style: f32,
    use_speaker_boost: bool,
}

/// ElevenLabs synthesis request
#[derive(Debug, Clone, Serialize)]
struct SynthesisRequest {
    text: String,
    model_id: String,
    voice_settings: VoiceSettings,
}

/// ElevenLabs voice response
#[derive(Debug, Clone, Deserialize)]
struct VoiceResponse {
    voices: Vec<ElevenLabsVoice>,
}

/// ElevenLabs voice info
#[derive(Debug, Clone, Deserialize)]
struct ElevenLabsVoice {
    voice_id: String,
    name: String,
    labels: Option<VoiceLabels>,
    description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct VoiceLabels {
    gender: Option<String>,
    accent: Option<String>,
    age: Option<String>,
}

impl ElevenLabsClient {
    /// API base URL
    const API_BASE: &'static str = "https://api.elevenlabs.io/v1";

    /// Create a new ElevenLabs client
    pub fn new(config: &ElevenLabsConfig) -> Self {
        Self {
            config: config.clone(),
            base_url: Self::API_BASE.to_string(),
        }
    }

    /// Check if client is configured
    pub fn is_configured(&self) -> bool {
        !self.config.api_key.is_empty()
    }

    /// Synthesize text to speech
    pub async fn synthesize(&self, text: &str) -> Result<SynthesizedAudio> {
        self.synthesize_with_voice(text, &self.config.voice_id).await
    }

    /// Synthesize with a specific voice
    pub async fn synthesize_with_voice(&self, text: &str, voice_id: &str) -> Result<SynthesizedAudio> {
        if !self.is_configured() {
            return Err(DrivenError::Config("ElevenLabs API key not configured".into()));
        }

        let url = format!(
            "{}/text-to-speech/{}?output_format=mp3_44100_128",
            self.base_url, voice_id
        );

        let request = SynthesisRequest {
            text: text.to_string(),
            model_id: self.config.model_id.clone(),
            voice_settings: VoiceSettings {
                stability: self.config.stability,
                similarity_boost: self.config.similarity_boost,
                style: self.config.style,
                use_speaker_boost: true,
            },
        };

        // Make HTTP request using reqwest
        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("xi-api-key", &self.config.api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!(
                "ElevenLabs API error ({}): {}",
                status, error_text
            )));
        }

        let audio_data = response
            .bytes()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        Ok(SynthesizedAudio {
            data: audio_data.to_vec(),
            format: AudioFormat::Mp3,
            duration_ms: None, // Would need to parse audio to determine
            sample_rate: Some(44100),
        })
    }

    /// List available voices
    pub async fn list_voices(&self) -> Result<Vec<TtsVoice>> {
        if !self.is_configured() {
            return Err(DrivenError::Config("ElevenLabs API key not configured".into()));
        }

        let url = format!("{}/voices", self.base_url);

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("xi-api-key", &self.config.api_key)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Failed to fetch voices".into()));
        }

        let voice_response: VoiceResponse = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        Ok(voice_response
            .voices
            .into_iter()
            .map(|v| TtsVoice {
                id: v.voice_id,
                name: v.name,
                language: "en".to_string(), // ElevenLabs supports multilingual
                gender: v.labels.and_then(|l| l.gender).and_then(|g| match g.to_lowercase().as_str() {
                    "male" => Some(VoiceGender::Male),
                    "female" => Some(VoiceGender::Female),
                    _ => Some(VoiceGender::Neutral),
                }),
                description: v.description,
            })
            .collect())
    }

    /// Get voice info by ID
    pub async fn get_voice(&self, voice_id: &str) -> Result<TtsVoice> {
        let voices = self.list_voices().await?;
        voices
            .into_iter()
            .find(|v| v.id == voice_id)
            .ok_or_else(|| DrivenError::NotFound(format!("Voice {} not found", voice_id)))
    }

    /// Get user subscription info
    pub async fn get_subscription(&self) -> Result<SubscriptionInfo> {
        if !self.is_configured() {
            return Err(DrivenError::Config("ElevenLabs API key not configured".into()));
        }

        let url = format!("{}/user/subscription", self.base_url);

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .header("xi-api-key", &self.config.api_key)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Failed to fetch subscription".into()));
        }

        response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))
    }
}

/// ElevenLabs subscription info
#[derive(Debug, Clone, Deserialize)]
pub struct SubscriptionInfo {
    /// Characters used this period
    pub character_count: u64,
    /// Character limit
    pub character_limit: u64,
    /// Subscription tier
    pub tier: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let config = ElevenLabsConfig {
            api_key: "test_key".to_string(),
            voice_id: "test_voice".to_string(),
            model_id: "eleven_multilingual_v2".to_string(),
            stability: 0.5,
            similarity_boost: 0.75,
            style: 0.0,
            speed: 1.0,
        };
        let client = ElevenLabsClient::new(&config);
        assert!(client.is_configured());
    }

    #[test]
    fn test_unconfigured_client() {
        let config = ElevenLabsConfig {
            api_key: String::new(),
            voice_id: String::new(),
            model_id: String::new(),
            stability: 0.5,
            similarity_boost: 0.75,
            style: 0.0,
            speed: 1.0,
        };
        let client = ElevenLabsClient::new(&config);
        assert!(!client.is_configured());
    }
}
