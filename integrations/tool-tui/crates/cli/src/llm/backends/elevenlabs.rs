//! ElevenLabs Text-to-Speech backend

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{Backend, BackendType};

const DEFAULT_BASE_URL: &str = "https://api.elevenlabs.io";
const DEFAULT_VOICE_ID: &str = "pMsXgVXv3BLzUgSXRplE"; // Adam voice
const DEFAULT_MODEL_ID: &str = "eleven_multilingual_v2";

#[derive(Debug, Clone, Serialize)]
pub struct VoiceSettings {
    pub stability: f32,
    pub similarity_boost: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_speaker_boost: Option<bool>,
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            stability: 0.5,
            similarity_boost: 0.75,
            style: Some(0.0),
            use_speaker_boost: Some(true),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct TtsRequest {
    text: String,
    model_id: String,
    voice_settings: VoiceSettings,
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    language_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Voice {
    pub voice_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct VoicesResponse {
    voices: Vec<Voice>,
}

pub struct ElevenLabsBackend {
    client: reqwest::Client,
    api_key: Option<String>,
    base_url: String,
    voice_id: String,
    model_id: String,
    voice_settings: VoiceSettings,
    output_format: String,
    seed: Option<u32>,
    language_code: Option<String>,
}

impl ElevenLabsBackend {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            api_key,
            base_url: DEFAULT_BASE_URL.to_string(),
            voice_id: DEFAULT_VOICE_ID.to_string(),
            model_id: DEFAULT_MODEL_ID.to_string(),
            voice_settings: VoiceSettings::default(),
            output_format: "mp3_44100_128".to_string(),
            seed: None,
            language_code: None,
        }
    }

    pub fn with_voice_id(mut self, voice_id: String) -> Self {
        self.voice_id = voice_id;
        self
    }

    pub fn with_model_id(mut self, model_id: String) -> Self {
        self.model_id = model_id;
        self
    }

    pub fn with_voice_settings(mut self, settings: VoiceSettings) -> Self {
        self.voice_settings = settings;
        self
    }

    pub fn with_output_format(mut self, format: String) -> Self {
        self.output_format = format;
        self
    }

    pub fn with_seed(mut self, seed: u32) -> Self {
        self.seed = Some(seed);
        self
    }

    pub fn with_language_code(mut self, code: String) -> Self {
        self.language_code = Some(code);
        self
    }

    pub fn set_api_key(&mut self, api_key: String) {
        self.api_key = Some(api_key);
    }

    pub fn has_api_key(&self) -> bool {
        self.api_key.is_some()
    }

    /// List available voices
    pub async fn list_voices(&self) -> Result<Vec<Voice>> {
        let api_key = self.api_key.as_ref().context("ElevenLabs API key not configured")?;

        let url = format!("{}/v1/voices", self.base_url);

        let response = self
            .client
            .get(&url)
            .header("xi-api-key", api_key)
            .send()
            .await
            .context("Failed to fetch voices from ElevenLabs")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("ElevenLabs API error {}: {}", status, error_text);
        }

        let result: VoicesResponse =
            response.json().await.context("Failed to parse voices response")?;

        Ok(result.voices)
    }

    /// Generate speech from text and save to file
    pub async fn text_to_speech(&self, text: &str, output_path: &PathBuf) -> Result<()> {
        let api_key = self.api_key.as_ref().context("ElevenLabs API key not configured")?;

        let url = format!(
            "{}/v1/text-to-speech/{}?output_format={}",
            self.base_url, self.voice_id, self.output_format
        );

        let request = TtsRequest {
            text: text.to_string(),
            model_id: self.model_id.clone(),
            voice_settings: self.voice_settings.clone(),
            seed: self.seed,
            language_code: self.language_code.clone(),
        };

        let response = self
            .client
            .post(&url)
            .header("xi-api-key", api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send TTS request to ElevenLabs")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("ElevenLabs TTS API error {}: {}", status, error_text);
        }

        let audio_bytes = response.bytes().await.context("Failed to read audio response")?;

        // Ensure parent directory exists
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(output_path, audio_bytes).context("Failed to write audio file")?;

        Ok(())
    }

    /// Generate speech and return audio bytes
    pub async fn text_to_speech_bytes(&self, text: &str) -> Result<Vec<u8>> {
        let api_key = self.api_key.as_ref().context("ElevenLabs API key not configured")?;

        let url = format!(
            "{}/v1/text-to-speech/{}?output_format={}",
            self.base_url, self.voice_id, self.output_format
        );

        let request = TtsRequest {
            text: text.to_string(),
            model_id: self.model_id.clone(),
            voice_settings: self.voice_settings.clone(),
            seed: self.seed,
            language_code: self.language_code.clone(),
        };

        let response = self
            .client
            .post(&url)
            .header("xi-api-key", api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send TTS request to ElevenLabs")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("ElevenLabs TTS API error {}: {}", status, error_text);
        }

        let audio_bytes = response.bytes().await.context("Failed to read audio response")?;

        Ok(audio_bytes.to_vec())
    }
}

#[async_trait]
impl Backend for ElevenLabsBackend {
    async fn initialize(&mut self) -> Result<()> {
        if self.api_key.is_none() {
            anyhow::bail!("ElevenLabs API key not configured");
        }
        Ok(())
    }

    async fn generate(&self, prompt: &str, _max_tokens: usize) -> Result<String> {
        // For TTS, we return a status message instead of generated text
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("elevenlabs_output.mp3");

        self.text_to_speech(prompt, &output_path).await?;

        Ok(format!("Audio generated successfully: {}", output_path.display()))
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        max_tokens: usize,
        callback: Box<dyn Fn(String) + Send>,
    ) -> Result<()> {
        // For TTS, streaming means generating audio and reporting progress
        callback("Generating audio...".to_string());
        let result = self.generate(prompt, max_tokens).await?;
        callback(result);
        Ok(())
    }

    fn is_available(&self) -> bool {
        self.api_key.is_some()
    }

    fn backend_type(&self) -> BackendType {
        BackendType::ElevenLabs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_settings_default() {
        let settings = VoiceSettings::default();
        assert_eq!(settings.stability, 0.5);
        assert_eq!(settings.similarity_boost, 0.75);
    }

    #[test]
    fn test_backend_creation() {
        let backend = ElevenLabsBackend::new(Some("test_key".to_string()));
        assert!(backend.has_api_key());
        assert_eq!(backend.voice_id, DEFAULT_VOICE_ID);
        assert_eq!(backend.model_id, DEFAULT_MODEL_ID);
    }

    #[test]
    fn test_builder_pattern() {
        let backend = ElevenLabsBackend::new(Some("test_key".to_string()))
            .with_voice_id("custom_voice".to_string())
            .with_model_id("custom_model".to_string())
            .with_seed(42);

        assert_eq!(backend.voice_id, "custom_voice");
        assert_eq!(backend.model_id, "custom_model");
        assert_eq!(backend.seed, Some(42));
    }
}
