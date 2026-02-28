//! Unified LLM Provider trait — Universe A (Language Intelligence).
//!
//! Every LLM provider (100+) implements this trait. DX abstracts all differences
//! behind a single interface with cost tracking, rate limiting, and fallback chains.

use crate::cost::{MicroCost, TokenPricing};
use anyhow::Result;
use futures::stream::BoxStream;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Unique identifier for an LLM provider.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LlmProviderId(pub Arc<str>);

impl LlmProviderId {
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for LlmProviderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Provider tier classification for routing and fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LlmProviderTier {
    /// Tier 1: Native adapters with full SDK-level implementation.
    /// OpenAI, Anthropic, Google, AWS Bedrock, Azure, Ollama.
    Native,
    /// Tier 2: Named adapters handling provider-specific quirks.
    /// Mistral, Cohere, DeepSeek, xAI, Groq, etc.
    Named,
    /// Tier 3: OpenAI-compatible generic adapter (40+ providers).
    OpenAiCompatible,
    /// Tier 4: Aggregator multipliers (each = 100+ models).
    /// OpenRouter, Cloudflare AI Gateway, etc.
    Aggregator,
    /// Tier 5: Local models (offline, unlimited, free).
    /// Ollama, LM Studio, llama.cpp, Candle-native.
    Local,
}

/// A model available from a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmModelInfo {
    pub id: String,
    pub name: String,
    pub provider_id: LlmProviderId,
    pub context_window: usize,
    pub max_output_tokens: Option<usize>,
    pub pricing: Option<TokenPricing>,
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub supports_vision: bool,
}

/// Message role in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: LlmRole,
    pub content: String,
    /// Optional image data (base64) for vision models.
    pub images: Vec<String>,
}

/// Request to an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub model: String,
    pub messages: Vec<LlmMessage>,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub stop_sequences: Vec<String>,
    pub stream: bool,
}

/// Response from an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub model: String,
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub cost: MicroCost,
    pub finish_reason: Option<String>,
}

/// A streaming chunk from an LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmStreamChunk {
    pub delta: String,
    pub finish_reason: Option<String>,
}

/// Embedding request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub model: String,
    pub inputs: Vec<String>,
}

/// Embedding response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub embeddings: Vec<Vec<f32>>,
    pub model: String,
    pub input_tokens: usize,
    pub cost: MicroCost,
}

/// The core trait every LLM provider must implement.
///
/// This is the heart of Universe A — the unified interface for 100+ LLM providers.
#[async_trait::async_trait]
pub trait LlmProvider: Send + Sync {
    /// Unique provider identifier.
    fn id(&self) -> &LlmProviderId;

    /// Human-readable provider name.
    fn name(&self) -> &str;

    /// Provider tier classification.
    fn tier(&self) -> LlmProviderTier;

    /// Whether this provider is currently healthy/available.
    fn is_available(&self) -> bool;

    /// List available models from this provider.
    async fn list_models(&self) -> Result<Vec<LlmModelInfo>>;

    /// Send a completion request and get a full response.
    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse>;

    /// Send a streaming completion request.
    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>>;

    /// Generate embeddings for the given inputs.
    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse>;

    /// Get token pricing for a specific model.
    fn pricing(&self, model: &str) -> Option<TokenPricing>;
}

/// A fallback chain of LLM providers — tries each in order until one succeeds.
pub struct LlmFallbackChain {
    pub providers: Vec<Arc<dyn LlmProvider>>,
}

impl LlmFallbackChain {
    pub fn new(providers: Vec<Arc<dyn LlmProvider>>) -> Self {
        Self { providers }
    }

    pub async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let mut last_error = None;
        for provider in &self.providers {
            if !provider.is_available() {
                continue;
            }
            match provider.complete(request).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    log::warn!(
                        "LLM provider {} failed, trying next: {:?}",
                        provider.name(),
                        e
                    );
                    last_error = Some(e);
                }
            }
        }
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("No available LLM providers in fallback chain")))
    }
}

/// Configuration for the OpenAI-compatible generic adapter (Tier 3).
/// One adapter, 40+ providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiCompatibleConfig {
    pub provider_name: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub default_model: Option<String>,
    pub custom_headers: std::collections::HashMap<String, String>,
}

/// Well-known OpenAI-compatible providers with pre-configured base URLs.
pub fn known_openai_compatible_providers() -> Vec<OpenAiCompatibleConfig> {
    vec![
        OpenAiCompatibleConfig {
            provider_name: "Cerebras".into(),
            base_url: "https://api.cerebras.ai/v1".into(),
            api_key: None,
            default_model: Some("llama3.1-70b".into()),
            custom_headers: Default::default(),
        },
        OpenAiCompatibleConfig {
            provider_name: "Perplexity".into(),
            base_url: "https://api.perplexity.ai".into(),
            api_key: None,
            default_model: Some("llama-3.1-sonar-large-128k-online".into()),
            custom_headers: Default::default(),
        },
        OpenAiCompatibleConfig {
            provider_name: "Venice AI".into(),
            base_url: "https://api.venice.ai/api/v1".into(),
            api_key: None,
            default_model: None,
            custom_headers: Default::default(),
        },
        OpenAiCompatibleConfig {
            provider_name: "Deep Infra".into(),
            base_url: "https://api.deepinfra.com/v1/openai".into(),
            api_key: None,
            default_model: None,
            custom_headers: Default::default(),
        },
        OpenAiCompatibleConfig {
            provider_name: "SiliconFlow".into(),
            base_url: "https://api.siliconflow.cn/v1".into(),
            api_key: None,
            default_model: None,
            custom_headers: Default::default(),
        },
        OpenAiCompatibleConfig {
            provider_name: "Nebius".into(),
            base_url: "https://api.studio.nebius.ai/v1".into(),
            api_key: None,
            default_model: None,
            custom_headers: Default::default(),
        },
    ]
}
