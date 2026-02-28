//! Local Embedding Provider
//!
//! Generates embeddings locally without external API calls.
//! Uses a hash-based approach for deterministic embeddings with
//! optional support for ONNX model inference when available.
//!
//! # Models
//!
//! - `all-MiniLM-L6-v2`: 384-dimensional (default, pseudo-implementation)
//! - Future: ONNX Runtime integration for real local inference

use async_trait::async_trait;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::EmbeddingProvider;
use crate::memory::MemoryError;

/// Local embedding model variants
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalModel {
    /// all-MiniLM-L6-v2 (384 dimensions)
    MiniLmL6V2,
    /// all-MiniLM-L12-v2 (384 dimensions)
    MiniLmL12V2,
    /// bge-small-en-v1.5 (384 dimensions)
    BgeSmallEnV15,
    /// Custom model with specified dimension
    Custom { name: String, dimension: usize },
}

impl LocalModel {
    /// Get model name
    pub fn name(&self) -> &str {
        match self {
            Self::MiniLmL6V2 => "all-MiniLM-L6-v2",
            Self::MiniLmL12V2 => "all-MiniLM-L12-v2",
            Self::BgeSmallEnV15 => "bge-small-en-v1.5",
            Self::Custom { name, .. } => name,
        }
    }

    /// Get embedding dimension
    pub fn dimension(&self) -> usize {
        match self {
            Self::MiniLmL6V2 => 384,
            Self::MiniLmL12V2 => 384,
            Self::BgeSmallEnV15 => 384,
            Self::Custom { dimension, .. } => *dimension,
        }
    }
}

impl Default for LocalModel {
    fn default() -> Self {
        Self::MiniLmL6V2
    }
}

/// Local embedding provider configuration
#[derive(Debug, Clone)]
pub struct LocalEmbeddingConfig {
    /// Model to use
    pub model: LocalModel,
    /// Number of threads for inference
    pub num_threads: usize,
    /// Maximum input length (characters)
    pub max_input_length: usize,
    /// Whether to normalize output vectors
    pub normalize: bool,
}

impl Default for LocalEmbeddingConfig {
    fn default() -> Self {
        Self {
            model: LocalModel::default(),
            num_threads: num_cpus::get().min(4),
            max_input_length: 512,
            normalize: true,
        }
    }
}

/// Local embedding provider
///
/// Currently uses a deterministic hash-based pseudo-embedding approach.
/// In production, this would be replaced with ONNX Runtime or Candle inference.
pub struct LocalEmbeddingProvider {
    config: LocalEmbeddingConfig,
}

impl LocalEmbeddingProvider {
    /// Create a new local embedding provider
    pub fn new(config: LocalEmbeddingConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration
    pub fn default_provider() -> Self {
        Self::new(LocalEmbeddingConfig::default())
    }

    /// Generate a deterministic pseudo-embedding from text
    ///
    /// Uses FNV-style hashing to distribute text features across dimensions.
    /// This is NOT a real embedding model but serves as a working placeholder.
    fn pseudo_embed(&self, text: &str) -> Vec<f32> {
        let dim = self.config.model.dimension();
        let mut embedding = vec![0.0f32; dim];

        // Tokenize into words
        let words: Vec<&str> = text.split_whitespace().collect();

        if words.is_empty() {
            return embedding;
        }

        // Character n-grams for sub-word information
        let text_lower = text.to_lowercase();
        let chars: Vec<char> = text_lower.chars().collect();

        // Word-level features
        for (i, word) in words.iter().enumerate() {
            let mut hasher = DefaultHasher::new();
            word.to_lowercase().hash(&mut hasher);
            let hash = hasher.finish();

            // Distribute hash across multiple dimensions
            for j in 0..8 {
                let dim_idx = ((hash >> (j * 8)) as usize + i * 13) % dim;
                let sign = if (hash >> (j + 32)) & 1 == 0 {
                    1.0
                } else {
                    -1.0
                };
                embedding[dim_idx] += sign * 0.1;
            }
        }

        // Character 3-gram features
        for window in chars.windows(3) {
            let mut hasher = DefaultHasher::new();
            window.hash(&mut hasher);
            let hash = hasher.finish();

            let dim_idx = (hash as usize) % dim;
            let sign = if hash & 1 == 0 { 1.0 } else { -1.0 };
            embedding[dim_idx] += sign * 0.05;
        }

        // Normalize if configured
        if self.config.normalize {
            let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for x in &mut embedding {
                    *x /= norm;
                }
            }
        }

        embedding
    }
}

#[async_trait]
impl EmbeddingProvider for LocalEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, MemoryError> {
        // Truncate to max length
        let truncated = if text.len() > self.config.max_input_length {
            &text[..self.config.max_input_length]
        } else {
            text
        };

        Ok(self.pseudo_embed(truncated))
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, MemoryError> {
        let mut embeddings = Vec::with_capacity(texts.len());
        for text in texts {
            embeddings.push(self.embed(text).await?);
        }
        Ok(embeddings)
    }

    fn dimension(&self) -> usize {
        self.config.model.dimension()
    }

    fn name(&self) -> &str {
        "local"
    }

    fn model(&self) -> &str {
        self.config.model.name()
    }

    fn max_input_length(&self) -> usize {
        self.config.max_input_length
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_embedding_basic() {
        let provider = LocalEmbeddingProvider::default_provider();
        let embedding = provider.embed("Hello world").await.unwrap();

        assert_eq!(embedding.len(), 384);
    }

    #[tokio::test]
    async fn test_local_embedding_deterministic() {
        let provider = LocalEmbeddingProvider::default_provider();
        let emb1 = provider.embed("Hello world").await.unwrap();
        let emb2 = provider.embed("Hello world").await.unwrap();

        assert_eq!(emb1, emb2);
    }

    #[tokio::test]
    async fn test_local_embedding_different_texts() {
        let provider = LocalEmbeddingProvider::default_provider();
        let emb1 = provider.embed("Hello world").await.unwrap();
        let emb2 = provider.embed("Goodbye moon").await.unwrap();

        assert_ne!(emb1, emb2);
    }

    #[tokio::test]
    async fn test_local_embedding_normalized() {
        let provider = LocalEmbeddingProvider::default_provider();
        let embedding = provider.embed("Test normalization").await.unwrap();

        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01, "Embedding should be normalized, got norm={}", norm);
    }

    #[tokio::test]
    async fn test_batch_embedding() {
        let provider = LocalEmbeddingProvider::default_provider();
        let texts = vec![
            "First text".to_string(),
            "Second text".to_string(),
            "Third text".to_string(),
        ];

        let embeddings = provider.embed_batch(&texts).await.unwrap();
        assert_eq!(embeddings.len(), 3);
        for emb in &embeddings {
            assert_eq!(emb.len(), 384);
        }
    }

    #[tokio::test]
    async fn test_empty_text() {
        let provider = LocalEmbeddingProvider::default_provider();
        let embedding = provider.embed("").await.unwrap();

        assert_eq!(embedding.len(), 384);
    }

    #[test]
    fn test_model_dimensions() {
        assert_eq!(LocalModel::MiniLmL6V2.dimension(), 384);
        assert_eq!(LocalModel::BgeSmallEnV15.dimension(), 384);

        let custom = LocalModel::Custom {
            name: "custom".to_string(),
            dimension: 768,
        };
        assert_eq!(custom.dimension(), 768);
    }

    #[test]
    fn test_provider_info() {
        let provider = LocalEmbeddingProvider::default_provider();
        assert_eq!(provider.name(), "local");
        assert_eq!(provider.model(), "all-MiniLM-L6-v2");
        assert_eq!(provider.dimension(), 384);
    }
}
