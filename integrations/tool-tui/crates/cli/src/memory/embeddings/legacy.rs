//! Embedding Generator
//!
//! Local embedding generation using MiniLM or similar models.

use super::MemoryError;

/// Embedding generator
pub struct EmbeddingGenerator {
    /// Model name
    model: String,
    /// Output dimension
    dimension: usize,
}

impl EmbeddingGenerator {
    /// Create a new embedding generator
    pub fn new(model: &str, dimension: usize) -> Self {
        Self {
            model: model.to_string(),
            dimension,
        }
    }

    /// Generate embedding for text
    pub async fn generate(&self, text: &str) -> Result<Vec<f32>, MemoryError> {
        // In production, this would use a real embedding model like:
        // - sentence-transformers with ONNX runtime
        // - fastembed-rs
        // - candle for local inference

        // For now, generate a deterministic pseudo-embedding
        let embedding = self.pseudo_embedding(text);

        Ok(embedding)
    }

    /// Generate embeddings for multiple texts
    pub async fn generate_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, MemoryError> {
        let mut embeddings = Vec::with_capacity(texts.len());
        for text in texts {
            embeddings.push(self.generate(text).await?);
        }
        Ok(embeddings)
    }

    /// Pseudo-embedding for testing (not for production)
    fn pseudo_embedding(&self, text: &str) -> Vec<f32> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut embedding = vec![0.0f32; self.dimension];

        // Use hash-based approach for deterministic "embeddings"
        let words: Vec<&str> = text.split_whitespace().collect();

        for (i, word) in words.iter().enumerate() {
            let mut hasher = DefaultHasher::new();
            word.hash(&mut hasher);
            let hash = hasher.finish();

            // Distribute hash bits across embedding dimensions
            for j in 0..self.dimension {
                let bit_idx = (j + i * 7) % 64;
                let bit = ((hash >> bit_idx) & 1) as f32;
                embedding[j] += if bit == 1.0 { 0.1 } else { -0.1 };
            }
        }

        // Normalize
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut embedding {
                *x /= norm;
            }
        }

        embedding
    }

    /// Calculate cosine similarity between two embeddings
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a > 0.0 && norm_b > 0.0 {
            dot / (norm_a * norm_b)
        } else {
            0.0
        }
    }

    /// Get model info
    pub fn model_info(&self) -> ModelInfo {
        ModelInfo {
            name: self.model.clone(),
            dimension: self.dimension,
            // Approximate values for MiniLM
            max_sequence_length: 512,
            pooling_strategy: "mean".to_string(),
        }
    }
}

/// Model information
#[derive(Debug, Clone)]
pub struct ModelInfo {
    /// Model name
    pub name: String,
    /// Embedding dimension
    pub dimension: usize,
    /// Maximum input sequence length
    pub max_sequence_length: usize,
    /// Pooling strategy
    pub pooling_strategy: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_embedding() {
        let generator = EmbeddingGenerator::new("test-model", 384);

        let embedding = generator.generate("Hello world").await.unwrap();

        assert_eq!(embedding.len(), 384);
    }

    #[tokio::test]
    async fn test_deterministic() {
        let generator = EmbeddingGenerator::new("test-model", 384);

        let emb1 = generator.generate("Hello world").await.unwrap();
        let emb2 = generator.generate("Hello world").await.unwrap();

        assert_eq!(emb1, emb2);
    }

    #[tokio::test]
    async fn test_different_text() {
        let generator = EmbeddingGenerator::new("test-model", 384);

        let emb1 = generator.generate("Hello world").await.unwrap();
        let emb2 = generator.generate("Goodbye moon").await.unwrap();

        assert_ne!(emb1, emb2);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let similarity = EmbeddingGenerator::cosine_similarity(&a, &a);

        assert!((similarity - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];

        let similarity = EmbeddingGenerator::cosine_similarity(&a, &b);

        assert!(similarity.abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];

        let similarity = EmbeddingGenerator::cosine_similarity(&a, &b);

        assert!((similarity + 1.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_batch_generate() {
        let generator = EmbeddingGenerator::new("test-model", 384);

        let texts = vec![
            "Hello world".to_string(),
            "Goodbye moon".to_string(),
            "Test text".to_string(),
        ];

        let embeddings = generator.generate_batch(&texts).await.unwrap();

        assert_eq!(embeddings.len(), 3);
        for emb in &embeddings {
            assert_eq!(emb.len(), 384);
        }
    }

    #[test]
    fn test_model_info() {
        let generator = EmbeddingGenerator::new("all-MiniLM-L6-v2", 384);
        let info = generator.model_info();

        assert_eq!(info.name, "all-MiniLM-L6-v2");
        assert_eq!(info.dimension, 384);
    }
}
