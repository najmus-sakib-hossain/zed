//! Anthropic adapter — bridges to dx_core LlmProvider trait.

use anyhow::Result;
use dx_core::cost::{MicroCost, TokenPricing};
use dx_core::llm_provider::*;
use futures::stream::BoxStream;
use futures::StreamExt;
use http_client::HttpClient;
use serde_json::json;
use std::sync::Arc;

pub struct AnthropicAdapter {
    id: LlmProviderId,
    api_key: String,
    http_client: Arc<dyn HttpClient>,
    available: parking_lot::RwLock<bool>,
}

impl AnthropicAdapter {
    pub fn new(api_key: String, http_client: Arc<dyn HttpClient>) -> Self {
        Self {
            id: LlmProviderId::new("anthropic"),
            api_key,
            http_client,
            available: parking_lot::RwLock::new(true),
        }
    }

    fn build_messages(&self, messages: &[LlmMessage]) -> (Option<String>, Vec<serde_json::Value>) {
        let mut system = None;
        let mut msgs = Vec::new();

        for m in messages {
            match m.role {
                LlmRole::System => {
                    system = Some(m.content.clone());
                }
                _ => {
                    let role = match m.role {
                        LlmRole::User | LlmRole::Tool => "user",
                        LlmRole::Assistant => "assistant",
                        LlmRole::System => unreachable!(),
                    };
                    if m.images.is_empty() {
                        msgs.push(json!({ "role": role, "content": m.content }));
                    } else {
                        let mut content = vec![json!({"type": "text", "text": m.content})];
                        for img in &m.images {
                            content.push(json!({
                                "type": "image",
                                "source": {
                                    "type": "base64",
                                    "media_type": "image/png",
                                    "data": img
                                }
                            }));
                        }
                        msgs.push(json!({ "role": role, "content": content }));
                    }
                }
            }
        }

        (system, msgs)
    }
}

#[async_trait::async_trait]
impl LlmProvider for AnthropicAdapter {
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
        *self.available.read()
    }

    async fn list_models(&self) -> Result<Vec<LlmModelInfo>> {
        Ok(vec![
            LlmModelInfo {
                id: "claude-sonnet-4-20250514".into(),
                name: "Claude Sonnet 4".into(),
                provider_id: self.id.clone(),
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
                id: "claude-opus-4-20250514".into(),
                name: "Claude Opus 4".into(),
                provider_id: self.id.clone(),
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
                id: "claude-3-5-haiku-20241022".into(),
                name: "Claude 3.5 Haiku".into(),
                provider_id: self.id.clone(),
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
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let (system, messages) = self.build_messages(&request.messages);
        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
        });

        if let Some(sys) = system {
            body["system"] = json!(sys);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        if let Some(top_p) = request.top_p {
            body["top_p"] = json!(top_p);
        }
        if !request.stop_sequences.is_empty() {
            body["stop_sequences"] = json!(request.stop_sequences);
        }

        let url = "https://api.anthropic.com/v1/messages";
        let http_request = http_client::Request::builder()
            .method(http_client::Method::POST)
            .uri(url)
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .body(http_client::Body::from(serde_json::to_vec(&body)?))?;

        let mut response = self.http_client.send(http_request).await?;
        let body_str = http_client::read_body_to_string(&mut response).await?;

        if !response.status().is_success() {
            *self.available.write() = false;
            anyhow::bail!("Anthropic API error {}: {}", response.status(), body_str);
        }

        let resp: serde_json::Value = serde_json::from_str(&body_str)?;

        let content = resp["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["text"].as_str())
            .unwrap_or("")
            .to_string();

        let input_tokens = resp["usage"]["input_tokens"].as_u64().unwrap_or(0) as usize;
        let output_tokens = resp["usage"]["output_tokens"].as_u64().unwrap_or(0) as usize;
        let finish_reason = resp["stop_reason"].as_str().map(String::from);

        let cost = self
            .pricing(&request.model)
            .map(|p| {
                let ic = MicroCost((p.input_per_million.0 as f64 * input_tokens as f64 / 1_000_000.0) as u64);
                let oc = MicroCost((p.output_per_million.0 as f64 * output_tokens as f64 / 1_000_000.0) as u64);
                ic + oc
            })
            .unwrap_or(MicroCost::ZERO);

        Ok(LlmResponse {
            content,
            model: request.model.clone(),
            input_tokens,
            output_tokens,
            cost,
            finish_reason,
        })
    }

    async fn stream(
        &self,
        request: &LlmRequest,
    ) -> Result<BoxStream<'static, Result<LlmStreamChunk>>> {
        let (system, messages) = self.build_messages(&request.messages);
        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "stream": true,
        });

        if let Some(sys) = system {
            body["system"] = json!(sys);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }

        let url = "https://api.anthropic.com/v1/messages";
        let http_request = http_client::Request::builder()
            .method(http_client::Method::POST)
            .uri(url)
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .body(http_client::Body::from(serde_json::to_vec(&body)?))?;

        let mut response = self.http_client.send(http_request).await?;
        let body_str = http_client::read_body_to_string(&mut response).await?;

        let chunks: Vec<Result<LlmStreamChunk>> = body_str
            .lines()
            .filter(|line| line.starts_with("data: "))
            .filter_map(|line| {
                let json_str = &line["data: ".len()..];
                serde_json::from_str::<serde_json::Value>(json_str).ok()
            })
            .filter_map(|v| {
                let event_type = v["type"].as_str().unwrap_or("");
                match event_type {
                    "content_block_delta" => {
                        let delta = v["delta"]["text"].as_str().unwrap_or("").to_string();
                        Some(Ok(LlmStreamChunk {
                            delta,
                            finish_reason: None,
                        }))
                    }
                    "message_delta" => {
                        let stop = v["delta"]["stop_reason"].as_str().map(String::from);
                        Some(Ok(LlmStreamChunk {
                            delta: String::new(),
                            finish_reason: stop,
                        }))
                    }
                    _ => None,
                }
            })
            .collect();

        Ok(futures::stream::iter(chunks).boxed())
    }

    async fn embed(&self, _request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        anyhow::bail!("Anthropic does not support embeddings directly. Use Voyage AI for Anthropic-recommended embeddings.")
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        match model {
            m if m.contains("opus-4") => Some(TokenPricing {
                input_per_million: MicroCost::from_dollars(15.00),
                output_per_million: MicroCost::from_dollars(75.00),
                cached_input_per_million: Some(MicroCost::from_dollars(1.50)),
            }),
            m if m.contains("sonnet-4") => Some(TokenPricing {
                input_per_million: MicroCost::from_dollars(3.00),
                output_per_million: MicroCost::from_dollars(15.00),
                cached_input_per_million: Some(MicroCost::from_dollars(0.30)),
            }),
            m if m.contains("haiku") => Some(TokenPricing {
                input_per_million: MicroCost::from_dollars(0.80),
                output_per_million: MicroCost::from_dollars(4.00),
                cached_input_per_million: Some(MicroCost::from_dollars(0.08)),
            }),
            _ => None,
        }
    }
}
