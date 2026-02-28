//! OpenAI TTS Client
//!
//! Text-to-speech using OpenAI's TTS API (fallback provider).

use super::{AudioFormat, OpenAiTtsConfig, SynthesizedAudio, TtsVoice, VoiceGender};
use crate::error::{DrivenError, Result};
use serde::Serialize;

/// OpenAI TTS client
#[derive(Debug, Clone)]
pub struct OpenAiTtsClient {
    config: OpenAiTtsConfig,
    base_url: String,
}

/// OpenAI TTS request
#[derive(Debug, Clone, Serialize)]
struct TtsRequest {
    model: String,
    input: String,
    voice: String,
    response_format: String,
    speed: f32,
}

impl OpenAiTtsClient {
    /// API base URL
    const API_BASE: &'static str = "https://api.openai.com/v1";

    /// Available OpenAI voices
    const VOICES: &'static [(&'static str, &'static str, VoiceGender)] = &[
        ("alloy", "Alloy - Neutral and balanced", VoiceGender::Neutral),
        ("echo", "Echo - Warm and engaging", VoiceGender::Male),
        ("fable", "Fable - British and expressive", VoiceGender::Neutral),
        ("onyx", "Onyx - Deep and authoritative", VoiceGender::Male),
        ("nova", "Nova - Friendly and upbeat", VoiceGender::Female),
        ("shimmer", "Shimmer - Clear and pleasant", VoiceGender::Female),
    ];

    /// Create a new OpenAI TTS client
    pub fn new(config: &OpenAiTtsConfig) -> Self {
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
        self.synthesize_with_voice(text, &self.config.voice).await
    }

    /// Synthesize with a specific voice
    pub async fn synthesize_with_voice(&self, text: &str, voice: &str) -> Result<SynthesizedAudio> {
        if !self.is_configured() {
            return Err(DrivenError::Config("OpenAI API key not configured".into()));
        }

        let url = format!("{}/audio/speech", self.base_url);

        let request = TtsRequest {
            model: self.config.model.clone(),
            input: text.to_string(),
            voice: voice.to_string(),
            response_format: self.config.response_format.clone(),
            speed: self.config.speed.clamp(0.25, 4.0),
        };

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!(
                "OpenAI TTS API error ({}): {}",
                status, error_text
            )));
        }

        let audio_data = response
            .bytes()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        let format = match self.config.response_format.as_str() {
            "mp3" => AudioFormat::Mp3,
            "opus" => AudioFormat::Opus,
            "aac" => AudioFormat::Aac,
            "flac" => AudioFormat::Flac,
            _ => AudioFormat::Mp3,
        };

        Ok(SynthesizedAudio {
            data: audio_data.to_vec(),
            format,
            duration_ms: None,
            sample_rate: None,
        })
    }

    /// Get available voices
    pub fn available_voices() -> Vec<TtsVoice> {
        Self::VOICES
            .iter()
            .map(|(id, desc, gender)| TtsVoice {
                id: id.to_string(),
                name: id.to_string(),
                language: "en".to_string(),
                gender: Some(*gender),
                description: Some(desc.to_string()),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_available_voices() {
        let voices = OpenAiTtsClient::available_voices();
        assert_eq!(voices.len(), 6);
        assert!(voices.iter().any(|v| v.id == "alloy"));
    }
}
