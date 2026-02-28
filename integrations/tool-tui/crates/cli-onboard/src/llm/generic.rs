use crate::llm::error::{ProviderError, RateLimitError};
use crate::llm::provider::{LlmProvider, ProviderStream};
use crate::llm::types::{
    AuthRequirement, ChatChunk, ChatRequest, ChatResponse, ModelInfo, ProviderCapabilities,
    ProviderMetadata, RateLimitInfo, UsageInfo,
};
use async_stream::try_stream;
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct GenericProvider {
    id: String,
    base_url: String,
    api_key: String,
    metadata: ProviderMetadata,
    custom_headers: BTreeMap<String, String>,
    client: Client,
}

impl GenericProvider {
    pub fn new(
        id: impl Into<String>,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        let id = id.into();
        let metadata = ProviderMetadata {
            id: id.clone(),
            name: id.clone(),
            category: "openai-compatible".to_string(),
            auth_requirement: AuthRequirement::BearerToken,
            capabilities: ProviderCapabilities {
                chat: true,
                streaming: true,
                tools: true,
                vision: true,
                audio_input: true,
                audio_output: true,
                model_listing: true,
            },
            rate_limits: Some(RateLimitInfo::default()),
            docs_url: None,
            website: None,
        };

        Self {
            id,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key: api_key.into(),
            metadata,
            custom_headers: BTreeMap::new(),
            client: Client::new(),
        }
    }

    pub fn with_metadata(mut self, metadata: ProviderMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_custom_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_headers.insert(key.into(), value.into());
        self
    }

    pub fn with_client(mut self, client: Client) -> Self {
        self.client = client;
        self
    }

    fn auth_headers(&self) -> Result<HeaderMap, ProviderError> {
        let mut headers = HeaderMap::new();
        let auth = format!("Bearer {}", self.api_key);
        let auth_value =
            HeaderValue::from_str(&auth).map_err(|err| ProviderError::InvalidConfig {
                provider: self.id.clone(),
                detail: format!("invalid api key header: {err}"),
            })?;
        headers.insert(AUTHORIZATION, auth_value);

        for (key, value) in &self.custom_headers {
            let name = HeaderName::from_str(key).map_err(|err| ProviderError::InvalidConfig {
                provider: self.id.clone(),
                detail: format!("invalid custom header name `{key}`: {err}"),
            })?;
            let value =
                HeaderValue::from_str(value).map_err(|err| ProviderError::InvalidConfig {
                    provider: self.id.clone(),
                    detail: format!("invalid custom header value for `{key}`: {err}"),
                })?;
            headers.insert(name, value);
        }

        Ok(headers)
    }

    async fn validate_status(
        &self,
        response: reqwest::Response,
    ) -> Result<reqwest::Response, ProviderError> {
        if response.status().is_success() {
            return Ok(response);
        }

        let status = response.status();
        if status.as_u16() == 429 {
            return Err(RateLimitError::Exceeded {
                provider: self.id.clone(),
            }
            .into());
        }

        let body = response.text().await.unwrap_or_default();
        Err(ProviderError::HttpStatus {
            provider: self.id.clone(),
            status: status.as_u16(),
            body,
        })
    }
}

#[derive(Debug, Deserialize)]
struct OpenAiChoiceMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: Option<OpenAiChoiceMessage>,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    total_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChatResponse {
    id: Option<String>,
    model: Option<String>,
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamDelta {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChoice {
    delta: Option<OpenAiStreamDelta>,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChunk {
    id: Option<String>,
    model: Option<String>,
    choices: Vec<OpenAiStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiModelsResponse {
    data: Vec<OpenAiModel>,
}

#[derive(Debug, Deserialize)]
struct OpenAiModel {
    id: String,
}

#[async_trait]
impl LlmProvider for GenericProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn api_key(&self) -> &str {
        &self.api_key
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn chat(&self, mut request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        request.stream = false;

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .headers(self.auth_headers()?)
            .json(&request)
            .send()
            .await?;

        let response = self.validate_status(response).await?;
        let raw: Value = response.json().await?;
        let parsed: OpenAiChatResponse = serde_json::from_value(raw.clone())?;

        let first = parsed.choices.first().ok_or_else(|| ProviderError::InvalidResponse {
            provider: self.id.clone(),
            detail: "missing choices[0] in chat response".to_string(),
        })?;
        let content =
            first.message.as_ref().and_then(|msg| msg.content.clone()).unwrap_or_default();

        Ok(ChatResponse {
            id: parsed.id,
            model: parsed.model,
            content,
            finish_reason: first.finish_reason.clone(),
            usage: parsed.usage.map(|usage| UsageInfo {
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
                total_tokens: usage.total_tokens,
            }),
            raw,
        })
    }

    async fn stream(&self, mut request: ChatRequest) -> Result<ProviderStream, ProviderError> {
        request.stream = true;

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .headers(self.auth_headers()?)
            .json(&request)
            .send()
            .await?;
        let response = self.validate_status(response).await?;

        let byte_stream = response.bytes_stream();

        let stream = try_stream! {
            let mut pending = String::new();
            futures_util::pin_mut!(byte_stream);

            while let Some(chunk_result) = byte_stream.next().await {
                let chunk = chunk_result?;
                let text = String::from_utf8_lossy(&chunk);
                pending.push_str(&text);

                while let Some(newline_index) = pending.find('\n') {
                    let line = pending[..newline_index].trim().to_string();
                    pending = pending[newline_index + 1..].to_string();

                    if !line.starts_with("data:") {
                        continue;
                    }

                    let payload = line.trim_start_matches("data:").trim();
                    if payload == "[DONE]" || payload.is_empty() {
                        continue;
                    }

                    let raw: Value = serde_json::from_str(payload)?;
                    let parsed: OpenAiStreamChunk = serde_json::from_value(raw.clone())?;

                    if let Some(choice) = parsed.choices.first() {
                        let delta = choice
                            .delta
                            .as_ref()
                            .and_then(|d| d.content.clone())
                            .unwrap_or_default();

                        yield ChatChunk {
                            id: parsed.id.clone(),
                            model: parsed.model.clone(),
                            delta,
                            finish_reason: choice.finish_reason.clone(),
                            raw,
                        };
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    async fn get_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        let response = self
            .client
            .get(format!("{}/v1/models", self.base_url))
            .headers(self.auth_headers()?)
            .send()
            .await?;
        let response = self.validate_status(response).await?;

        let parsed: OpenAiModelsResponse = response.json().await?;
        Ok(parsed
            .data
            .into_iter()
            .map(|model| ModelInfo {
                id: model.id,
                display_name: None,
                context_window: None,
                max_output_tokens: None,
                supports_tools: false,
                supports_vision: false,
                supports_audio: false,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::types::{ChatMessage, MessageContent};
    use futures_util::StreamExt;
    use httpmock::Method::{GET, POST};
    use httpmock::MockServer;

    fn build_request(model: &str) -> ChatRequest {
        ChatRequest {
            model: model.to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: MessageContent::Text("hello".to_string()),
                name: None,
            }],
            temperature: Some(0.7),
            max_tokens: Some(64),
            top_p: None,
            stop: None,
            tools: None,
            tool_choice: None,
            stream: false,
            extra: None,
        }
    }

    #[tokio::test]
    async fn chat_parses_openai_shape() {
        let server = MockServer::start_async().await;
        let _mock = server
            .mock_async(|when, then| {
                when.method(POST).path("/v1/chat/completions");
                then.status(200)
                    .header("content-type", "application/json")
                    .body(
                        r#"{
                          "id": "chatcmpl_test",
                          "model": "test-model",
                          "choices": [{"message": {"content": "hello from provider"}, "finish_reason": "stop"}],
                          "usage": {"prompt_tokens": 3, "completion_tokens": 4, "total_tokens": 7}
                        }"#,
                    );
            })
            .await;

        let provider = GenericProvider::new("test", server.base_url(), "key");
        let response = provider.chat(build_request("test-model")).await.expect("chat response");
        assert_eq!(response.content, "hello from provider");
        assert_eq!(response.finish_reason.as_deref(), Some("stop"));
    }

    #[tokio::test]
    async fn get_models_returns_ids() {
        let server = MockServer::start_async().await;
        let _mock = server
            .mock_async(|when, then| {
                when.method(GET).path("/v1/models");
                then.status(200)
                    .header("content-type", "application/json")
                    .body(r#"{"data": [{"id": "a-model"}, {"id": "b-model"}]}"#);
            })
            .await;

        let provider = GenericProvider::new("test", server.base_url(), "key");
        let models = provider.get_models().await.expect("models response");
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "a-model");
        assert_eq!(models[1].id, "b-model");
    }

    #[tokio::test]
    async fn stream_parses_sse_chunks() {
        let server = MockServer::start_async().await;
        let _mock = server
            .mock_async(|when, then| {
                when.method(POST).path("/v1/chat/completions");
                then.status(200)
                    .header("content-type", "text/event-stream")
                    .body(
                        "data: {\"id\":\"c1\",\"model\":\"m1\",\"choices\":[{\"delta\":{\"content\":\"Hel\"},\"finish_reason\":null}]}\n\n\
                         data: {\"id\":\"c1\",\"model\":\"m1\",\"choices\":[{\"delta\":{\"content\":\"lo\"},\"finish_reason\":\"stop\"}]}\n\n\
                         data: [DONE]\n\n",
                    );
            })
            .await;

        let provider = GenericProvider::new("test", server.base_url(), "key");
        let stream = provider.stream(build_request("m1")).await.expect("stream");
        let chunks = stream.collect::<Vec<_>>().await;

        assert_eq!(chunks.len(), 2);
        let first = chunks[0].as_ref().expect("first chunk");
        let second = chunks[1].as_ref().expect("second chunk");
        assert_eq!(first.delta, "Hel");
        assert_eq!(second.delta, "lo");
        assert_eq!(second.finish_reason.as_deref(), Some("stop"));
    }
}
