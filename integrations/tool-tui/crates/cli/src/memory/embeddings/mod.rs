//! Embedding Providers Module
//!
//! Pluggable embedding providers for generating vector representations
//! of text. Includes OpenAI API provider and local ONNX-based provider.

pub mod legacy;
pub mod local;
pub mod openai;

// Re-export the original EmbeddingGenerator for backwards compatibility
pub use legacy::EmbeddingGenerator;

use async_trait::async_trait;

use super::MemoryError;

/// Trait for embedding providers
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embedding for a single text
    async fn embed(&self, text: &str) -> Result<Vec<f32>, MemoryError>;

    /// Generate embeddings for multiple texts (batch)
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, MemoryError>;

    /// Get the embedding dimension
    fn dimension(&self) -> usize;

    /// Get the provider name
    fn name(&self) -> &str;

    /// Get the model name
    fn model(&self) -> &str;

    /// Maximum input tokens/characters
    fn max_input_length(&self) -> usize;
}
