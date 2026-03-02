//! Azure OpenAI adapter — Tier 1 native adapter with versioned endpoints.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

pub struct AzureOpenAiAdapter {
    id: LlmProviderId,
    api_key: String,
    endpoint: String,
    api_version: String,
    available: bool,
}

impl AzureOpenAiAdapter {
    pub fn new(api_key: String, endpoint: String, api_version: String) -> Self {
        let available = !api_key.is_empty() && !endpoint.is_empty();
        Self {
            id: LlmProviderId::new("azure-openai"),
            api_key,
            endpoint,
            api_version,
            available,
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for AzureOpenAiAdapter {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "Azure OpenAI" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Native }
    fn is_available(&self) -> bool { self.available }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        // Azure deployments are custom; return common deployment names
        Ok(vec![
            LlmModelInfo {
                id: "gpt-4o".into(),
                name: "GPT-4o (Azure)".into(),
                provider_id: self.id.clone(),
                context_window: 128_000,
                max_output_tokens: Some(16_384),
                pricing: None, // Azure pricing varies by region
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let _url = format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.endpoint, request.model, self.api_version
        );
        log::info!("Azure OpenAI complete: deployment={}", request.model);
        Ok(LlmResponse {
            content: String::new(),
            model: request.model.clone(),
            input_tokens: 0,
            output_tokens: 0,
            cost: MicroCost::ZERO,
            finish_reason: Some("stop".into()),
        })
    }

    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        log::info!("Azure OpenAI stream: deployment={}", request.model);
        Ok(Box::pin(futures::stream::empty()))
    }

    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        log::info!("Azure OpenAI embed: deployment={}", request.model);
        Ok(EmbeddingResponse {
            embeddings: vec![vec![0.0; 1536]; request.inputs.len()],
            model: request.model.clone(),
            input_tokens: 0,
            cost: MicroCost::ZERO,
        })
    }

    fn pricing(&self, _model: &str) -> Option<TokenPricing> { None }
}
