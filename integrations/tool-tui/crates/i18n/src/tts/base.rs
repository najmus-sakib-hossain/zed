//! Base trait for TTS providers

use crate::error::Result;
use async_trait::async_trait;
use std::path::Path;

/// Text-to-speech trait that all TTS providers must implement
#[async_trait]
pub trait TextToSpeech: Send + Sync {
    /// Convert text to speech and return audio bytes
    async fn synthesize(&self, text: &str) -> Result<Vec<u8>>;

    /// Convert text to speech and save to file
    async fn save(&self, text: &str, path: &Path) -> Result<()> {
        let audio = self.synthesize(text).await?;
        tokio::fs::write(path, audio).await?;
        Ok(())
    }

    /// Get supported languages/voices
    fn get_supported_languages(&self) -> Vec<&'static str>;

    /// Check if a language is supported
    fn is_language_supported(&self, language: &str) -> bool {
        self.get_supported_languages().contains(&language)
    }
}

/// TTS configuration
#[derive(Debug, Clone)]
pub struct TTSConfig {
    pub voice: String,
    pub rate: String,
    pub volume: String,
    pub pitch: String,
}

impl Default for TTSConfig {
    fn default() -> Self {
        Self {
            voice: "en-US-AriaNeural".to_string(),
            rate: "+0%".to_string(),
            volume: "+0%".to_string(),
            pitch: "+0Hz".to_string(),
        }
    }
}
