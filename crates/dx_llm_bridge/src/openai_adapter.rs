//! OpenAI adapter — bridges to dx_core LlmProvider trait.
//!
//! Implements the full LlmProvider interface for OpenAI's API including
//! chat completions, streaming, and embeddings.

use anyhow::Result;
use dx_core::cost::{MicroCost, TokenPricing};
use dx_core::llm_provider::*;
use futures::stream::BoxStream;
use futures::StreamExt;
use http_client::HttpClient;
use serde_json::json;
use std::sync::Arc;

pub struct OpenAiAdapter {
    id: LlmProviderId,
    api_key: String,
    http_client: Arc<dyn HttpClient>,
    available: parking_lot::RwLock<bool>,
}

impl OpenAiAdapter {
    pub fn new(api_key: String, http_client: Arc<dyn HttpClient>) -> Self {
        Self {
            id: LlmProviderId::new("openai"),
            api_key,
            http_client,
            available: parking_lot::RwLock::new(true),
        }
    }

    fn base_url(&self) -> &str {
        "https://api.openai.com/v1"
    }

    fn build_messages_json(&self, messages: &[LlmMessage]) -> Vec<serde_json::Value> {
        messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    LlmRole::System => "system",
                    LlmRole::User => "user",
                    LlmRole::Assistant => "assistant",
                    LlmRole::Tool => "tool",
                };
                if m.images.is_empty() {
                    json!({ "role": role, "content": m.content })
                } else {
                    let mut content = vec![json!({"type": "text", "text": m.content})];
                    for img in &m.images {
                        content.push(json!({
                            "type": "image_url",
                            "image_url": { "url": format!("data:image/png;base64,{}", img) }
                        }));
                    }
                    json!({ "role": role, "content": content })
                }
            })
            .collect()
    }

    async fn send_request(&self, body: serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}/chat/completions", self.base_url());
        let request = http_client::Request::builder()
            .method(http_client::Method::POST)
            .uri(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .body(http_client::Body::from(serde_json::to_vec(&body)?))?;

        let mut response = self.http_client.send(request).await?;
        let body_bytes = http_client::read_body_to_string(&mut response).await?;

        if !response.status().is_success() {
            *self.available.write() = false;
            anyhow::bail!("OpenAI API error {}: {}", response.status(), body_bytes);
        }

        Ok(serde_json::from_str(&body_bytes)?)
    }
}

#[async_trait::async_trait]
impl LlmProvider for OpenAiAdapter {
    fn id(&self) -> &LlmProviderId {
        &self.id
    }

    fn name(&self) -> &str {
        "OpenAI"
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
                id: "gpt-4o".into(),
                name: "GPT-4o".into(),
                provider_id: self.id.clone(),
                context_window: 128_000,
                max_output_tokens: Some(16_384),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(2.50),
                    output_per_million: MicroCost::from_dollars(10.00),
                    cached_input_per_million: Some(MicroCost::from_dollars(1.25)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "gpt-4o-mini".into(),
                name: "GPT-4o Mini".into(),
                provider_id: self.id.clone(),
                context_window: 128_000,
                max_output_tokens: Some(16_384),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(0.15),
                    output_per_million: MicroCost::from_dollars(0.60),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.075)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "o1".into(),
                name: "o1".into(),
                provider_id: self.id.clone(),
                context_window: 200_000,
                max_output_tokens: Some(100_000),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(15.00),
                    output_per_million: MicroCost::from_dollars(60.00),
                    cached_input_per_million: Some(MicroCost::from_dollars(7.50)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "o3-mini".into(),
                name: "o3 Mini".into(),
                provider_id: self.id.clone(),
                context_window: 200_000,
                max_output_tokens: Some(100_000),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(1.10),
                    output_per_million: MicroCost::from_dollars(4.40),
                    cached_input_per_million: Some(MicroCost::from_dollars(0.55)),
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
            LlmModelInfo {
                id: "gpt-4-turbo".into(),
                name: "GPT-4 Turbo".into(),
                provider_id: self.id.clone(),
                context_window: 128_000,
                max_output_tokens: Some(4_096),
                pricing: Some(TokenPricing {
                    input_per_million: MicroCost::from_dollars(10.00),
                    output_per_million: MicroCost::from_dollars(30.00),
                    cached_input_per_million: None,
                }),
                supports_streaming: true,
                supports_tools: true,
                supports_vision: true,
            },
        ])
    }

    async fn complete(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let messages = self.build_messages_json(&request.messages);
        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "stream": false,
        });

        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        if let Some(top_p) = request.top_p {
            body["top_p"] = json!(top_p);
        }
        if !request.stop_sequences.is_empty() {
            body["stop"] = json!(request.stop_sequences);
        }

        let resp = self.send_request(body).await?;

        let content = resp["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let input_tokens = resp["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as usize;
        let output_tokens = resp["usage"]["completion_tokens"].as_u64().unwrap_or(0) as usize;
        let finish_reason = resp["choices"][0]["finish_reason"]
            .as_str()
            .map(String::from);

        let cost = self
            .pricing(&request.model)
            .map(|p| {
                let input_cost =
                    MicroCost((p.input_per_million.0 as f64 * input_tokens as f64 / 1_000_000.0) as u64);
                let output_cost =
                    MicroCost((p.output_per_million.0 as f64 * output_tokens as f64 / 1_000_000.0) as u64);
                input_cost + output_cost
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
        let messages = self.build_messages_json(&request.messages);
        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "stream": true,
        });

        if let Some(max_tokens) = request.max_tokens {
            body["max_tokens"] = json!(max_tokens);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }
        if let Some(top_p) = request.top_p {
            body["top_p"] = json!(top_p);
        }

        let url = format!("{}/chat/completions", self.base_url());
        let http_request = http_client::Request::builder()
            .method(http_client::Method::POST)
            .uri(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .body(http_client::Body::from(serde_json::to_vec(&body)?))?;

        let response = self.http_client.send(request_to_http(http_request)).await?;
        let body_stream = http_client::read_body_to_string(&mut { response }).await?;

        // Parse SSE lines from the response body
        let chunks: Vec<Result<LlmStreamChunk>> = body_stream
            .lines()
            .filter(|line| line.starts_with("data: "))
            .filter(|line| *line != "data: [DONE]")
            .filter_map(|line| {
                let json_str = &line["data: ".len()..];
                serde_json::from_str::<serde_json::Value>(json_str).ok()
            })
            .map(|v| {
                let delta = v["choices"][0]["delta"]["content"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                let finish_reason = v["choices"][0]["finish_reason"]
                    .as_str()
                    .map(String::from);
                Ok(LlmStreamChunk {
                    delta,
                    finish_reason,
                })
            })
            .collect();

        Ok(futures::stream::iter(chunks).boxed())
    }

    async fn embed(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        let body = json!({
            "model": request.model,
            "input": request.inputs,
        });

        let url = format!("{}/embeddings", self.base_url());
        let http_request = http_client::Request::builder()
            .method(http_client::Method::POST)
            .uri(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .body(http_client::Body::from(serde_json::to_vec(&body)?))?;

        let mut response = self.http_client.send(http_request).await?;
        let body_str = http_client::read_body_to_string(&mut response).await?;
        let resp: serde_json::Value = serde_json::from_str(&body_str)?;

        let embeddings: Vec<Vec<f32>> = resp["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|d| {
                d["embedding"]
                    .as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect())
            })
            .collect();

        let input_tokens = resp["usage"]["total_tokens"].as_u64().unwrap_or(0) as usize;

        Ok(EmbeddingResponse {
            embeddings,
            model: request.model.clone(),
            input_tokens,
            cost: MicroCost::ZERO,
        })
    }

    fn pricing(&self, model: &str) -> Option<TokenPricing> {
        match model {
            "gpt-4o" => Some(TokenPricing {
                input_per_million: MicroCost::from_dollars(2.50),
                output_per_million: MicroCost::from_dollars(10.00),
                cached_input_per_million: Some(MicroCost::from_dollars(1.25)),
            }),
            "gpt-4o-mini" => Some(TokenPricing {
                input_per_million: MicroCost::from_dollars(0.15),
                output_per_million: MicroCost::from_dollars(0.60),
                cached_input_per_million: Some(MicroCost::from_dollars(0.075)),
            }),
            "o1" => Some(TokenPricing {
                input_per_million: MicroCost::from_dollars(15.00),
                output_per_million: MicroCost::from_dollars(60.00),
                cached_input_per_million: Some(MicroCost::from_dollars(7.50)),
            }),
            "o3-mini" => Some(TokenPricing {
                input_per_million: MicroCost::from_dollars(1.10),
                output_per_million: MicroCost::from_dollars(4.40),
                cached_input_per_million: Some(MicroCost::from_dollars(0.55)),
            }),
            _ => None,
        }
    }
}

// Helper to avoid type conflicts
fn request_to_http(req: http_client::Request<http_client::Body>) -> http_client::Request<http_client::Body> {
    req
}
