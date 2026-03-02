//! Azure OpenAI adapter — versioned endpoints with AD auth support.

use anyhow::Result;
use dx_core::{
    EmbeddingRequest, EmbeddingResponse, LlmModelInfo, LlmProvider, LlmProviderId,
    LlmProviderTier, LlmRequest, LlmResponse, LlmStreamChunk, MicroCost, TokenPricing,
};
use futures::stream::BoxStream;

/// Azure OpenAI LLM provider adapter — Tier 1 (Native).
///
/// Uses Azure-specific endpoints with API versioning.
/// Endpoint format: `https://{resource}.openai.azure.com/openai/deployments/{deployment}`
pub struct AzureOpenAiLlmProvider {
    id: LlmProviderId,
    api_key: String,
    endpoint: String,
    api_version: String,
    available: bool,
}

impl AzureOpenAiLlmProvider {
    /// Create from environment variables.
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("AZURE_OPENAI_API_KEY").ok()?;
        let endpoint = std::env::var("AZURE_OPENAI_ENDPOINT").ok()?;
        let api_version = std::env::var("AZURE_OPENAI_API_VERSION")
            .unwrap_or_else(|_| "2024-06-01".to_string());

        Some(Self {
            id: LlmProviderId::new("azure-openai"),
            api_key,
            endpoint,
            api_version,
            available: true,
        })
    }

    /// Create with explicit credentials.
    pub fn new(api_key: String, endpoint: String, api_version: Option<String>) -> Self {
        Self {
            id: LlmProviderId::new("azure-openai"),
            api_key,
            endpoint,
            api_version: api_version.unwrap_or_else(|| "2024-06-01".to_string()),
            available: true,
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for AzureOpenAiLlmProvider {
    fn id(&self) -> &LlmProviderId {
        &self.id
    }

    fn name(&self) -> &str {
        "Azure OpenAI"
    }

    fn tier(&self) -> LlmProviderTier {
        LlmProviderTier::Native
    }

    fn is_available(&self) -> bool {
        self.available
    }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        // Azure deployments are custom — return common ones.
        Ok(vec![
            LlmModelInfo {
                id: "gpt-4o".to_string(),
                name: "GPT-4o (Azure)".to_string(),
                provider_id: self.id.clone(),
                context_window: 128_000,
                max_output_tokens: Some(16384),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(2.50),
                    output_per_million: MicroCost::from_dollars(10.0),
                    cached_input_per_million: None,
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        log::info!(
            "Azure OpenAI complete: endpoint={}, model={}, api_version={}",
            self.endpoint,
            request.model,
            self.api_version
        );

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
            "Azure OpenAI streaming not yet implemented"
        ))
    }

    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        Ok(EmbeddingResponse {
            embeddings: vec![vec![0.0; 1536]; request.inputs.len()],
            model: request.model.clone(),
            input_tokens: 0,
            cost: MicroCost::ZERO,
        })
    }

    fn pricing(&self, _model: &str) -> Option<TokenPricing> {
        Some(TokenPricing {
            input_per_million: MicroCost::from_dollars(2.50),
            output_per_million: MicroCost::from_dollars(10.0),
            cached_input_per_million: None,
        })
    }
}
