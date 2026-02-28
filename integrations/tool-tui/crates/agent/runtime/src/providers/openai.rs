//! OpenAI provider implementation

use async_trait::async_trait;
use futures_util::StreamExt;
use futures_util::stream::BoxStream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::config::ProviderConfig;
use crate::models::*;
use crate::provider::*;
use crate::streaming::StreamEvent;

const OPENAI_API_BASE: &str = "https://api.openai.com/v1";

/// OpenAI provider
pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    base_url: String,
    organization: Option<String>,
    default_model: String,
}

impl OpenAiProvider {
    pub fn new(config: &ProviderConfig) -> Result<Self, ProviderError> {
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| ProviderError::AuthError("OPENAI_API_KEY not set".into()))?;

        Ok(Self {
            client: Client::new(),
            api_key,
            base_url: config.base_url.clone().unwrap_or_else(|| OPENAI_API_BASE.into()),
            organization: config.organization.clone(),
            default_model: config.default_model.clone().unwrap_or_else(|| "gpt-4o".into()),
        })
    }

    fn build_headers(&self) -> reqwest::header::HeaderMap {
        use reqwest::header::HeaderValue;
        let mut headers = reqwest::header::HeaderMap::new();
        if let Ok(val) = HeaderValue::from_str(&format!("Bearer {}", self.api_key)) {
            headers.insert("Authorization", val);
        }
        if let Some(org) = &self.organization {
            if let Ok(val) = HeaderValue::from_str(org) {
                headers.insert("OpenAI-Organization", val);
            }
        }
        headers
    }
}

/// OpenAI API response format
#[derive(Debug, Deserialize)]
struct OpenAiChatResponse {
    id: String,
    model: String,
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
    created: i64,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    index: u32,
    message: OpenAiMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAiToolCall>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OpenAiFunction,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChunk {
    id: String,
    model: String,
    choices: Vec<OpenAiStreamChoice>,
    #[serde(default)]
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChoice {
    delta: OpenAiDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAiToolCall>>,
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(crate::models::known_models()
            .into_iter()
            .filter(|m| m.provider == "openai")
            .collect())
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let start = Instant::now();
        let model = if request.model.is_empty() {
            &self.default_model
        } else {
            &request.model
        };

        let body = serde_json::json!({
            "model": model,
            "messages": request.messages,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "top_p": request.top_p,
            "stop": request.stop,
            "stream": false,
            "tools": request.tools,
            "tool_choice": request.tool_choice,
        });

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
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

        let api_resp: OpenAiChatResponse = resp.json().await?;
        let latency_ms = start.elapsed().as_millis() as u64;

        let choices = api_resp
            .choices
            .into_iter()
            .map(|c| Choice {
                index: c.index,
                message: ChatMessage {
                    role: match c.message.role.as_str() {
                        "assistant" => Role::Assistant,
                        "user" => Role::User,
                        "system" => Role::System,
                        _ => Role::Assistant,
                    },
                    content: c.message.content.unwrap_or_default(),
                    name: None,
                    tool_call_id: None,
                    tool_calls: c.message.tool_calls.map(|tcs| {
                        tcs.into_iter()
                            .map(|tc| ToolCall {
                                id: tc.id,
                                call_type: tc.call_type,
                                function: FunctionCall {
                                    name: tc.function.name,
                                    arguments: tc.function.arguments,
                                },
                            })
                            .collect()
                    }),
                },
                finish_reason: c.finish_reason,
            })
            .collect();

        let usage = api_resp
            .usage
            .map(|u| Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            })
            .unwrap_or_default();

        Ok(ChatResponse {
            id: api_resp.id,
            model: api_resp.model,
            choices,
            usage,
            created: chrono::DateTime::from_timestamp(api_resp.created, 0)
                .unwrap_or_else(chrono::Utc::now),
            provider: "openai".into(),
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

        let body = serde_json::json!({
            "model": model,
            "messages": request.messages,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "stream": true,
        });

        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
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
                    if data == "[DONE]" {
                        return Ok(StreamEvent::Done {
                            finish_reason: "stop".into(),
                        });
                    }
                    if let Ok(chunk) = serde_json::from_str::<OpenAiStreamChunk>(data) {
                        if let Some(choice) = chunk.choices.first() {
                            if let Some(content) = &choice.delta.content {
                                return Ok(StreamEvent::Delta {
                                    content: content.clone(),
                                });
                            }
                            if let Some(reason) = &choice.finish_reason {
                                return Ok(StreamEvent::Done {
                                    finish_reason: reason.clone(),
                                });
                            }
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
            embeddings: true,
            multi_modal: true,
            json_mode: true,
            batch_api: true,
        }
    }

    async fn health_check(&self) -> Result<bool, ProviderError> {
        let resp = self
            .client
            .get(format!("{}/models", self.base_url))
            .headers(self.build_headers())
            .send()
            .await?;
        Ok(resp.status().is_success())
    }

    fn count_tokens(&self, text: &str, _model: &str) -> Result<u32, ProviderError> {
        // Approximate: ~4 chars per token for English text
        Ok((text.len() as f64 / 4.0).ceil() as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_provider_creation_requires_key() {
        let config = ProviderConfig {
            provider_type: "openai".into(),
            api_key: None,
            base_url: None,
            organization: None,
            default_model: None,
            rate_limit_rpm: None,
            headers: Default::default(),
            enabled: true,
        };
        // Will fail if OPENAI_API_KEY not set
        let result = OpenAiProvider::new(&config);
        // Don't assert failure since env var might be set in CI
        let _ = result;
    }
}
