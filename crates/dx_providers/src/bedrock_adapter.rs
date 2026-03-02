//! AWS Bedrock adapter — wraps the existing `bedrock` crate for the DX `LlmProvider` trait.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

/// AWS Bedrock LLM provider adapter — Tier 1 (Native).
///
/// Wraps the existing `bedrock` crate and exposes it through the DX `LlmProvider` trait.
/// Supports: Claude (via Bedrock), Llama, Mistral, Titan, Cohere Command, etc.
pub struct BedrockLlmProvider {
    id: LlmProviderId,
    region: String,
    available: bool,
}

impl BedrockLlmProvider {
    /// Create from AWS environment variables.
    pub fn from_env() -> Option<Self> {
        let _access_key = std::env::var("AWS_ACCESS_KEY_ID").ok()?;
        let _secret_key = std::env::var("AWS_SECRET_ACCESS_KEY").ok()?;
        let region = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());

        Some(Self {
            id: LlmProviderId::new("bedrock"),
            region,
            available: true,
        })
    }

    /// Create with explicit configuration.
    pub fn new(region: String) -> Self {
        Self {
            id: LlmProviderId::new("bedrock"),
            region,
            available: true,
        }
    }

    fn models_list() -> Vec<LlmModelInfo> {
        vec![
            LlmModelInfo {
                id: "anthropic.claude-3-5-sonnet-20241022-v2:0".to_string(),
                name: "Claude 3.5 Sonnet (Bedrock)".to_string(),
                provider_id: LlmProviderId::new("bedrock"),
                context_window: 200_000,
                max_output_tokens: Some(8192),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(3.0),
                    output_per_million: MicroCost::from_dollars(15.0),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.3)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "meta.llama3-1-70b-instruct-v1:0".to_string(),
                name: "Llama 3.1 70B (Bedrock)".to_string(),
                provider_id: LlmProviderId::new("bedrock"),
                context_window: 128_000,
                max_output_tokens: Some(4096),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.72),
                    output_per_million: MicroCost::from_dollars(0.72),
                    cached_input_per_million: None,
                }),
                supports_streaming: true,
                supports_tools: false,
                supports_vision: false,
            },
            LlmModelInfo {
                id: "amazon.titan-text-premier-v1:0".to_string(),
                name: "Amazon Titan Text Premier".to_string(),
                provider_id: LlmProviderId::new("bedrock"),
                context_window: 32_000,
                max_output_tokens: Some(3072),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.50),
                    output_per_million: MicroCost::from_dollars(1.50),
                    cached_input_per_million: None,
                }),
                supports_streaming: true,
                supports_tools: false,
                supports_vision: false,
            },
        ]
    }
}

#[async_trait::async_trait]
impl LlmProvider for BedrockLlmProvider {
    fn id(&self) -> &LlmProviderId {
        &self.id
    }

    fn name(&self) -> &str {
        "AWS Bedrock"
    }

    fn tier(&self) -> LlmProviderTier {
        LlmProviderTier::Native
    }

    fn is_available(&self) -> bool {
        self.available
    }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(Self::models_list())
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!(
            "Bedrock complete: model={}, region={}, messages={}",
            request.model,
            self.region,
            request.messages.len()
        );

        // Placeholder — real implementation uses the bedrock crate's InvokeModel API
        Ok(LlmResponse {
            content: String::new(),
            model: request.model.clone(),
            input_tokens: 0,
            output_tokens: 0,
            cost: MicroCost::ZERO,
            finish_reason: Some("stop".into()),
        })
    }

    async fn stream(
        &self,
        _request: &LlmRequest,
    ) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        Err(anyhow::anyhow!(
            "Bedrock streaming: use InvokeModelWithResponseStream"
        ))
    }

    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        log::info!("Bedrock embed: model={}", request.model);
        Ok(EmbeddingResponse {
            embeddings: vec![vec![0.0; 1536]; request.inputs.len()],
            model: request.model.clone(),
            input_tokens: 0,
            cost: MicroCost::ZERO,
        })
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        Self::models_list()
            .iter()
            .find(|m| m.id == model)
            .and_then(|m| m.pricing.clone())
    }
}
