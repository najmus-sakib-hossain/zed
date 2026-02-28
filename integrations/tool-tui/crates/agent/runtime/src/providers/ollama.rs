//! Ollama local provider implementation

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

const OLLAMA_DEFAULT_URL: &str = "http://localhost:11434";

/// Ollama local provider
pub struct OllamaProvider {
    client: Client,
    base_url: String,
    default_model: String,
}

impl OllamaProvider {
    pub fn new(config: &ProviderConfig) -> Result<Self, ProviderError> {
        Ok(Self {
            client: Client::new(),
            base_url: config.base_url.clone().unwrap_or_else(|| OLLAMA_DEFAULT_URL.into()),
            default_model: config.default_model.clone().unwrap_or_else(|| "llama3.2".into()),
        })
    }
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    model: String,
    message: OllamaMessage,
    done: bool,
    #[serde(default)]
    total_duration: Option<u64>,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
    #[serde(default)]
    eval_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaModel {
    name: String,
    #[serde(default)]
    size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelList {
    models: Vec<OllamaModel>,
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        let resp = self.client.get(format!("{}/api/tags", self.base_url)).send().await?;

        if !resp.status().is_success() {
            return Err(ProviderError::Unavailable("Ollama not running".into()));
        }

        let list: OllamaModelList = resp.json().await?;
        Ok(list
            .models
            .into_iter()
            .map(|m| ModelInfo {
                id: m.name.clone(),
                name: m.name,
                provider: "ollama".into(),
                context_window: 128_000,
                max_output_tokens: 4096,
                input_cost_per_1k: 0.0, // Local = free
                output_cost_per_1k: 0.0,
                supports_streaming: true,
                supports_tools: true,
                supports_vision: false,
            })
            .collect())
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let start = Instant::now();
        let model = if request.model.is_empty() {
            &self.default_model
        } else {
            &request.model
        };

        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        Role::System => "system",
                        Role::User => "user",
                        Role::Assistant => "assistant",
                        Role::Tool => "tool",
                    },
                    "content": m.content,
                })
            })
            .collect();

        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": false,
            "options": {
                "temperature": request.temperature.unwrap_or(0.7),
                "num_predict": request.max_tokens.unwrap_or(4096),
            }
        });

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Ollama not reachable: {}", e)))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError {
                status: status.as_u16(),
                message: text,
            });
        }

        let api_resp: OllamaChatResponse = resp.json().await?;
        let latency_ms = start.elapsed().as_millis() as u64;

        let prompt_tokens = api_resp.prompt_eval_count.unwrap_or(0);
        let completion_tokens = api_resp.eval_count.unwrap_or(0);

        Ok(ChatResponse {
            id: uuid::Uuid::new_v4().to_string(),
            model: api_resp.model,
            choices: vec![Choice {
                index: 0,
                message: ChatMessage {
                    role: Role::Assistant,
                    content: api_resp.message.content,
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                },
                finish_reason: Some("stop".into()),
            }],
            usage: Usage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            },
            created: chrono::Utc::now(),
            provider: "ollama".into(),
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

        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        Role::System => "system",
                        Role::User => "user",
                        Role::Assistant => "assistant",
                        Role::Tool => "tool",
                    },
                    "content": m.content,
                })
            })
            .collect();

        let body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": true,
        });

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Unavailable(format!("Ollama not reachable: {}", e)))?;

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

            // Ollama streams JSON objects, one per line
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if let Ok(resp) = serde_json::from_str::<OllamaChatResponse>(line) {
                    if resp.done {
                        return Ok(StreamEvent::Done {
                            finish_reason: "stop".into(),
                        });
                    }
                    return Ok(StreamEvent::Delta {
                        content: resp.message.content,
                    });
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
            vision: false,
            embeddings: true,
            multi_modal: false,
            json_mode: true,
            batch_api: false,
        }
    }

    async fn health_check(&self) -> Result<bool, ProviderError> {
        let resp = self.client.get(format!("{}/api/tags", self.base_url)).send().await;
        Ok(resp.is_ok() && resp.unwrap().status().is_success())
    }

    fn count_tokens(&self, text: &str, _model: &str) -> Result<u32, ProviderError> {
        // Approximate for llama-family models
        Ok((text.len() as f64 / 4.0).ceil() as u32)
    }
}
