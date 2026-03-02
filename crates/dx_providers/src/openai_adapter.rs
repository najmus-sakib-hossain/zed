//! OpenAI adapter — wraps the existing `open_ai` crate for the DX `LlmProvider` trait.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;
use std::sync::Arc;

/// OpenAI LLM provider adapter — Tier 1 (Native).
///
/// Wraps the existing `open_ai` crate and exposes it through the DX `LlmProvider` trait.
/// Supports: GPT-4o, GPT-4o-mini, GPT-4.5, o1, o3, etc.
pub struct OpenAiLlmProvider {
    id: LlmProviderId,
    api_key: String,
    base_url: String,
    available: bool,
}

impl OpenAiLlmProvider {
    /// Create from environment variable `OPENAI_API_KEY`.
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("OPENAI_API_KEY").ok()?;
        Some(Self {
            id: LlmProviderId::new("openai"),
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
            available: true,
        })
    }

    /// Create with explicit credentials.
    pub fn new(api_key: String, base_url: Option<String>) -> Self {
        Self {
            id: LlmProviderId::new("openai"),
            api_key,
            base_url: base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            available: true,
        }
    }

    fn models_list() -> Vec<LlmModelInfo> {
        vec![
            LlmModelInfo {
                id: "gpt-4o".to_string(),
                name: "GPT-4o".to_string(),
                provider_id: LlmProviderId::new("openai"),
                context_window: 128_000,
                max_output_tokens: Some(16_384),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(2.50),
                    output_per_million: MicroCost::from_dollars(10.00),
                    cached_input_per_million: Some(MicroCost::from_dollars(1.25)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "gpt-4o-mini".to_string(),
                name: "GPT-4o Mini".to_string(),
                provider_id: LlmProviderId::new("openai"),
                context_window: 128_000,
                max_output_tokens: Some(16_384),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.15),
                    output_per_million: MicroCost::from_dollars(0.60),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.075)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "o3-mini".to_string(),
                name: "o3-mini".to_string(),
                provider_id: LlmProviderId::new("openai"),
                context_window: 200_000,
                max_output_tokens: Some(100_000),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(1.10),
                    output_per_million: MicroCost::from_dollars(4.40),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.55)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: false,
            },
            LlmModelInfo {
                id: "gpt-4.5-preview".to_string(),
                name: "GPT-4.5 Preview".to_string(),
                provider_id: LlmProviderId::new("openai"),
                context_window: 128_000,
                max_output_tokens: Some(16_384),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(75.00),
                    output_per_million: MicroCost::from_dollars(150.00),
                    cached_input_per_million: Some(MicroCost::from_dollars(37.50)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
        ]
    }
}

#[async_trait::async_trait]
impl LlmProvider for OpenAiLlmProvider {
    fn id(&self) -> &LlmProviderId {
        &self.id
    }

    fn name(&self) -> &str {
        "OpenAI"
    }

    fn tier(&self) -> LlmProviderTier {
        LlmProviderTier::Native
    }

    fn is_available(&self) -> bool {
        self.available && !self.api_key.is_empty()
    }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(Self::models_list())
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        // Bridge to the existing open_ai crate's completion logic.
        // The actual HTTP call uses open_ai::stream_completion or similar.
        // For now, this returns a structured placeholder that the bridge layer
        // will replace with real calls once the HTTP client is wired.
        let _ = &self.base_url;
        let _ = &self.api_key;

        log::debug!("OpenAI complete: model={}, messages={}", request.model, request.messages.len());

        Err(anyhow::anyhow!(
            "OpenAI adapter: HTTP bridge not yet wired. \
             Use `provider_bridge::complete_via_http()` to connect."
        ))
    }

    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        let _ = request;
        Err(anyhow::anyhow!("OpenAI adapter: streaming bridge not yet wired"))
    }

    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        let _ = request;
        Err(anyhow::anyhow!("OpenAI adapter: embedding bridge not yet wired"))
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        Self::models_list()
            .into_iter()
            .find(|m| m.id == model)
            .and_then(|m| m.pricing)
    }
}
