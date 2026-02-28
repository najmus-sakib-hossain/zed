//! DX Agent Voice - TTS, STT, and audio streaming
//!
//! Integrates with ElevenLabs and OpenAI for text-to-speech,
//! and provides platform notification support.

pub mod elevenlabs;
pub mod notifications;
pub mod openai_tts;
pub mod platform;
pub mod vad;

use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};

/// Voice provider error
#[derive(Debug, thiserror::Error)]
pub enum VoiceError {
    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Audio format not supported: {0}")]
    UnsupportedFormat(String),

    #[error("Rate limited")]
    RateLimited,

    #[error("Authentication failed")]
    AuthError,
}

/// TTS voice selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceConfig {
    pub voice_id: String,
    pub model_id: Option<String>,
    pub stability: f32,
    pub similarity_boost: f32,
    pub output_format: AudioOutputFormat,
}

/// Audio output format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioOutputFormat {
    Mp3,
    Wav,
    Ogg,
    Pcm,
}

impl Default for VoiceConfig {
    fn default() -> Self {
        Self {
            voice_id: "default".into(),
            model_id: None,
            stability: 0.5,
            similarity_boost: 0.5,
            output_format: AudioOutputFormat::Mp3,
        }
    }
}

/// TTS provider trait
#[async_trait]
pub trait TtsProvider: Send + Sync {
    /// Get provider name
    fn name(&self) -> &str;

    /// Synthesize text to audio bytes
    async fn synthesize(&self, text: &str, config: &VoiceConfig) -> Result<Bytes, VoiceError>;

    /// List available voices
    async fn list_voices(&self) -> Result<Vec<VoiceInfo>, VoiceError>;

    /// Check if provider is available
    async fn health_check(&self) -> Result<bool, VoiceError>;
}

/// Voice information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceInfo {
    pub id: String,
    pub name: String,
    pub language: Option<String>,
    pub gender: Option<String>,
    pub preview_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_voice_config() {
        let config = VoiceConfig::default();
        assert_eq!(config.stability, 0.5);
        assert_eq!(config.voice_id, "default");
    }
}
