//! Google AI adapter — wraps the existing `google_ai` crate for the DX `LlmProvider` trait.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

/// Google AI LLM provider adapter — Tier 1 (Native).
///
/// Wraps the existing `google_ai` crate. Supports Gemini 2.5, Gemini 2.0 Flash, etc.
pub struct GoogleAiLlmProvider {
    id: LlmProviderId,
    api_key: String,
    available: bool,
}

impl GoogleAiLlmProvider {
    /// Create from environment variable `GOOGLE_AI_API_KEY` or `GEMINI_API_KEY`.
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("GOOGLE_AI_API_KEY")
            .or_else(|_| std::env::var("GEMINI_API_KEY"))
            .ok()?;
        Some(Self {
            id: LlmProviderId::new("google-ai"),
            api_key,
            available: true,
        })
    }

    /// Create with explicit API key.
    pub fn new(api_key: String) -> Self {
        Self {
            id: LlmProviderId::new("google-ai"),
            api_key,
            available: true,
        }
    }

    fn models_list() -> Vec<LlmModelInfo> {
        vec![
            LlmModelInfo {
                id: "gemini-2.5-pro".to_string(),
                name: "Gemini 2.5 Pro".to_string(),
                provider_id: LlmProviderId::new("google-ai"),
                context_window: 1_000_000,
                max_output_tokens: Some(65_536),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(1.25),
                    output_per_million: MicroCost::from_dollars(10.00),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.31)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "gemini-2.5-flash".to_string(),
                name: "Gemini 2.5 Flash".to_string(),
                provider_id: LlmProviderId::new("google-ai"),
                context_window: 1_000_000,
                max_output_tokens: Some(65_536),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.15),
                    output_per_million: MicroCost::from_dollars(0.60),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.0375)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "gemini-2.0-flash".to_string(),
                name: "Gemini 2.0 Flash".to_string(),
                provider_id: LlmProviderId::new("google-ai"),
                context_window: 1_000_000,
                max_output_tokens: Some(8_192),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.10),
                    output_per_million: MicroCost::from_dollars(0.40),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.025)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
        ]
    }
}

#[async_trait::async_trait]
impl LlmProvider for GoogleAiLlmProvider {
    fn id(&self) -> &LlmProviderId {
        &self.id
    }

    fn name(&self) -> &str {
        "Google AI"
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
            "Google AI complete: model={}, messages={}",
            request.model,
            request.messages.len()
        );
        Err(anyhow::anyhow!(
            "Google AI adapter: HTTP bridge not yet wired to `google_ai` crate"
        ))
    }

    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        let _ = request;
        Err(anyhow::anyhow!("Google AI adapter: streaming bridge not yet wired"))
    }

    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        let _ = &self.api_key;
        let _ = request;
        Err(anyhow::anyhow!("Google AI adapter: embedding bridge not yet wired"))
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        Self::models_list()
            .into_iter()
            .find(|m| m.id == model)
            .and_then(|m| m.pricing)
    }
}
