//! Anthropic Claude provider implementation

use async_trait::async_trait;
use futures_util::StreamExt;
use futures_util::stream::BoxStream;
use reqwest::Client;
use serde::Deserialize;
use std::time::Instant;

use crate::config::ProviderConfig;
use crate::models::*;
use crate::provider::*;
use crate::streaming::StreamEvent;

const ANTHROPIC_API_BASE: &str = "https://api.anthropic.com/v1";
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Anthropic Claude provider
pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    base_url: String,
    default_model: String,
}

impl AnthropicProvider {
    pub fn new(config: &ProviderConfig) -> Result<Self, ProviderError> {
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .ok_or_else(|| ProviderError::AuthError("ANTHROPIC_API_KEY not set".into()))?;

        Ok(Self {
            client: Client::new(),
            api_key,
            base_url: config.base_url.clone().unwrap_or_else(|| ANTHROPIC_API_BASE.into()),
            default_model: config
                .default_model
                .clone()
                .unwrap_or_else(|| "claude-sonnet-4-20250514".into()),
        })
    }

    fn build_headers(&self) -> reqwest::header::HeaderMap {
        use reqwest::header::HeaderValue;
        let mut headers = reqwest::header::HeaderMap::new();
        if let Ok(val) = HeaderValue::from_str(&self.api_key) {
            headers.insert("x-api-key", val);
        }
        headers.insert("anthropic-version", HeaderValue::from_static(ANTHROPIC_VERSION));
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers
    }
}

/// Anthropic Messages API response
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    model: String,
    content: Vec<AnthropicContent>,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    input: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    delta: Option<AnthropicDelta>,
    #[serde(default)]
    message: Option<AnthropicStreamMessage>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
    #[serde(default)]
    index: Option<u32>,
    #[serde(default)]
    content_block: Option<AnthropicContent>,
}

#[derive(Debug, Deserialize)]
struct AnthropicDelta {
    #[serde(rename = "type")]
    #[serde(default)]
    delta_type: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicStreamMessage {
    id: String,
    model: String,
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(crate::models::known_models()
            .into_iter()
            .filter(|m| m.provider == "anthropic")
            .collect())
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let start = Instant::now();
        let model = if request.model.is_empty() {
            &self.default_model
        } else {
            &request.model
        };

        // Extract system message (Anthropic handles it separately)
        let system = request
            .messages
            .iter()
            .find(|m| m.role == Role::System)
            .map(|m| m.content.clone());

        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        Role::User => "user",
                        Role::Assistant => "assistant",
                        Role::Tool => "user",
                        _ => "user",
                    },
                    "content": m.content,
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
        });

        if let Some(sys) = system {
            body["system"] = serde_json::Value::String(sys);
        }
        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }
        if let Some(top_p) = request.top_p {
            body["top_p"] = serde_json::json!(top_p);
        }

        let resp = self
            .client
            .post(format!("{}/messages", self.base_url))
            .headers(self.build_headers())
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            if status.as_u16() == 429 {
                return Err(ProviderError::RateLimited {
                    retry_after_ms: 60_000,
                });
            }
            return Err(ProviderError::ApiError {
                status: status.as_u16(),
                message: text,
            });
        }

        let api_resp: AnthropicResponse = resp.json().await?;
        let latency_ms = start.elapsed().as_millis() as u64;

        // Collect text content and tool calls
        let mut text_content = String::new();
        let mut tool_calls = Vec::new();

        for block in &api_resp.content {
            match block.content_type.as_str() {
                "text" => {
                    if let Some(text) = &block.text {
                        text_content.push_str(text);
                    }
                }
                "tool_use" => {
                    if let (Some(id), Some(name), Some(input)) =
                        (&block.id, &block.name, &block.input)
                    {
                        tool_calls.push(ToolCall {
                            id: id.clone(),
                            call_type: "function".into(),
                            function: FunctionCall {
                                name: name.clone(),
                                arguments: serde_json::to_string(input).unwrap_or_default(),
                            },
                        });
                    }
                }
                _ => {}
            }
        }

        Ok(ChatResponse {
            id: api_resp.id,
            model: api_resp.model,
            choices: vec![Choice {
                index: 0,
                message: ChatMessage {
                    role: Role::Assistant,
                    content: text_content,
                    name: None,
                    tool_call_id: None,
                    tool_calls: if tool_calls.is_empty() {
                        None
                    } else {
                        Some(tool_calls)
                    },
                },
                finish_reason: api_resp.stop_reason,
            }],
            usage: Usage {
                prompt_tokens: api_resp.usage.input_tokens,
                completion_tokens: api_resp.usage.output_tokens,
                total_tokens: api_resp.usage.input_tokens + api_resp.usage.output_tokens,
            },
            created: chrono::Utc::now(),
            provider: "anthropic".into(),
            latency_ms,
        })
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<BoxStream<'static, Result<StreamEvent, ProviderError>>, ProviderError> {
        let model = if request.model.is_empty() {
            self.default_model.clone()
        } else {
            request.model.clone()
        };

        let system = request
            .messages
            .iter()
            .find(|m| m.role == Role::System)
            .map(|m| m.content.clone());

        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        Role::User => "user",
                        Role::Assistant => "assistant",
                        _ => "user",
                    },
                    "content": m.content,
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "stream": true,
        });

        if let Some(sys) = system {
            body["system"] = serde_json::Value::String(sys);
        }

        let resp = self
            .client
            .post(format!("{}/messages", self.base_url))
            .headers(self.build_headers())
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError {
                status: 0,
                message: text,
            });
        }

        let stream = resp.bytes_stream().map(move |chunk| {
            let chunk = chunk.map_err(ProviderError::Network)?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                let line = line.trim();
                if line.starts_with("data: ") {
                    let data = &line[6..];
                    if let Ok(event) = serde_json::from_str::<AnthropicStreamEvent>(data) {
                        match event.event_type.as_str() {
                            "content_block_delta" => {
                                if let Some(delta) = &event.delta {
                                    if let Some(text) = &delta.text {
                                        return Ok(StreamEvent::Delta {
                                            content: text.clone(),
                                        });
                                    }
                                }
                            }
                            "message_start" => {
                                if let Some(msg) = &event.message {
                                    return Ok(StreamEvent::Start {
                                        id: msg.id.clone(),
                                        model: msg.model.clone(),
                                    });
                                }
                            }
                            "message_delta" => {
                                if let Some(delta) = &event.delta {
                                    if let Some(reason) = &delta.stop_reason {
                                        return Ok(StreamEvent::Done {
                                            finish_reason: reason.clone(),
                                        });
                                    }
                                }
                            }
                            "message_stop" => {
                                return Ok(StreamEvent::Done {
                                    finish_reason: "end_turn".into(),
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }

            Ok(StreamEvent::Delta {
                content: String::new(),
            })
        });

        Ok(Box::pin(stream))
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            streaming: true,
            function_calling: true,
            vision: true,
            embeddings: false,
            multi_modal: true,
            json_mode: false,
            batch_api: true,
        }
    }

    async fn health_check(&self) -> Result<bool, ProviderError> {
        // Anthropic doesn't have a health endpoint, try a minimal request
        // Just check if we can reach the API
        let resp = self
            .client
            .post(format!("{}/messages", self.base_url))
            .headers(self.build_headers())
            .json(&serde_json::json!({
                "model": self.default_model,
                "max_tokens": 1,
                "messages": [{"role": "user", "content": "hi"}]
            }))
            .send()
            .await?;
        // 200 or 429 means the API is reachable
        Ok(resp.status().is_success() || resp.status().as_u16() == 429)
    }

    fn count_tokens(&self, text: &str, _model: &str) -> Result<u32, ProviderError> {
        // Anthropic uses similar tokenization to GPT. ~4 chars per token avg
        Ok((text.len() as f64 / 4.0).ceil() as u32)
    }
}
