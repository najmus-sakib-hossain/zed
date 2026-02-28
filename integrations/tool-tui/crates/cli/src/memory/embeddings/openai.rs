//! OpenAI Embeddings Provider
//!
//! Generates embeddings using OpenAI's text-embedding API.
//! Supports text-embedding-3-small (1536d) and text-embedding-3-large (3072d).
//!
//! # Usage
//!
//! ```rust,ignore
//! let provider = OpenAiEmbeddingProvider::new("sk-...", OpenAiModel::TextEmbedding3Small);
//! let embedding = provider.embed("Hello world").await?;
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::EmbeddingProvider;
use crate::memory::MemoryError;

/// OpenAI embedding model variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenAiModel {
    /// text-embedding-3-small (1536 dimensions)
    TextEmbedding3Small,
    /// text-embedding-3-large (3072 dimensions)  
    TextEmbedding3Large,
    /// text-embedding-ada-002 (1536 dimensions, legacy)
    TextEmbeddingAda002,
}

impl OpenAiModel {
    /// Get the API model name string
    pub fn api_name(&self) -> &'static str {
        match self {
            Self::TextEmbedding3Small => "text-embedding-3-small",
            Self::TextEmbedding3Large => "text-embedding-3-large",
            Self::TextEmbeddingAda002 => "text-embedding-ada-002",
        }
    }

    /// Get the output embedding dimension
    pub fn dimension(&self) -> usize {
        match self {
            Self::TextEmbedding3Small => 1536,
            Self::TextEmbedding3Large => 3072,
            Self::TextEmbeddingAda002 => 1536,
        }
    }

    /// Maximum input tokens
    pub fn max_tokens(&self) -> usize {
        match self {
            Self::TextEmbedding3Small => 8191,
            Self::TextEmbedding3Large => 8191,
            Self::TextEmbeddingAda002 => 8191,
        }
    }
}

/// OpenAI embedding request body
#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    input: Vec<String>,
    encoding_format: String,
}

/// OpenAI embedding response
#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
    model: String,
    usage: EmbeddingUsage,
}

/// Single embedding in the response
#[derive(Debug, Deserialize)]
struct EmbeddingData {
    #[allow(dead_code)]
    index: usize,
    embedding: Vec<f32>,
}

/// Token usage stats
#[derive(Debug, Deserialize)]
struct EmbeddingUsage {
    prompt_tokens: usize,
    total_tokens: usize,
}

/// OpenAI Error response
#[derive(Debug, Deserialize)]
struct OpenAiError {
    error: OpenAiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct OpenAiErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
}

/// OpenAI embeddings provider configuration
#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    /// API key
    pub api_key: String,
    /// Model to use
    pub model: OpenAiModel,
    /// API base URL (for custom endpoints / Azure)
    pub base_url: String,
    /// Maximum batch size per request
    pub max_batch_size: usize,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Maximum retries on failure
    pub max_retries: u32,
}

impl OpenAiConfig {
    /// Create config with API key and default model
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: OpenAiModel::TextEmbedding3Small,
            base_url: "https://api.openai.com/v1".to_string(),
            max_batch_size: 100,
            timeout_secs: 30,
            max_retries: 3,
        }
    }

    /// Set the model
    pub fn with_model(mut self, model: OpenAiModel) -> Self {
        self.model = model;
        self
    }

    /// Set custom base URL (for Azure, proxies, etc.)
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

/// OpenAI embeddings provider
pub struct OpenAiEmbeddingProvider {
    config: OpenAiConfig,
    client: reqwest::Client,
}

impl OpenAiEmbeddingProvider {
    /// Create a new OpenAI embedding provider
    pub fn new(config: OpenAiConfig) -> Result<Self, MemoryError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| {
                MemoryError::EmbeddingError(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self { config, client })
    }

    /// Create provider from API key with default settings
    pub fn from_api_key(api_key: impl Into<String>) -> Result<Self, MemoryError> {
        Self::new(OpenAiConfig::new(api_key))
    }

    /// Create provider from environment variable
    pub fn from_env() -> Result<Self, MemoryError> {
        let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
            MemoryError::EmbeddingError("OPENAI_API_KEY environment variable not set".to_string())
        })?;
        Self::from_api_key(api_key)
    }

    /// Make an embedding API call
    async fn call_api(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, MemoryError> {
        let url = format!("{}/embeddings", self.config.base_url);

        let request = EmbeddingRequest {
            model: self.config.model.api_name().to_string(),
            input: texts.to_vec(),
            encoding_format: "float".to_string(),
        };

        let mut last_err = None;

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                // Exponential backoff
                let delay = std::time::Duration::from_millis(100 * 2u64.pow(attempt - 1));
                tokio::time::sleep(delay).await;
            }

            let result = self
                .client
                .post(&url)
                .header("Authorization", format!("Bearer {}", self.config.api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await;

            match result {
                Ok(response) => {
                    if response.status().is_success() {
                        let body: EmbeddingResponse = response.json().await.map_err(|e| {
                            MemoryError::EmbeddingError(format!("Failed to parse response: {}", e))
                        })?;

                        tracing::debug!(
                            "OpenAI embeddings: model={}, tokens={}",
                            body.model,
                            body.usage.total_tokens
                        );

                        // Sort by index to ensure correct order
                        let mut data = body.data;
                        data.sort_by_key(|d| d.index);

                        return Ok(data.into_iter().map(|d| d.embedding).collect());
                    } else {
                        let status = response.status();
                        let error_text = response.text().await.unwrap_or_default();

                        // Try to parse as OpenAI error
                        if let Ok(api_error) = serde_json::from_str::<OpenAiError>(&error_text) {
                            last_err = Some(MemoryError::EmbeddingError(format!(
                                "OpenAI API error ({}): {} - {}",
                                status, api_error.error.error_type, api_error.error.message
                            )));
                        } else {
                            last_err = Some(MemoryError::EmbeddingError(format!(
                                "OpenAI API error ({}): {}",
                                status, error_text
                            )));
                        }

                        // Don't retry on 4xx errors (except 429 rate limit)
                        if status.as_u16() < 500 && status.as_u16() != 429 {
                            return Err(last_err.unwrap());
                        }
                    }
                }
                Err(e) => {
                    last_err = Some(MemoryError::EmbeddingError(format!("Request failed: {}", e)));
                }
            }
        }

        Err(last_err.unwrap_or_else(|| MemoryError::EmbeddingError("Unknown error".to_string())))
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAiEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, MemoryError> {
        let results = self.call_api(&[text.to_string()]).await?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| MemoryError::EmbeddingError("Empty response from API".to_string()))
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, MemoryError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let mut all_embeddings = Vec::with_capacity(texts.len());

        // Process in batches
        for chunk in texts.chunks(self.config.max_batch_size) {
            let batch_results = self.call_api(&chunk.to_vec()).await?;
            all_embeddings.extend(batch_results);
        }

        Ok(all_embeddings)
    }

    fn dimension(&self) -> usize {
        self.config.model.dimension()
    }

    fn name(&self) -> &str {
        "openai"
    }

    fn model(&self) -> &str {
        self.config.model.api_name()
    }

    fn max_input_length(&self) -> usize {
        self.config.model.max_tokens()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_names() {
        assert_eq!(OpenAiModel::TextEmbedding3Small.api_name(), "text-embedding-3-small");
        assert_eq!(OpenAiModel::TextEmbedding3Large.api_name(), "text-embedding-3-large");
    }

    #[test]
    fn test_model_dimensions() {
        assert_eq!(OpenAiModel::TextEmbedding3Small.dimension(), 1536);
        assert_eq!(OpenAiModel::TextEmbedding3Large.dimension(), 3072);
        assert_eq!(OpenAiModel::TextEmbeddingAda002.dimension(), 1536);
    }

    #[test]
    fn test_config_creation() {
        let config = OpenAiConfig::new("test-key");
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.model, OpenAiModel::TextEmbedding3Small);
        assert_eq!(config.base_url, "https://api.openai.com/v1");
    }

    #[test]
    fn test_config_builder() {
        let config = OpenAiConfig::new("test-key")
            .with_model(OpenAiModel::TextEmbedding3Large)
            .with_base_url("https://custom.api.com/v1");

        assert_eq!(config.model, OpenAiModel::TextEmbedding3Large);
        assert_eq!(config.base_url, "https://custom.api.com/v1");
    }

    #[test]
    fn test_from_env_missing_key() {
        // Ensure key is not set for this test
        std::env::remove_var("OPENAI_API_KEY");
        let result = OpenAiEmbeddingProvider::from_env();
        assert!(result.is_err());
    }
}
