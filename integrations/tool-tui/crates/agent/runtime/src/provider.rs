//! LLM Provider trait - unified interface for all providers

use async_trait::async_trait;
use futures_util::stream::BoxStream;

use crate::models::{ChatRequest, ChatResponse, ModelInfo};
use crate::streaming::StreamEvent;

/// Provider error types
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Rate limited: retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Context length exceeded: {used} > {max}")]
    ContextLengthExceeded { used: u32, max: u32 },

    #[error("Provider unavailable: {0}")]
    Unavailable(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Provider error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("Timeout after {0}ms")]
    Timeout(u64),
}

/// Provider capabilities
#[derive(Debug, Clone, Default)]
pub struct ProviderCapabilities {
    pub streaming: bool,
    pub function_calling: bool,
    pub vision: bool,
    pub embeddings: bool,
    pub multi_modal: bool,
    pub json_mode: bool,
    pub batch_api: bool,
}

/// Unified LLM provider interface
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Get provider name (e.g., "openai", "anthropic", "google")
    fn name(&self) -> &str;

    /// List available models
    async fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError>;

    /// Send a chat completion request
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError>;

    /// Send a streaming chat completion request
    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<StreamEvent, ProviderError>>, ProviderError>;

    /// Get provider capabilities
    fn capabilities(&self) -> ProviderCapabilities;

    /// Check if provider is healthy/reachable
    async fn health_check(&self) -> Result<bool, ProviderError>;

    /// Count tokens for a given text (approximate)
    fn count_tokens(&self, text: &str, model: &str) -> Result<u32, ProviderError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_error_display() {
        let err = ProviderError::RateLimited {
            retry_after_ms: 5000,
        };
        assert!(err.to_string().contains("5000"));

        let err = ProviderError::ContextLengthExceeded {
            used: 200_000,
            max: 128_000,
        };
        assert!(err.to_string().contains("200000"));
    }

    #[test]
    fn test_capabilities_default() {
        let caps = ProviderCapabilities::default();
        assert!(!caps.streaming);
        assert!(!caps.vision);
    }
}
