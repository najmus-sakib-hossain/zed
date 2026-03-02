//! AWS Bedrock adapter — bridges to dx_core LlmProvider trait.

use anyhow::Result;
use dx_core::cost::{MicroCost, TokenPricing};
use dx_core::llm_provider::*;
use futures::stream::BoxStream;
use futures::StreamExt;
use http_client::HttpClient;
use serde_json::json;
use std::sync::Arc;

pub struct BedrockAdapter {
    id: LlmProviderId,
    region: String,
    http_client: Arc<dyn HttpClient>,
    available: parking_lot::RwLock<bool>,
}

impl BedrockAdapter {
    pub fn new(http_client: Arc<dyn HttpClient>) -> Self {
        Self {
            id: LlmProviderId::new("bedrock"),
            region: std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
            http_client,
            available: parking_lot::RwLock::new(true),
        }
    }

    fn endpoint(&self, model_id: &str) -> String {
        format!(
            "https://bedrock-runtime.{}.amazonaws.com/model/{}/converse",
            self.region, model_id
        )
    }
}

#[async_trait::async_trait]
impl LlmProvider for BedrockAdapter {
    fn id(&self) -> &LlmProviderId { &self.id }
    fn name(&self) -> &str { "AWS Bedrock" }
    fn tier(&self) -> LlmProviderTier { LlmProviderTier::Native }
    fn is_available(&self) -> bool { *self.available.read() }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo {
                id: "anthropic.claude-sonnet-4-20250514-v1:0".into(),
                name: "Claude Sonnet 4 (Bedrock)".into(),
                provider_id: self.id.clone(),
                context_window: 200_000,
                max_output_tokens: Some(64_000),
                pricing: self.pricing("anthropic.claude-sonnet-4-20250514-v1:0"),
                supports_streaming: true, supports_tools: true, supports_vision: true,
            },
            LlmModelInfo {
                id: "amazon.nova-pro-v1:0".into(),
                name: "Amazon Nova Pro".into(),
                provider_id: self.id.clone(),
                context_window: 300_000,
                max_output_tokens: Some(5_000),
                pricing: self.pricing("amazon.nova-pro-v1:0"),
                supports_streaming: true, supports_tools: true, supports_vision: true,
            },
            LlmModelInfo {
                id: "meta.llama3-1-405b-instruct-v1:0".into(),
                name: "Llama 3.1 405B (Bedrock)".into(),
                provider_id: self.id.clone(),
                context_window: 128_000,
                max_output_tokens: Some(4_096),
                pricing: None,
                supports_streaming: true, supports_tools: false, supports_vision: false,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        // Bedrock uses the Converse API — requires AWS SigV4 signing which
        // needs the full AWS SDK. This implementation builds the request body
        // in Bedrock's Converse format. The actual signing should be handled
        // by the aws-sdk-bedrockruntime crate or a SigV4 middleware.
        let mut messages_json = Vec::new();
        let mut system_prompts = Vec::new();

        for m in &request.messages {
            match m.role {
                LlmRole::System => {
                    system_prompts.push(json!({"text": m.content}));
                }
                _ => {
                    let role = match m.role {
                        LlmRole::User | LlmRole::Tool => "user",
                        LlmRole::Assistant => "assistant",
                        LlmRole::System => unreachable!(),
                    };
                    messages_json.push(json!({
                        "role": role,
                        "content": [{"text": m.content}]
                    }));
                }
            }
        }

        let mut body = json!({
            "messages": messages_json,
            "inferenceConfig": {
                "maxTokens": request.max_tokens.unwrap_or(4096),
            }
        });

        if !system_prompts.is_empty() {
            body["system"] = json!(system_prompts);
        }
        if let Some(temp) = request.temperature {
            body["inferenceConfig"]["temperature"] = json!(temp);
        }
        if let Some(top_p) = request.top_p {
            body["inferenceConfig"]["topP"] = json!(top_p);
        }

        let url = self.endpoint(&request.model);
        let http_request = http_client::Request::builder()
            .method(http_client::Method::POST)
            .uri(&url)
            .header("Content-Type", "application/json")
            .body(http_client::Body::from(serde_json::to_vec(&body)?))?;

        let mut response = self.http_client.send(http_request).await?;
        let body_str = http_client::read_body_to_string(&mut response).await?;

        if !response.status().is_success() {
            *self.available.write() = false;
            anyhow::bail!("Bedrock error {}: {}", response.status(), body_str);
        }

        let resp: serde_json::Value = serde_json::from_str(&body_str)?;

        let content = resp["output"]["message"]["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let input_tokens = resp["usage"]["inputTokens"].as_u64().unwrap_or(0) as usize;
        let output_tokens = resp["usage"]["outputTokens"].as_u64().unwrap_or(0) as usize;
        let finish_reason = resp["stopReason"].as_str().map(String::from);

        Ok(LlmResponse {
            content, model: request.model.clone(), input_tokens, output_tokens,
            cost: MicroCost::ZERO, finish_reason,
        })
    }

    async fn stream(&self, request: &LlmRequest) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        // For streaming, Bedrock uses converseStream which returns event stream.
        // Simplified: fall back to complete and emit as single chunk.
        let response = self.complete(request).await?;
        Ok(futures::stream::iter(vec![Ok(LlmStreamChunk {
            delta: response.content,
            finish_reason: response.finish_reason,
        })]).boxed())
    }

    async fn embed(&self, _request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        anyhow::bail!("Bedrock embeddings use a different API (InvokeModel with Titan/Cohere). Use a dedicated embedding adapter.")
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        match model {
            m if m.contains("claude-sonnet-4") => Some(TokenPricing {
                input_per_million: MicroCost::from_dollars(3.00),
                output_per_million: MicroCost::from_dollars(15.00),
                cached_input_per_million: Some(MicroCost::from_dollars(0.30)),
            }),
            m if m.contains("nova-pro") => Some(TokenPricing {
                input_per_million: MicroCost::from_dollars(0.80),
                output_per_million: MicroCost::from_dollars(3.20),
                cached_input_per_million: None,
            }),
            _ => None,
        }
    }
}
