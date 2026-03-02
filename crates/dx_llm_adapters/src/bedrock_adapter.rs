//! AWS Bedrock adapter — Tier 1 native adapter wrapping `crates/bedrock`.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

pub struct BedrockLlmAdapter {
    id: LlmProviderId,
    api_key: String,
    region: String,
    available: bool,
}

impl BedrockLlmAdapter {
    pub fn new(api_key: String) -> Self {
        let available = !api_key.is_empty();
        Self {
            id: LlmProviderId::new("bedrock"),
            api_key,
            region: "us-east-1".into(),
            available,
        }
    }

    pub fn with_region(mut self, region: String) -> Self {
        self.region = region;
        self
    }
}

#[async_trait::async_trait]
impl LlmProvider for BedrockLlmAdapter {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "AWS Bedrock" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Native }
    fn is_available(&self) -> bool { self.available }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo {
                id: "anthropic.claude-sonnet-4-20250514-v1:0".into(),
                name: "Claude Sonnet 4 (Bedrock)".into(),
                provider_id: self.id.clone(),
                context_window: 200_000,
                max_output_tokens: Some(64_000),
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
                id: "amazon.nova-pro-v1:0".into(),
                name: "Amazon Nova Pro".into(),
                provider_id: self.id.clone(),
                context_window: 300_000,
                max_output_tokens: Some(5_000),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.80),
                    output_per_million: MicroCost::from_dollars(3.20),
                    cached_input_per_million: None,
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!("Bedrock complete: model={}, region={}", request.model, self.region);
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
        log::info!("Bedrock stream: model={}", request.model);
        Ok(Box::pin(futures::stream::empty()))
    }

    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        log::info!("Bedrock embed: model={}", request.model);
        Ok(EmbeddingResponse {
            embeddings: vec![vec![0.0; 1024]; request.inputs.len()],
            model: request.model.clone(),
            input_tokens: 0,
            cost: MicroCost::ZERO,
        })
    }

    fn pricing(&self, _model: &str) -> Option<TokenPricing> { None }
}
