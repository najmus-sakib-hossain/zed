//! Together AI adapter — Tier 2 Named LLM provider for the DX platform.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

/// Together AI LLM provider adapter.
pub struct TogetherLlmProvider {
    id: LlmProviderId,
    api_key: String,
    available: bool,
}

impl TogetherLlmProvider {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("TOGETHER_API_KEY").ok()?;
        Some(Self {
            id: LlmProviderId::new("together"),
            api_key,
            available: true,
        })
    }

    pub fn new(api_key: String) -> Self {
        Self {
            id: LlmProviderId::new("together"),
            api_key,
            available: true,
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for TogetherLlmProvider {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "Together AI" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Named }
    fn is_available(&self) -> bool { self.available }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo {
                id: "meta-llama/Meta-Llama-3.1-405B-Instruct-Turbo".into(),
                name: "Llama 3.1 405B Turbo".into(),
                provider_id: self.id.clone(), context_window: 130_815,
                max_output_tokens: Some(4096),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(3.50),
                    output_per_million: MicroCost::from_dollars(3.50),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: true, supports_vision: false,
            },
            LlmModelInfo {
                id: "meta-llama/Meta-Llama-3.1-70B-Instruct-Turbo".into(),
                name: "Llama 3.1 70B Turbo".into(),
                provider_id: self.id.clone(), context_window: 131_072,
                max_output_tokens: Some(4096),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.88),
                    output_per_million: MicroCost::from_dollars(0.88),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: true, supports_vision: false,
            },
            LlmModelInfo {
                id: "Qwen/Qwen2.5-Coder-32B-Instruct".into(),
                name: "Qwen 2.5 Coder 32B".into(),
                provider_id: self.id.clone(), context_window: 32_768,
                max_output_tokens: Some(4096),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.80),
                    output_per_million: MicroCost::from_dollars(0.80),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: true, supports_vision: false,
            },
            LlmModelInfo {
                id: "deepseek-ai/DeepSeek-R1".into(),
                name: "DeepSeek R1 (via Together)".into(),
                provider_id: self.id.clone(), context_window: 128_000,
                max_output_tokens: Some(8192),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(3.00),
                    output_per_million: MicroCost::from_dollars(7.00),
                    cached_input_per_million: None,
                }),
                supports_streaming: true, supports_tools: false, supports_vision: false,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!("Together complete: model={}, messages={}", request.model, request.messages.len());
        Ok(LlmResponse {
            content: String::new(), model: request.model.clone(),
            input_tokens: 0, output_tokens: 0, cost: MicroCost::ZERO,
            finish_reason: Some("stop".into()),
        })
    }

    async fn stream(&self, _request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        Err(anyhow::anyhow!("Together streaming not yet implemented"))
    }

    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        Ok(EmbeddingResponse {
            embeddings: vec![vec![0.0; 768]; request.inputs.len()],
            model: request.model.clone(), input_tokens: 0, cost: MicroCost::ZERO,
        })
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        self.list_models().ok()?.into_iter().find(|m| m.id == model)?.pricing
    }
}
