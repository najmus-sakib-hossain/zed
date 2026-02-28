Now it's time to start working on our code editor. In our code editor AI model by default if no models are configured, we show no message but instead of that please show these 4 models by default. 

This is a demo but create a new crate in crates folder called providers and please put these 4 models by default instead of showing no models in our code editor!!!

```rust
// OpenCode provider for free LLM access
// Powered by OpenCode Zen: https://opencode.ai
//
// OpenCode offers free promotional models during feedback phases.
// This is a legitimate integration of their open-source platform.

use crate::llm::error::ProviderError;
use crate::llm::provider::{LlmProvider, ProviderStream};
use crate::llm::types::{
    AuthRequirement, ChatChunk, ChatMessage, ChatRequest, ChatResponse, MessageContent, ModelInfo,
    ProviderCapabilities, ProviderMetadata, UsageInfo,
};
use async_trait::async_trait;
use futures_util::stream::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

const OPENCODE_API: &str = "https://opencode.ai/zen/v1";
const MODELS_DEV_API: &str = "https://models.dev/api.json";

/// Free models recommended for DX users (no API key required).
///
/// These are the models that:
/// - appear under the `opencode` provider in `models.dev`,
/// - have `cost.input == 0` and `cost.output == 0`,
/// - and work with OpenCode Zen's OpenAI-compatible `chat/completions` endpoint.
pub const FREE_MODELS: [&str; 4] = [
    "glm-5-free",
    "minimax-m2.5-free",
    "big-pickle",
    "trinity-large-preview-free",
];

#[derive(Debug, Serialize)]
struct OpenCodeChatRequest {
    model: String,
    messages: Vec<OpenCodeMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenCodeMessage {
    role: String,
    content: String,
    #[serde(default)]
    reasoning: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenCodeChatResponse {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    choices: Vec<OpenCodeChoice>,
    #[serde(default)]
    usage: Option<OpenCodeUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenCodeChoice {
    message: OpenCodeMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenCodeUsage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    total_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OpenCodeStreamChunk {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    choices: Vec<OpenCodeStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenCodeStreamChoice {
    delta: OpenCodeDelta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenCodeDelta {
    #[serde(default)]
    content: Option<String>,
}

pub struct OpenCodeProvider {
    client: Client,
    base_url: String,
}

impl OpenCodeProvider {
    pub fn new() -> Result<Self, ProviderError> {
        let client = Client::builder().timeout(Duration::from_secs(60)).build()?;

        Ok(Self {
            client,
            base_url: OPENCODE_API.to_string(),
        })
    }

    fn convert_message(msg: &ChatMessage) -> OpenCodeMessage {
        let content = match &msg.content {
            MessageContent::Text(text) => text.clone(),
            MessageContent::Parts(parts) => {
                // Extract text from parts (simplified)
                parts
                    .iter()
                    .filter_map(|part| match part {
                        crate::llm::types::ContentPart::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        };

        OpenCodeMessage {
            role: msg.role.clone(),
            content,
            reasoning: None,
            reasoning_content: None,
        }
    }

    async fn fetch_free_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        let response = self
            .client
            .get(MODELS_DEV_API)
            .header("User-Agent", "dx-onboard")
            .send()
            .await?
            .json::<Value>()
            .await?;

        let mut models = Vec::new();

        let provider_data = response
            .get("opencode")
            .ok_or_else(|| ProviderError::InvalidResponse {
                provider: self.id().to_string(),
                detail: "models.dev payload missing 'opencode' provider".to_string(),
            })?;

        let provider_models = provider_data
            .get("models")
            .and_then(|m| m.as_object())
            .ok_or_else(|| ProviderError::InvalidResponse {
                provider: self.id().to_string(),
                detail: "models.dev payload missing 'opencode.models' map".to_string(),
            })?;

        for (model_id, model_data) in provider_models {
            if model_data
                .get("status")
                .and_then(|v| v.as_str())
                .is_some_and(|status| status.eq_ignore_ascii_case("deprecated"))
            {
                continue;
            }

            let Some(cost) = model_data.get("cost") else {
                continue;
            };

            let input_cost = cost.get("input").and_then(|v| v.as_f64());
            let output_cost = cost.get("output").and_then(|v| v.as_f64());

            if input_cost != Some(0.0) || output_cost != Some(0.0) {
                continue;
            }

            // OpenCode Zen serves different models behind different OpenAI-ish endpoints.
            // Our DX onboarding provider currently speaks `chat/completions` only.
            // Models like GPT-* are served via `/responses`, so skip them here.
            if model_id.starts_with("gpt-") {
                continue;
            }

            let name = model_data
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(model_id);

            let context_window = model_data
                .get("limit")
                .and_then(|l| l.get("context"))
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);

            let max_output = model_data
                .get("limit")
                .and_then(|l| l.get("output"))
                .and_then(|v| v.as_u64())
                .map(|v| v as u32);

            let supports_tools = model_data
                .get("tool_call")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let supports_vision = model_data
                .get("modalities")
                .and_then(|m| m.get("input"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .any(|mode| mode.eq_ignore_ascii_case("image"))
                })
                .unwrap_or(false);

            let supports_audio = model_data
                .get("modalities")
                .and_then(|m| m.get("input"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .any(|mode| mode.eq_ignore_ascii_case("audio"))
                })
                .unwrap_or(false);

            let display_name = if FREE_MODELS.contains(&model_id.as_str()) {
                Some(format!("{} (recommended)", name))
            } else {
                Some(name.to_string())
            };

            models.push(ModelInfo {
                id: model_id.to_string(),
                display_name,
                context_window,
                max_output_tokens: max_output,
                supports_tools,
                supports_vision,
                supports_audio,
            });
        }

        models.sort_by(|a, b| a.id.cmp(&b.id));
        models.dedup_by(|a, b| a.id == b.id);

        Ok(models)
    }
}

impl Default for OpenCodeProvider {
    fn default() -> Self {
        Self::new().expect("Failed to create OpenCode provider")
    }
}

#[async_trait]
impl LlmProvider for OpenCodeProvider {
    fn id(&self) -> &str {
        "opencode"
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn api_key(&self) -> &str {
        "public" // OpenCode's public key for free models
    }

    fn metadata(&self) -> &ProviderMetadata {
        static METADATA: std::sync::OnceLock<ProviderMetadata> = std::sync::OnceLock::new();
        METADATA.get_or_init(|| ProviderMetadata {
            id: "opencode".to_string(),
            name: "OpenCode (Free Models)".to_string(),
            category: "free".to_string(),
            auth_requirement: AuthRequirement::None,
            capabilities: ProviderCapabilities {
                chat: true,
                streaming: true,
                tools: true,
                vision: false,
                audio_input: false,
                audio_output: false,
                model_listing: true,
            },
            rate_limits: None,
            docs_url: Some("https://opencode.ai/docs/zen/".to_string()),
            website: Some("https://opencode.ai".to_string()),
        })
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let messages: Vec<OpenCodeMessage> =
            request.messages.iter().map(Self::convert_message).collect();

        let opencode_request = OpenCodeChatRequest {
            model: request.model.clone(),
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: Some(false),
        };

        let response = self
            .client
            .post(&format!("{}/chat/completions", self.base_url))
            .header("Authorization", "Bearer public")
            .header("Content-Type", "application/json")
            .json(&opencode_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::HttpStatus {
                provider: self.id().to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let opencode_response: OpenCodeChatResponse = response.json().await.map_err(|e| {
            ProviderError::InvalidResponse {
                provider: self.id().to_string(),
                detail: e.to_string(),
            }
        })?;

        let choice = opencode_response
            .choices
            .first()
            .ok_or_else(|| ProviderError::InvalidResponse {
                provider: self.id().to_string(),
                detail: "No choices in response".to_string(),
            })?;

        let content = if !choice.message.content.trim().is_empty() {
            choice.message.content.clone()
        } else if let Some(reasoning) = choice.message.reasoning_content.as_deref()
            && !reasoning.trim().is_empty()
        {
            reasoning.to_string()
        } else if let Some(reasoning) = choice.message.reasoning.as_deref()
            && !reasoning.trim().is_empty()
        {
            reasoning.to_string()
        } else {
            String::new()
        };

        Ok(ChatResponse {
            id: opencode_response.id,
            model: opencode_response.model,
            content,
            finish_reason: choice.finish_reason.clone(),
            usage: opencode_response.usage.map(|u| UsageInfo {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
            raw: serde_json::json!({}),
        })
    }

    async fn stream(&self, request: ChatRequest) -> Result<ProviderStream, ProviderError> {
        let provider_id = self.id().to_string();

        let messages: Vec<OpenCodeMessage> =
            request.messages.iter().map(Self::convert_message).collect();

        let opencode_request = OpenCodeChatRequest {
            model: request.model.clone(),
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: Some(true),
        };

        let response = self
            .client
            .post(&format!("{}/chat/completions", self.base_url))
            .header("Authorization", "Bearer public")
            .header("Content-Type", "application/json")
            .json(&opencode_request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::HttpStatus {
                provider: self.id().to_string(),
                status: status.as_u16(),
                body,
            });
        }

        let stream = response.bytes_stream();
        let mapped_stream = stream.map(move |result| {
            let bytes = match result {
                Ok(bytes) => bytes,
                Err(e) => return Err(ProviderError::Transport(e)),
            };

            let text = String::from_utf8_lossy(&bytes);

            // Parse SSE format
            for line in text.lines() {
                if line.starts_with("data: ") {
                    let json_str = &line[6..];
                    if json_str == "[DONE]" {
                        continue;
                    }

                    if let Ok(chunk) = serde_json::from_str::<OpenCodeStreamChunk>(json_str) {
                        if let Some(choice) = chunk.choices.first() {
                            if let Some(content) = &choice.delta.content {
                                return Ok(ChatChunk {
                                    id: chunk.id,
                                    model: chunk.model,
                                    delta: content.clone(),
                                    finish_reason: choice.finish_reason.clone(),
                                    raw: serde_json::json!({}),
                                });
                            }
                        }
                    }
                }
            }

            Err(ProviderError::InvalidResponse {
                provider: provider_id.clone(),
                detail: "Invalid stream chunk".to_string(),
            })
        });

        Ok(Box::pin(mapped_stream))
    }

    async fn get_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        self.fetch_free_models().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_free_models() {
        let provider = OpenCodeProvider::new().unwrap();
        let models = provider.get_models().await.unwrap();

        assert!(!models.is_empty(), "Should find free models");
        assert!(
            models.len() >= FREE_MODELS.len(),
            "Should include at least the recommended free models"
        );

        for model in &models {
            println!("Free model: {} ({})", model.display_name.as_ref().unwrap(), model.id);
        }
    }
}
```
