//! Anthropic adapter — wraps the existing `anthropic` crate for the DX `LlmProvider` trait.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

/// Anthropic LLM provider adapter — Tier 1 (Native).
///
/// Wraps the existing `anthropic` crate. Supports Claude 4 Opus, Claude 4 Sonnet,
/// Claude 3.5 Haiku, etc.
pub struct AnthropicLlmProvider {
    id: LlmProviderId,
    api_key: String,
    available: bool,
}

impl AnthropicLlmProvider {
    /// Create from environment variable `ANTHROPIC_API_KEY`.
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY").ok()?;
        Some(Self {
            id: LlmProviderId::new("anthropic"),
            api_key,
            available: true,
        })
    }

    /// Create with explicit API key.
    pub fn new(api_key: String) -> Self {
        Self {
            id: LlmProviderId::new("anthropic"),
            api_key,
            available: true,
        }
    }

    fn models_list() -> Vec<LlmModelInfo> {
        vec![
            LlmModelInfo {
                id: "claude-sonnet-4-20250514".to_string(),
                name: "Claude Sonnet 4".to_string(),
                provider_id: LlmProviderId::new("anthropic"),
                context_window: 200_000,
                max_output_tokens: Some(64_000),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(3.00),
                    output_per_million: MicroCost::from_dollars(15.00),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.30)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "claude-opus-4-20250514".to_string(),
                name: "Claude Opus 4".to_string(),
                provider_id: LlmProviderId::new("anthropic"),
                context_window: 200_000,
                max_output_tokens: Some(32_000),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(15.00),
                    output_per_million: MicroCost::from_dollars(75.00),
                    cached_input_per_million: Some(MicroCost::from_dollars(1.50)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "claude-3-5-haiku-20241022".to_string(),
                name: "Claude 3.5 Haiku".to_string(),
                provider_id: LlmProviderId::new("anthropic"),
                context_window: 200_000,
                max_output_tokens: Some(8_192),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.80),
                    output_per_million: MicroCost::from_dollars(4.00),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.08)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
        ]
    }
}

#[async_trait::async_trait]
impl LlmProvider for AnthropicLlmProvider {
    fn id(&self) -> &LlmProviderId {
        &self.id
    }

    fn name(&self) -> &str {
        "Anthropic"
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
        let _ = &self.api_key;
        log::debug!(
            "Anthropic complete: model={}, messages={}",
            request.model,
            request.messages.len()
        );
        Err(anyhow::anyhow!(
            "Anthropic adapter: HTTP bridge not yet wired to `anthropic` crate"
        ))
    }

    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        let _ = request;
        Err(anyhow::anyhow!("Anthropic adapter: streaming bridge not yet wired"))
    }

    async fn embed(&self, _request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        Err(anyhow::anyhow!("Anthropic does not support embeddings"))
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        Self::models_list()
            .into_iter()
            .find(|m| m.id == model)
            .and_then(|m| m.pricing)
    }
}
