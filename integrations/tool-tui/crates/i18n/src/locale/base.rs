//! Base trait for all translators

use crate::error::Result;
use async_trait::async_trait;

/// Base translator trait that all translation providers must implement
#[async_trait]
pub trait Translator: Send + Sync {
    /// Translate a single text from source to target language
    async fn translate(&self, text: &str) -> Result<String>;

    /// Translate a batch of texts
    async fn translate_batch(&self, texts: &[&str]) -> Result<Vec<String>> {
        let mut results = Vec::new();
        for text in texts {
            results.push(self.translate(text).await?);
        }
        Ok(results)
    }

    /// Get supported languages
    fn get_supported_languages(&self) -> Vec<&'static str>;

    /// Check if a language is supported
    fn is_language_supported(&self, language: &str) -> bool {
        self.get_supported_languages().contains(&language)
    }

    /// Get the source language
    fn source(&self) -> &str;

    /// Get the target language
    fn target(&self) -> &str;
}
