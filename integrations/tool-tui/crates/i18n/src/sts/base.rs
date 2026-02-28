//! Base trait for STT providers

use crate::error::Result;
use async_trait::async_trait;
use std::path::Path;

/// Speech-to-text trait that all STT providers must implement
#[async_trait]
pub trait SpeechToText: Send + Sync {
    /// Transcribe audio from file
    async fn transcribe_file(&self, path: &Path) -> Result<String>;

    /// Transcribe audio from raw samples (f32, 16kHz, mono)
    async fn transcribe_samples(&self, samples: &[f32]) -> Result<String>;

    /// Get supported languages
    fn get_supported_languages(&self) -> Vec<&'static str>;

    /// Check if a language is supported
    fn is_language_supported(&self, language: &str) -> bool {
        self.get_supported_languages().contains(&language)
    }
}
