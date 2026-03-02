//! OpenRouter adapter — Tier 4 aggregator.
//!
//! OpenRouter provides a single API that proxies requests to 200+ models
//! across dozens of providers, automatically handling fallback and pricing.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

pub struct OpenRouterLlmAdapter {
    id: LlmProviderId,
    api_key: String,
    available: bool,
    /// Cached model list obtained via `/api/v1/models`.
    cached_models: parking_lot::Mutex<Option<Vec<LlmModelInfo>>>,
}

impl OpenRouterLlmAdapter {
    pub fn new(api_key: String) -> Self {
        let available = !api_key.is_empty();
        Self {
            id: LlmProviderId::new("openrouter"),
            api_key,
            available,
            cached_models: parking_lot::Mutex::new(None),
        }
    }

    /// Try to create from the standard `OPENROUTER_API_KEY` env var.
    pub fn from_env() -> Self {
        let key = std::env::var("OPENROUTER_API_KEY").unwrap_or_default();
        Self::new(key)
    }

    /// The well-known base URL for the OpenRouter API.
    pub fn base_url() -> &'static str {
        "https://openrouter.ai/api/v1"
    }
}

#[async_trait::async_trait]
impl LlmProvider for OpenRouterLlmAdapter {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "OpenRouter" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Aggregator }
    fn is_available(&self) -> bool { self.available }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        // Return cached list if we have one.
        if let Some(cached) = self.cached_models.lock().as_ref() {
            return Ok(cached.clone());
        }

        // In production, this would GET `{base_url}/models` and parse the JSON response.
        // For now, return a representative subset of popular models available via OpenRouter.
        let models = vec![
            LlmModelInfo {
                id: "openai/gpt-4o".into(),
                name: "GPT-4o (via OpenRouter)".into(),
                provider_id: self.id.clone(),
                context_window: 128_000,
                max_output_tokens: Some(16_384),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(2.50),
                    output_per_million: MicroCost::from_dollars(10.00),
                    cached_input_per_million: None,
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "anthropic/claude-sonnet-4".into(),
                name: "Claude Sonnet 4 (via OpenRouter)".into(),
                provider_id: self.id.clone(),
                context_window: 200_000,
                max_output_tokens: Some(16_384),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(3.00),
                    output_per_million: MicroCost::from_dollars(15.00),
                    cached_input_per_million: None,
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "google/gemini-2.5-pro".into(),
                name: "Gemini 2.5 Pro (via OpenRouter)".into(),
                provider_id: self.id.clone(),
                context_window: 1_000_000,
                max_output_tokens: Some(65_536),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(1.25),
                    output_per_million: MicroCost::from_dollars(10.00),
                    cached_input_per_million: None,
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "meta-llama/llama-4-maverick".into(),
                name: "Llama 4 Maverick (via OpenRouter)".into(),
                provider_id: self.id.clone(),
                context_window: 1_000_000,
                max_output_tokens: Some(256_000),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.30),
                    output_per_million: MicroCost::from_dollars(0.50),
                    cached_input_per_million: None,
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
        ];

        *self.cached_models.lock() = Some(models.clone());
        Ok(models)
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!("OpenRouter complete: model={}", request.model);
        // Would POST to `{base_url}/chat/completions` with the X-Title header.
        Ok(LlmResponse {
            content: String::new(), model: request.model.clone(),
            input_tokens: 0, output_tokens: 0, cost: MicroCost::ZERO,
            finish_reason: Some("stop".into()),
        })
    }

    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        log::info!("OpenRouter stream: model={}", request.model);
        Ok(Box::pin(futures::stream::empty()))
    }

    async fn embed(&self, _request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        // OpenRouter supports some embedding models.
        Ok(EmbeddingResponse {
            embeddings: vec![],
            model: String::new(),
            total_tokens: 0,
            cost: MicroCost::ZERO,
        })
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        // In production we'd use the dynamic pricing from `/models`.
        // Placeholder: delegate to cached model info.
        let cache = self.cached_models.lock();
        cache.as_ref().and_then(|models| {
            models.iter().find(|m| m.id == model).and_then(|m| m.pricing.clone())
        })
    }
}
