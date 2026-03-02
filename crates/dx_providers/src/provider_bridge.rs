//! Provider bridge utilities — shared HTTP and serialization helpers.
//!
//! These functions provide the common HTTP request/response plumbing
//! that all provider adapters use to communicate with remote APIs.

use anyhow::Result;
use dx_core::{LlmRequest, LlmResponse, LlmStreamChunk, MicroCost};
use serde::{Deserialize, Serialize};

/// Generic OpenAI-format chat completion request body.
#[derive(Debug, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub stop: Vec<String>,
    pub stream: bool,
}

/// Generic OpenAI-format chat message.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Generic OpenAI-format chat completion response.
#[derive(Debug, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub choices: Vec<ChatChoice>,
    pub usage: Option<ChatUsage>,
    pub model: String,
}

/// A choice in a completion response.
#[derive(Debug, Deserialize)]
pub struct ChatChoice {
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

/// Token usage from a completion response.
#[derive(Debug, Deserialize)]
pub struct ChatUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}

/// Provider bridge — coordinates between DX types and OpenAI-format HTTP APIs.
pub struct ProviderBridge;

impl ProviderBridge {
    /// Convert a DX `LlmRequest` into an OpenAI-format `ChatCompletionRequest`.
    pub fn to_openai_request(request: &LlmRequest) -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: request.model.clone(),
            messages: request
                .messages
                .iter()
                .map(|m| ChatMessage {
                    role: match m.role {
                        dx_core::LlmRole::System => "system".to_string(),
                        dx_core::LlmRole::User => "user".to_string(),
                        dx_core::LlmRole::Assistant => "assistant".to_string(),
                        dx_core::LlmRole::Tool => "tool".to_string(),
                    },
                    content: m.content.clone(),
                })
                .collect(),
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            top_p: request.top_p,
            stop: request.stop_sequences.clone(),
            stream: request.stream,
        }
    }

    /// Convert an OpenAI-format `ChatCompletionResponse` into a DX `LlmResponse`.
    pub fn from_openai_response(response: ChatCompletionResponse) -> Result<LlmResponse> {
        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No choices in response"))?;

        let usage = response.usage.unwrap_or(ChatUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        });

        Ok(LlmResponse {
            content: choice.message.content,
            model: response.model,
            input_tokens: usage.prompt_tokens,
            output_tokens: usage.completion_tokens,
            cost: MicroCost::ZERO, // Cost calculated by the adapter using pricing()
            finish_reason: choice.finish_reason,
        })
    }

    /// Parse a streaming SSE line into a `LlmStreamChunk`.
    pub fn parse_sse_chunk(line: &str) -> Option<LlmStreamChunk> {
        let data = line.strip_prefix("data: ")?;
        if data.trim() == "[DONE]" {
            return Some(LlmStreamChunk {
                delta: String::new(),
                finish_reason: Some("stop".to_string()),
            });
        }

        #[derive(Deserialize)]
        struct SseChunk {
            choices: Vec<SseChoice>,
        }

        #[derive(Deserialize)]
        struct SseChoice {
            delta: SseDelta,
            finish_reason: Option<String>,
        }

        #[derive(Deserialize)]
        struct SseDelta {
            content: Option<String>,
        }

        let chunk: SseChunk = serde_json::from_str(data).ok()?;
        let choice = chunk.choices.into_iter().next()?;

        Some(LlmStreamChunk {
            delta: choice.delta.content.unwrap_or_default(),
            finish_reason: choice.finish_reason,
        })
    }
}
