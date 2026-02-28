use crate::llm::copilot::{CopilotApiSurface, endpoint_for_model, model_api_surface};
use crate::llm::error::{ProviderError, RateLimitError};
use crate::llm::provider::{LlmProvider, ProviderStream};
use crate::llm::types::{
    AuthRequirement, ChatChunk, ChatRequest, ChatResponse, MessageContent, ModelInfo,
    ProviderCapabilities, ProviderMetadata, UsageInfo,
};
use async_stream::try_stream;
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub struct AzureOpenAiProvider {
    id: String,
    base_url: String,
    api_key: String,
    deployment: String,
    model: String,
    api_version: String,
    metadata: ProviderMetadata,
    client: Client,
}

#[derive(Debug, Clone)]
pub struct VertexAiProvider {
    id: String,
    base_url: String,
    access_token: String,
    model: String,
    metadata: ProviderMetadata,
    client: Client,
}

#[derive(Debug, Clone)]
pub struct GitHubCopilotProvider {
    id: String,
    base_url: String,
    access_token: String,
    metadata: ProviderMetadata,
    client: Client,
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

impl AzureOpenAiProvider {
    pub fn new(
        resource: impl Into<String>,
        deployment: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        let resource = resource.into();
        let deployment = deployment.into();
        let api_version = "2024-10-21".to_string();
        let base_url = format!("https://{}.openai.azure.com", resource);
        let model = model.into();

        Self {
            id: "azure-openai".to_string(),
            base_url,
            api_key: api_key.into(),
            deployment,
            model,
            api_version,
            metadata: ProviderMetadata {
                id: "azure-openai".to_string(),
                name: "Azure OpenAI".to_string(),
                category: "major-cloud".to_string(),
                auth_requirement: AuthRequirement::HeaderApiKey,
                capabilities: ProviderCapabilities {
                    chat: true,
                    streaming: true,
                    tools: true,
                    vision: true,
                    audio_input: true,
                    audio_output: false,
                    model_listing: true,
                },
                rate_limits: None,
                docs_url: Some("https://learn.microsoft.com/azure/ai-services/openai".to_string()),
                website: Some("https://azure.microsoft.com".to_string()),
            },
            client: Client::new(),
        }
    }

    pub fn with_api_version(mut self, api_version: impl Into<String>) -> Self {
        self.api_version = api_version.into();
        self
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_string();
        self
    }

    pub fn with_client(mut self, client: Client) -> Self {
        self.client = client;
        self
    }

    fn chat_url(&self) -> String {
        format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.base_url, self.deployment, self.api_version
        )
    }

    fn auth_headers(&self) -> Result<HeaderMap, ProviderError> {
        let mut headers = HeaderMap::new();
        let value =
            HeaderValue::from_str(&self.api_key).map_err(|err| ProviderError::InvalidConfig {
                provider: self.id.clone(),
                detail: format!("invalid Azure api-key header: {err}"),
            })?;
        headers.insert(HeaderName::from_static("api-key"), value);
        Ok(headers)
    }
}

impl VertexAiProvider {
    pub fn new(
        project: impl Into<String>,
        location: impl Into<String>,
        access_token: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        let project = project.into();
        let location = location.into();
        let base_url = format!(
            "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/endpoints/openapi",
            location, project, location
        );
        let model = model.into();

        Self {
            id: "vertex-ai".to_string(),
            base_url,
            access_token: access_token.into(),
            model,
            metadata: ProviderMetadata {
                id: "vertex-ai".to_string(),
                name: "Google Vertex AI".to_string(),
                category: "major-cloud".to_string(),
                auth_requirement: AuthRequirement::OAuth,
                capabilities: ProviderCapabilities {
                    chat: true,
                    streaming: true,
                    tools: true,
                    vision: true,
                    audio_input: true,
                    audio_output: false,
                    model_listing: true,
                },
                rate_limits: None,
                docs_url: Some("https://cloud.google.com/vertex-ai/docs".to_string()),
                website: Some("https://cloud.google.com/vertex-ai".to_string()),
            },
            client: Client::new(),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_string();
        self
    }

    pub fn with_client(mut self, client: Client) -> Self {
        self.client = client;
        self
    }

    fn auth_headers(&self) -> Result<HeaderMap, ProviderError> {
        bearer_headers(&self.id, &self.access_token)
    }
}

impl GitHubCopilotProvider {
    pub fn new(access_token: impl Into<String>, model: impl Into<String>) -> Self {
        let _ = model.into();
        let base_url = std::env::var("GITHUB_COPILOT_BASE_URL")
            .ok()
            .and_then(|value| {
                let trimmed = value.trim().to_string();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            })
            .unwrap_or_else(|| "https://api.individual.githubcopilot.com".to_string());
        Self {
            id: "github_copilot".to_string(),
            base_url,
            access_token: access_token.into(),
            metadata: ProviderMetadata {
                id: "github_copilot".to_string(),
                name: "GitHub Copilot".to_string(),
                category: "enterprise".to_string(),
                auth_requirement: AuthRequirement::OAuth,
                capabilities: ProviderCapabilities {
                    chat: true,
                    streaming: true,
                    tools: true,
                    vision: true,
                    audio_input: false,
                    audio_output: false,
                    model_listing: true,
                },
                rate_limits: None,
                docs_url: Some("https://docs.github.com/copilot".to_string()),
                website: Some("https://github.com/features/copilot".to_string()),
            },
            client: Client::new(),
        }
    }

    pub fn from_env() -> Result<Self, ProviderError> {
        let token = crate::llm::copilot::retrieve_copilot_token().ok_or_else(|| {
            ProviderError::InvalidConfig {
                provider: "github_copilot".to_string(),
                detail: "missing GITHUB_COPILOT_TOKEN (Copilot service token). Run Copilot bootstrap to fetch a fresh token.".to_string(),
            }
        })?;

        let model = std::env::var("GITHUB_COPILOT_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());

        Ok(Self::new(token, model))
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_string();
        self
    }

    pub fn with_client(mut self, client: Client) -> Self {
        self.client = client;
        self
    }

    fn auth_headers(&self) -> Result<HeaderMap, ProviderError> {
        let mut headers = bearer_headers(&self.id, &self.access_token)?;
        headers.insert(
            HeaderName::from_static("editor-version"),
            HeaderValue::from_static("DX-CLI/1.0.0"),
        );
        headers.insert(
            HeaderName::from_static("copilot-integration-id"),
            HeaderValue::from_static("dx-cli"),
        );
        Ok(headers)
    }

    fn supplemental_models(&self) -> Vec<ModelInfo> {
        let models = vec![
            ("raptor-mini", "Raptor mini", true, true),
            ("gpt-5.3-codex", "GPT-5.3-Codex", true, true),
            ("gpt-5.2-codex", "GPT-5.2-Codex", true, true),
            ("gpt-5.1-codex-mini", "GPT-5.1-Codex-Mini", true, true),
            ("gpt-5.1-codex-max", "GPT-5.1-Codex-Max", true, true),
            ("gpt-5.1-codex", "GPT-5.1-Codex", true, true),
            ("gpt-5.2", "GPT-5.2", true, true),
            ("gpt-5.1", "GPT-5.1", true, true),
            ("gpt-5", "GPT-5", true, true),
            ("gpt-5-mini", "GPT-5 mini", true, true),
            ("gpt-4.1", "GPT-4.1", true, true),
            ("gpt-4o", "GPT-4o", true, true),
            ("gpt-4o-mini", "GPT-4o Mini", true, false),
            ("claude-opus-4.6", "Claude Opus 4.6", true, true),
            ("claude-sonnet-4.6", "Claude Sonnet 4.6", true, true),
            ("claude-sonnet-4.5", "Claude Sonnet 4.5", true, true),
            ("claude-haiku-4.5", "Claude Haiku 4.5", true, true),
            ("gemini-3-pro-preview", "Gemini 3 Pro", true, true),
            ("gemini-3-flash-preview", "Gemini 3 Flash", true, true),
            ("gemini-2.5-pro", "Gemini 2.5 Pro", true, true),
            ("grok-code-fast-1", "Grok Code Fast 1", true, false),
        ];

        models
            .into_iter()
            .map(|(id, name, supports_tools, supports_vision)| ModelInfo {
                id: id.to_string(),
                display_name: Some(format!("Copilot: {}", name)),
                context_window: None,
                max_output_tokens: None,
                supports_tools,
                supports_vision,
                supports_audio: false,
            })
            .collect()
    }

    async fn chat_with_surface(
        &self,
        request: ChatRequest,
        surface: CopilotApiSurface,
        endpoint: &str,
    ) -> Result<ChatResponse, ProviderError> {
        match surface {
            CopilotApiSurface::Responses => {
                copilot_responses_chat_impl(
                    &self.id,
                    &self.client,
                    endpoint,
                    self.auth_headers()?,
                    request,
                )
                .await
            }
            CopilotApiSurface::ChatCompletions => {
                chat_impl(&self.id, &self.client, endpoint, self.auth_headers()?, request).await
            }
        }
    }
}

fn bearer_headers(provider_id: &str, token: &str) -> Result<HeaderMap, ProviderError> {
    let mut headers = HeaderMap::new();
    let auth = format!("Bearer {}", token);
    let auth_value = HeaderValue::from_str(&auth).map_err(|err| ProviderError::InvalidConfig {
        provider: provider_id.to_string(),
        detail: format!("invalid auth header: {err}"),
    })?;
    headers.insert(AUTHORIZATION, auth_value);
    Ok(headers)
}

async fn validate_status(
    provider_id: &str,
    response: reqwest::Response,
) -> Result<reqwest::Response, ProviderError> {
    if response.status().is_success() {
        return Ok(response);
    }

    let status = response.status();
    if status.as_u16() == 429 {
        return Err(RateLimitError::Exceeded {
            provider: provider_id.to_string(),
        }
        .into());
    }

    let body = response.text().await.unwrap_or_default();
    Err(ProviderError::HttpStatus {
        provider: provider_id.to_string(),
        status: status.as_u16(),
        body,
    })
}

async fn chat_impl(
    provider_id: &str,
    client: &Client,
    endpoint: &str,
    headers: HeaderMap,
    mut request: ChatRequest,
) -> Result<ChatResponse, ProviderError> {
    request.stream = false;
    let response = client.post(endpoint).headers(headers).json(&request).send().await?;
    let response = validate_status(provider_id, response).await?;
    let raw: Value = response.json().await?;
    let parsed: OpenAiChatResponse = serde_json::from_value(raw.clone())?;

    let first = parsed.choices.first().ok_or_else(|| ProviderError::InvalidResponse {
        provider: provider_id.to_string(),
        detail: "missing choices[0] in chat response".to_string(),
    })?;

    let content = first.message.as_ref().and_then(|msg| msg.content.clone()).unwrap_or_default();

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

async fn stream_impl(
    provider_id: String,
    client: Client,
    endpoint: String,
    headers: HeaderMap,
    mut request: ChatRequest,
) -> Result<ProviderStream, ProviderError> {
    request.stream = true;

    let response = client.post(endpoint).headers(headers).json(&request).send().await?;
    let response = validate_status(&provider_id, response).await?;

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
                pending.drain(..=newline_index);

                if line.is_empty() || !line.starts_with("data:") {
                    continue;
                }

                let payload = line.trim_start_matches("data:").trim();
                if payload == "[DONE]" {
                    break;
                }

                let raw: Value = serde_json::from_str(payload)?;
                let parsed: OpenAiStreamChunk = serde_json::from_value(raw.clone())?;
                let first = parsed.choices.first().ok_or_else(|| ProviderError::InvalidResponse {
                    provider: provider_id.clone(),
                    detail: "missing choices[0] in stream chunk".to_string(),
                })?;

                let delta = first
                    .delta
                    .as_ref()
                    .and_then(|d| d.content.clone())
                    .unwrap_or_default();

                yield ChatChunk {
                    id: parsed.id,
                    model: parsed.model,
                    delta,
                    finish_reason: first.finish_reason.clone(),
                    raw,
                };
            }
        }
    };

    Ok(Box::pin(stream))
}

fn flatten_message_content(content: &MessageContent) -> String {
    match content {
        MessageContent::Text(text) => text.clone(),
        MessageContent::Parts(parts) => parts
            .iter()
            .filter_map(|part| match part {
                crate::llm::types::ContentPart::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

async fn copilot_responses_chat_impl(
    provider_id: &str,
    client: &Client,
    endpoint: &str,
    headers: HeaderMap,
    mut request: ChatRequest,
) -> Result<ChatResponse, ProviderError> {
    request.stream = false;

    let input_messages = request
        .messages
        .iter()
        .map(|message| {
            json!({
                "role": message.role,
                "content": [{
                    "type": "input_text",
                    "text": flatten_message_content(&message.content),
                }]
            })
        })
        .collect::<Vec<_>>();

    let body = json!({
        "model": request.model,
        "input": input_messages,
        "temperature": request.temperature,
        "max_output_tokens": request.max_tokens,
        "top_p": request.top_p,
    });

    let response = client.post(endpoint).headers(headers).json(&body).send().await?;
    let response = validate_status(provider_id, response).await?;
    let raw: Value = response.json().await?;

    let content = raw
        .get("output_text")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            raw.get("output").and_then(Value::as_array).and_then(|output| {
                output.iter().find_map(|item| {
                    item.get("content").and_then(Value::as_array).and_then(|parts| {
                        parts.iter().find_map(|part| {
                            part.get("text").and_then(Value::as_str).map(str::to_string)
                        })
                    })
                })
            })
        })
        .unwrap_or_default();

    let usage = raw.get("usage");
    Ok(ChatResponse {
        id: raw.get("id").and_then(Value::as_str).map(str::to_string),
        model: raw.get("model").and_then(Value::as_str).map(str::to_string),
        content,
        finish_reason: raw.get("status").and_then(Value::as_str).map(str::to_string),
        usage: Some(UsageInfo {
            prompt_tokens: usage
                .and_then(|value| value.get("input_tokens"))
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok()),
            completion_tokens: usage
                .and_then(|value| value.get("output_tokens"))
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok()),
            total_tokens: usage
                .and_then(|value| value.get("total_tokens"))
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok()),
        }),
        raw,
    })
}

#[async_trait]
impl LlmProvider for AzureOpenAiProvider {
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

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        chat_impl(&self.id, &self.client, &self.chat_url(), self.auth_headers()?, request).await
    }

    async fn stream(&self, request: ChatRequest) -> Result<ProviderStream, ProviderError> {
        stream_impl(
            self.id.clone(),
            self.client.clone(),
            self.chat_url(),
            self.auth_headers()?,
            request,
        )
        .await
    }

    async fn get_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(vec![ModelInfo {
            id: self.model.clone(),
            display_name: Some(self.deployment.clone()),
            context_window: None,
            max_output_tokens: None,
            supports_tools: true,
            supports_vision: true,
            supports_audio: true,
        }])
    }
}

#[async_trait]
impl LlmProvider for VertexAiProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn api_key(&self) -> &str {
        &self.access_token
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        chat_impl(
            &self.id,
            &self.client,
            &format!("{}/chat/completions", self.base_url),
            self.auth_headers()?,
            request,
        )
        .await
    }

    async fn stream(&self, request: ChatRequest) -> Result<ProviderStream, ProviderError> {
        stream_impl(
            self.id.clone(),
            self.client.clone(),
            format!("{}/chat/completions", self.base_url),
            self.auth_headers()?,
            request,
        )
        .await
    }

    async fn get_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        Ok(vec![ModelInfo {
            id: self.model.clone(),
            display_name: Some("Vertex model".to_string()),
            context_window: None,
            max_output_tokens: None,
            supports_tools: true,
            supports_vision: true,
            supports_audio: true,
        }])
    }
}

#[async_trait]
impl LlmProvider for GitHubCopilotProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn base_url(&self) -> &str {
        &self.base_url
    }

    fn api_key(&self) -> &str {
        &self.access_token
    }

    fn metadata(&self) -> &ProviderMetadata {
        &self.metadata
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        let preferred_surface = model_api_surface(&request.model);
        let base = self.base_url.trim_end_matches('/');

        let mut attempts: Vec<(CopilotApiSurface, String)> = Vec::new();
        attempts.push((preferred_surface, endpoint_for_model(&self.base_url, &request.model)));

        match preferred_surface {
            CopilotApiSurface::Responses => {
                attempts.push((CopilotApiSurface::Responses, format!("{}/responses", base)));
                attempts.push((
                    CopilotApiSurface::ChatCompletions,
                    format!("{}/v1/chat/completions", base),
                ));
                attempts.push((
                    CopilotApiSurface::ChatCompletions,
                    format!("{}/chat/completions", base),
                ));
            }
            CopilotApiSurface::ChatCompletions => {
                attempts.push((
                    CopilotApiSurface::ChatCompletions,
                    format!("{}/chat/completions", base),
                ));
                attempts.push((CopilotApiSurface::Responses, format!("{}/v1/responses", base)));
                attempts.push((CopilotApiSurface::Responses, format!("{}/responses", base)));
            }
        }

        let mut last_error: Option<ProviderError> = None;
        for (surface, endpoint) in attempts {
            match self.chat_with_surface(request.clone(), surface, &endpoint).await {
                Ok(response) => return Ok(response),
                Err(ProviderError::HttpStatus { status: 404, .. }) => {
                    last_error = Some(ProviderError::HttpStatus {
                        provider: self.id.clone(),
                        status: 404,
                        body: format!("404 page not found (endpoint: {})", endpoint),
                    });
                }
                Err(err) => return Err(err),
            }
        }

        Err(last_error.unwrap_or_else(|| ProviderError::InvalidResponse {
            provider: self.id.clone(),
            detail: "all Copilot endpoint attempts failed".to_string(),
        }))
    }

    async fn stream(&self, request: ChatRequest) -> Result<ProviderStream, ProviderError> {
        let endpoint = format!("{}/v1/chat/completions", self.base_url);
        stream_impl(self.id.clone(), self.client.clone(), endpoint, self.auth_headers()?, request)
            .await
    }

    async fn get_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        // GitHub Copilot models endpoint: https://api.githubcopilot.com/models
        let endpoint = format!("{}/models", self.base_url);
        let mut headers = self.auth_headers()?;
        headers.insert(
            HeaderName::from_static("accept"),
            HeaderValue::from_static("application/json"),
        );

        let response = self.client.get(&endpoint).headers(headers).send().await;

        if let Ok(response) = response {
            if response.status().is_success() {
                if let Ok(raw_json) = response.json::<Value>().await {
                    // Parse the response format: {"data": [...], "object": "list"}
                    if let Some(data) = raw_json.get("data").and_then(Value::as_array) {
                        let mut models = data
                            .iter()
                            .filter_map(|model| {
                                let id = model.get("id")?.as_str()?.to_string();
                                if id.trim().is_empty() {
                                    return None;
                                }

                                let name = model
                                    .get("name")
                                    .and_then(Value::as_str)
                                    .or_else(|| model.get("display_name").and_then(Value::as_str));

                                let supports =
                                    model.get("capabilities").and_then(|caps| caps.get("supports"));

                                let supports_tools = supports
                                    .and_then(|value| value.get("tool_calls"))
                                    .and_then(Value::as_bool)
                                    .unwrap_or(self.metadata.capabilities.tools);
                                let supports_vision = supports
                                    .and_then(|value| value.get("vision"))
                                    .and_then(Value::as_bool)
                                    .unwrap_or(self.metadata.capabilities.vision);

                                Some(ModelInfo {
                                    id: id.clone(),
                                    display_name: Some(format!(
                                        "Copilot: {}",
                                        name.unwrap_or(id.as_str())
                                    )),
                                    context_window: None,
                                    max_output_tokens: None,
                                    supports_tools,
                                    supports_vision,
                                    supports_audio: false,
                                })
                            })
                            .collect::<Vec<_>>();

                        // Sort and deduplicate
                        models.sort_by(|a, b| a.id.cmp(&b.id));
                        models.dedup_by(|a, b| a.id == b.id);

                        models.extend(self.supplemental_models());
                        models.sort_by(|a, b| a.id.cmp(&b.id));
                        models.dedup_by(|a, b| a.id == b.id);

                        if !models.is_empty() {
                            return Ok(models);
                        }
                    }
                }
            } else {
                let status = response.status().as_u16();
                let body = response.text().await.unwrap_or_default();
                if status == 401 || status == 403 {
                    return Err(ProviderError::InvalidConfig {
                        provider: self.id.clone(),
                        detail: format!(
                            "Copilot token is not authorized for chat/models (http {status}). DX expects a fresh Copilot *service token* in GITHUB_COPILOT_TOKEN (short-lived). Re-run `dx onboard` Copilot setup to re-auth and fetch a new token; do not reuse a persisted/stored token. Raw response: {body}"
                        ),
                    });
                }
            }
        }

        Ok(self.supplemental_models())
    }
}

pub fn register_enterprise_custom_from_env() -> Vec<Box<dyn LlmProvider>> {
    let mut providers: Vec<Box<dyn LlmProvider>> = Vec::new();

    if let (Ok(resource), Ok(deployment), Ok(api_key)) = (
        std::env::var("AZURE_OPENAI_RESOURCE"),
        std::env::var("AZURE_OPENAI_DEPLOYMENT"),
        std::env::var("AZURE_OPENAI_API_KEY"),
    ) {
        let model = std::env::var("AZURE_OPENAI_MODEL").unwrap_or_else(|_| deployment.clone());
        let api_version =
            std::env::var("AZURE_OPENAI_API_VERSION").unwrap_or_else(|_| "2024-10-21".to_string());
        providers.push(Box::new(
            AzureOpenAiProvider::new(resource, deployment, api_key, model)
                .with_api_version(api_version),
        ));
    }

    if let (Ok(project), Ok(location), Ok(token)) = (
        std::env::var("VERTEX_AI_PROJECT"),
        std::env::var("VERTEX_AI_LOCATION"),
        std::env::var("VERTEX_AI_ACCESS_TOKEN"),
    ) {
        let model =
            std::env::var("VERTEX_AI_MODEL").unwrap_or_else(|_| "gemini-2.5-flash".to_string());
        providers.push(Box::new(VertexAiProvider::new(project, location, token, model)));
    }

    if let Ok(provider) = GitHubCopilotProvider::from_env() {
        providers.push(Box::new(provider));
    }

    providers
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::provider::LlmProvider;
    use crate::llm::types::{ChatMessage, MessageContent};
    use httpmock::Method::{GET, POST};
    use httpmock::MockServer;
    use std::collections::BTreeMap;

    fn sample_request() -> ChatRequest {
        ChatRequest {
            model: "test-model".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: MessageContent::Text("hello".to_string()),
                name: None,
            }],
            temperature: None,
            max_tokens: None,
            top_p: None,
            stop: None,
            tools: None,
            tool_choice: None,
            stream: false,
            extra: Some(BTreeMap::new()),
        }
    }

    #[tokio::test]
    async fn azure_uses_api_key_header() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/openai/deployments/gpt/chat/completions")
                .query_param("api-version", "2024-10-21")
                .header("api-key", "azure-token");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    r#"{"id":"1","model":"gpt","choices":[{"message":{"content":"ok"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}"#,
                );
        });

        let provider = AzureOpenAiProvider::new("resource", "gpt", "azure-token", "gpt")
            .with_base_url(server.base_url());
        let response = provider.chat(sample_request()).await.expect("chat response");

        mock.assert();
        assert_eq!(response.content, "ok");
    }

    #[tokio::test]
    async fn vertex_uses_bearer_token() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/chat/completions")
                .header("authorization", "Bearer vertex-token");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    r#"{"id":"2","model":"vertex","choices":[{"message":{"content":"ok-v"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}"#,
                );
        });

        let provider = VertexAiProvider::new("p", "us-central1", "vertex-token", "gemini")
            .with_base_url(server.base_url());
        let response = provider.chat(sample_request()).await.expect("chat response");

        mock.assert();
        assert_eq!(response.content, "ok-v");
    }

    #[tokio::test]
    async fn copilot_uses_bearer_token() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/v1/chat/completions")
                .header("authorization", "Bearer gh-token");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    r#"{"id":"3","model":"copilot","choices":[{"message":{"content":"ok-c"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}"#,
                );
        });

        let provider =
            GitHubCopilotProvider::new("gh-token", "gpt-4o-mini").with_base_url(server.base_url());
        let response = provider.chat(sample_request()).await.expect("chat response");

        mock.assert();
        assert_eq!(response.content, "ok-c");
    }

    #[tokio::test]
    async fn copilot_gpt5_routes_to_responses_api() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/v1/responses")
                .header("authorization", "Bearer gh-token");
            then.status(200).header("content-type", "application/json").body(
                r#"{"id":"resp_1","model":"gpt-5","status":"completed","output_text":"ok-r","usage":{"input_tokens":2,"output_tokens":3,"total_tokens":5}}"#,
            );
        });

        let mut request = sample_request();
        request.model = "gpt-5".to_string();
        let provider =
            GitHubCopilotProvider::new("gh-token", "gpt-5").with_base_url(server.base_url());
        let response = provider.chat(request).await.expect("chat response");

        mock.assert();
        assert_eq!(response.content, "ok-r");
    }

    #[tokio::test]
    async fn copilot_get_models_fetches_catalog() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/models").header("authorization", "Bearer gh-token");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"data":[{"id":"gpt-4o"},{"id":"gpt-5"},{"id":"claude-sonnet-4.5"}]}"#);
        });

        let provider =
            GitHubCopilotProvider::new("gh-token", "gpt-4o").with_base_url(server.base_url());
        let models = provider.get_models().await.expect("model list");

        mock.assert();
        assert!(models.iter().any(|item| item.id == "gpt-4o"));
        assert!(models.iter().any(|item| item.id == "gpt-5"));
        assert!(models.iter().any(|item| item.id == "claude-sonnet-4.5"));
    }

    #[tokio::test]
    async fn copilot_get_models_accepts_id_only_entries() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/models").header("authorization", "Bearer gh-token");
            then.status(200).header("content-type", "application/json").body(
                r#"{"data":[{"id":"raptor-mini"},{"id":"gpt-5.3-codex","name":"GPT-5.3-Codex"}]}"#,
            );
        });

        let provider =
            GitHubCopilotProvider::new("gh-token", "gpt-4o").with_base_url(server.base_url());
        let models = provider.get_models().await.expect("model list");

        mock.assert();
        assert!(models.iter().any(|item| item.id == "raptor-mini"));
        assert!(models.iter().any(|item| item.id == "gpt-5.3-codex"));
    }

    #[tokio::test]
    async fn copilot_chat_falls_back_to_responses_on_404() {
        let server = MockServer::start();
        let chat_404 = server.mock(|when, then| {
            when.method(POST)
                .path("/v1/chat/completions")
                .header("authorization", "Bearer gh-token");
            then.status(404).body("404 page not found");
        });
        let responses_ok = server.mock(|when, then| {
            when.method(POST)
                .path("/v1/responses")
                .header("authorization", "Bearer gh-token");
            then.status(200).header("content-type", "application/json").body(
                r#"{"id":"resp_2","model":"gpt-4","status":"completed","output_text":"fallback-ok","usage":{"input_tokens":2,"output_tokens":3,"total_tokens":5}}"#,
            );
        });

        let mut request = sample_request();
        request.model = "gpt-4".to_string();
        let provider =
            GitHubCopilotProvider::new("gh-token", "gpt-4").with_base_url(server.base_url());

        let response = provider.chat(request).await.expect("chat response");

        chat_404.assert();
        responses_ok.assert();
        assert_eq!(response.content, "fallback-ok");
    }

    #[test]
    fn copilot_from_env_reads_token() {
        unsafe {
            std::env::remove_var("GITHUB_TOKEN");
            std::env::set_var("GITHUB_COPILOT_TOKEN", "test-token");
            std::env::set_var("GITHUB_COPILOT_MODEL", "gpt-5");
        }

        let provider = GitHubCopilotProvider::from_env().expect("provider from env");
        assert_eq!(provider.access_token, "test-token");

        unsafe {
            std::env::remove_var("GITHUB_COPILOT_TOKEN");
            std::env::remove_var("GITHUB_COPILOT_MODEL");
        }
    }
}
