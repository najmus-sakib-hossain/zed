//! Google Gemini provider implementation

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

const GOOGLE_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

/// Google Gemini provider
pub struct GoogleProvider {
    client: Client,
    api_key: String,
    base_url: String,
    default_model: String,
}

impl GoogleProvider {
    pub fn new(config: &ProviderConfig) -> Result<Self, ProviderError> {
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("GOOGLE_API_KEY").ok())
            .or_else(|| std::env::var("GEMINI_API_KEY").ok())
            .ok_or_else(|| ProviderError::AuthError("GOOGLE_API_KEY not set".into()))?;

        Ok(Self {
            client: Client::new(),
            api_key,
            base_url: config.base_url.clone().unwrap_or_else(|| GOOGLE_API_BASE.into()),
            default_model: config
                .default_model
                .clone()
                .unwrap_or_else(|| "gemini-2.0-flash".into()),
        })
    }
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
    #[serde(rename = "usageMetadata")]
    usage_metadata: Option<GeminiUsage>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
    #[serde(rename = "finishReason")]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Deserialize)]
struct GeminiPart {
    #[serde(default)]
    text: Option<String>,
    #[serde(rename = "functionCall")]
    #[serde(default)]
    function_call: Option<GeminiFunctionCall>,
}

#[derive(Debug, Deserialize)]
struct GeminiFunctionCall {
    name: String,
    args: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct GeminiUsage {
    #[serde(rename = "promptTokenCount")]
    prompt_token_count: Option<u32>,
    #[serde(rename = "candidatesTokenCount")]
    candidates_token_count: Option<u32>,
    #[serde(rename = "totalTokenCount")]
    total_token_count: Option<u32>,
}

#[async_trait]
impl LlmProvider for GoogleProvider {
    fn name(&self) -> &str {
        "google"
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(crate::models::known_models()
            .into_iter()
            .filter(|m| m.provider == "google")
            .collect())
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let start = Instant::now();
        let model = if request.model.is_empty() {
            &self.default_model
        } else {
            &request.model
        };

        // Convert to Gemini format
        let system_instruction =
            request.messages.iter().find(|m| m.role == Role::System).map(|m| {
                serde_json::json!({
                    "parts": [{"text": m.content}]
                })
            });

        let contents: Vec<serde_json::Value> = request
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        Role::User | Role::Tool => "user",
                        Role::Assistant => "model",
                        _ => "user",
                    },
                    "parts": [{"text": m.content}]
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "contents": contents,
            "generationConfig": {
                "maxOutputTokens": request.max_tokens.unwrap_or(4096),
                "temperature": request.temperature.unwrap_or(1.0),
            }
        });

        if let Some(sys) = system_instruction {
            body["systemInstruction"] = sys;
        }

        let url =
            format!("{}/models/{}:generateContent?key={}", self.base_url, model, self.api_key);

        let resp = self.client.post(&url).json(&body).send().await?;

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

        let api_resp: GeminiResponse = resp.json().await?;
        let latency_ms = start.elapsed().as_millis() as u64;

        let mut text_content = String::new();
        let mut tool_calls = Vec::new();
        let mut finish_reason = None;

        if let Some(candidates) = &api_resp.candidates {
            if let Some(candidate) = candidates.first() {
                finish_reason = candidate.finish_reason.clone();
                for part in &candidate.content.parts {
                    if let Some(text) = &part.text {
                        text_content.push_str(text);
                    }
                    if let Some(fc) = &part.function_call {
                        tool_calls.push(ToolCall {
                            id: uuid::Uuid::new_v4().to_string(),
                            call_type: "function".into(),
                            function: FunctionCall {
                                name: fc.name.clone(),
                                arguments: serde_json::to_string(&fc.args).unwrap_or_default(),
                            },
                        });
                    }
                }
            }
        }

        let usage = api_resp
            .usage_metadata
            .map(|u| Usage {
                prompt_tokens: u.prompt_token_count.unwrap_or(0),
                completion_tokens: u.candidates_token_count.unwrap_or(0),
                total_tokens: u.total_token_count.unwrap_or(0),
            })
            .unwrap_or_default();

        Ok(ChatResponse {
            id: uuid::Uuid::new_v4().to_string(),
            model: model.to_string(),
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
                finish_reason,
            }],
            usage,
            created: chrono::Utc::now(),
            provider: "google".into(),
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

        let contents: Vec<serde_json::Value> = request
            .messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        Role::User | Role::Tool => "user",
                        Role::Assistant => "model",
                        _ => "user",
                    },
                    "parts": [{"text": m.content}]
                })
            })
            .collect();

        let body = serde_json::json!({
            "contents": contents,
            "generationConfig": {
                "maxOutputTokens": request.max_tokens.unwrap_or(4096),
            }
        });

        let url = format!(
            "{}/models/{}:streamGenerateContent?key={}&alt=sse",
            self.base_url, model, self.api_key
        );

        let resp = self.client.post(&url).json(&body).send().await?;

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
                    if let Ok(resp) = serde_json::from_str::<GeminiResponse>(data) {
                        if let Some(candidates) = &resp.candidates {
                            if let Some(candidate) = candidates.first() {
                                for part in &candidate.content.parts {
                                    if let Some(text) = &part.text {
                                        return Ok(StreamEvent::Delta {
                                            content: text.clone(),
                                        });
                                    }
                                }
                                if candidate.finish_reason.is_some() {
                                    return Ok(StreamEvent::Done {
                                        finish_reason: candidate
                                            .finish_reason
                                            .clone()
                                            .unwrap_or_else(|| "STOP".into()),
                                    });
                                }
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
            batch_api: false,
        }
    }

    async fn health_check(&self) -> Result<bool, ProviderError> {
        let url = format!("{}/models?key={}", self.base_url, self.api_key);
        let resp = self.client.get(&url).send().await?;
        Ok(resp.status().is_success())
    }

    fn count_tokens(&self, text: &str, _model: &str) -> Result<u32, ProviderError> {
        Ok((text.len() as f64 / 4.0).ceil() as u32)
    }
}
