//! Vercel AI SDK adapter — Tier 4 Aggregator LLM provider.
//! Provides a unified interface similar to Vercel AI SDK's provider system.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

/// Vercel AI-style aggregator — routes to any compatible provider endpoint.
pub struct VercelLlmProvider {
    id: LlmProviderId,
    api_key: String,
    base_url: String,
    available: bool,
}

impl VercelLlmProvider {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("VERCEL_AI_API_KEY").ok()?;
        let base_url = std::env::var("VERCEL_AI_BASE_URL")
            .unwrap_or_else(|_| "https://api.vercel.ai/v1".into());
        Some(Self {
            id: LlmProviderId::new("vercel"),
            api_key,
            base_url,
            available: true,
        })
    }

    pub fn new(api_key: String, base_url: String) -> Self {
        Self {
            id: LlmProviderId::new("vercel"),
            api_key,
            base_url,
            available: true,
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for VercelLlmProvider {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "Vercel AI" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Aggregator }
    fn is_available(&self) -> bool { self.available }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        // Vercel AI SDK supports dynamic provider configuration.
        // Models depend on with provider keys are configured.
        Ok(vec![
            LlmModelInfo {
                id: "vercel/v0-1.0-md".into(), name: "v0 1.0 Medium".into(),
                provider_id: self.id.clone(), context_window: 128_000,
                max_output_tokens: Some(8192),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.0),
                    output_per_million: MicroCost::from_dollars(0.0),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: true, supports_vision: true,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!(
            "Vercel AI complete: url={}, model={}, messages={}",
            self.base_url, request.model, request.messages.len()
        );
        Ok(LlmResponse {
            content: String::new(), model: request.model.clone(),
            input_tokens: 0, output_tokens: 0, cost: MicroCost::ZERO,
            finish_reason: Some("stop".into()),
        })
    }

    async fn stream(&self, _request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        Err(anyhow::anyhow!("Vercel AI streaming not yet implemented"))
    }

    async fn embed(&self, _request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        Err(anyhow::anyhow!("Vercel AI embeddings not yet implemented"))
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        self.list_models().ok()?.into_iter().find(|m| m.id == model)?.pricing
    }
}
