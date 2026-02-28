use crate::llm::error::ProviderError;
use crate::llm::provider::{LlmProvider, ProviderStream};
use crate::llm::types::{ChatRequest, ChatResponse, MessageContent, ModelInfo, ProviderMetadata};
use async_trait::async_trait;
use genai::Client;
use genai::chat::{ChatMessage as GenAiChatMessage, ChatRequest as GenAiChatRequest};
use serde::Deserialize;
use serde_json::json;

#[derive(Clone)]
pub struct GenAiProvider {
    id: String,
    model: String,
    base_url: String,
    api_key: String,
    metadata: ProviderMetadata,
    client: Client,
}

impl GenAiProvider {
    pub fn new(
        id: impl Into<String>,
        model: impl Into<String>,
        metadata: ProviderMetadata,
    ) -> Self {
        Self {
            id: id.into(),
            model: model.into(),
            base_url: "genai://native".to_string(),
            api_key: "managed-by-genai-client".to_string(),
            metadata,
            client: Client::default(),
        }
    }

    fn to_genai_request(&self, request: ChatRequest) -> Result<GenAiChatRequest, ProviderError> {
        let mut messages = Vec::with_capacity(request.messages.len());

        for message in request.messages {
            let text = match message.content {
                MessageContent::Text(text) => text,
                MessageContent::Parts(parts) => {
                    let text_only = parts
                        .into_iter()
                        .filter_map(|part| match part {
                            crate::llm::types::ContentPart::Text { text } => Some(text),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    if text_only.is_empty() {
                        return Err(ProviderError::InvalidConfig {
                            provider: self.id.clone(),
                            detail: "genai adapter currently supports text parts only".to_string(),
                        });
                    }
                    text_only
                }
            };

            let msg = match message.role.as_str() {
                "system" => GenAiChatMessage::system(text),
                "assistant" => GenAiChatMessage::assistant(text),
                _ => GenAiChatMessage::user(text),
            };
            messages.push(msg);
        }

        Ok(GenAiChatRequest::new(messages))
    }

    fn env_key_candidates(&self) -> Vec<&'static str> {
        match self.id.as_str() {
            "google" => vec!["GOOGLE_API_KEY", "GEMINI_API_KEY"],
            "groq" => vec!["GROQ_API_KEY"],
            "openrouter" => vec!["OPENROUTER_API_KEY"],
            "openai" => vec!["OPENAI_API_KEY"],
            "anthropic" => vec!["ANTHROPIC_API_KEY"],
            "github_copilot" | "github-copilot" => vec!["GITHUB_COPILOT_TOKEN"],
            _ => vec![],
        }
    }

    fn resolve_api_key(&self) -> Option<String> {
        for key in self.env_key_candidates() {
            if let Ok(value) = std::env::var(key)
                && !value.trim().is_empty()
            {
                return Some(value);
            }
        }
        None
    }

    async fn fetch_google_models(&self, api_key: &str) -> Result<Vec<ModelInfo>, ProviderError> {
        #[derive(Debug, Deserialize)]
        struct GoogleModelsResponse {
            models: Option<Vec<GoogleModel>>,
        }

        #[derive(Debug, Deserialize)]
        struct GoogleModel {
            name: Option<String>,
            #[serde(default)]
            display_name: Option<String>,
            #[serde(default)]
            output_token_limit: Option<u32>,
            #[serde(default)]
            supported_generation_methods: Vec<String>,
        }

        let url = format!("https://generativelanguage.googleapis.com/v1beta/models?key={api_key}");
        let response = reqwest::Client::new().get(url).send().await?;
        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::HttpStatus {
                provider: self.id.clone(),
                status,
                body,
            });
        }

        let raw: GoogleModelsResponse = response.json().await?;
        let mut models = Vec::new();
        for model in raw.models.unwrap_or_default() {
            let Some(name) = model.name else {
                continue;
            };
            if !model.supported_generation_methods.iter().any(|method| {
                method.eq_ignore_ascii_case("generateContent")
                    || method.eq_ignore_ascii_case("streamGenerateContent")
            }) {
                continue;
            }
            let id = name.strip_prefix("models/").unwrap_or(&name).to_string();
            if id.trim().is_empty() {
                continue;
            }

            models.push(ModelInfo {
                id,
                display_name: model.display_name,
                context_window: None,
                max_output_tokens: model.output_token_limit,
                supports_tools: self.metadata.capabilities.tools,
                supports_vision: self.metadata.capabilities.vision,
                supports_audio: self.metadata.capabilities.audio_input,
            });
        }

        models.sort_by(|a, b| a.id.cmp(&b.id));
        models.dedup_by(|a, b| a.id == b.id);
        Ok(models)
    }

    async fn fetch_openai_models(
        &self,
        base_url: &str,
        api_key: &str,
    ) -> Result<Vec<ModelInfo>, ProviderError> {
        #[derive(Debug, Deserialize)]
        struct OpenAiModelsResponse {
            data: Vec<OpenAiModel>,
        }

        #[derive(Debug, Deserialize)]
        struct OpenAiModel {
            id: String,
        }

        let endpoint = format!("{}/v1/models", base_url.trim_end_matches('/'));
        let response = reqwest::Client::new().get(endpoint).bearer_auth(api_key).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::HttpStatus {
                provider: self.id.clone(),
                status,
                body,
            });
        }

        let payload: OpenAiModelsResponse = response.json().await?;
        let mut models = payload
            .data
            .into_iter()
            .map(|model| ModelInfo {
                id: model.id,
                display_name: None,
                context_window: None,
                max_output_tokens: None,
                supports_tools: self.metadata.capabilities.tools,
                supports_vision: self.metadata.capabilities.vision,
                supports_audio: self.metadata.capabilities.audio_input,
            })
            .collect::<Vec<_>>();

        models.sort_by(|a, b| a.id.cmp(&b.id));
        models.dedup_by(|a, b| a.id == b.id);
        Ok(models)
    }

    /// Well-known Google Gemini models (fallback when API key is missing or API call fails)
    fn google_fallback_models(meta: &ProviderMetadata) -> Vec<ModelInfo> {
        [
            "gemini-2.5-flash",
            "gemini-2.5-pro",
            "gemini-2.0-flash",
            "gemini-2.0-flash-lite",
            "gemini-1.5-flash",
            "gemini-1.5-pro",
            "gemini-3-pro-preview",
            "gemini-3-flash-preview",
        ]
        .iter()
        .map(|id| ModelInfo {
            id: id.to_string(),
            display_name: None,
            context_window: None,
            max_output_tokens: None,
            supports_tools: meta.capabilities.tools,
            supports_vision: meta.capabilities.vision,
            supports_audio: meta.capabilities.audio_input,
        })
        .collect()
    }

    /// Well-known Groq models (fallback)
    fn groq_fallback_models(meta: &ProviderMetadata) -> Vec<ModelInfo> {
        [
            "llama-3.3-70b-versatile",
            "llama-3.1-8b-instant",
            "llama-3.1-70b-versatile",
            "gemma2-9b-it",
            "mixtral-8x7b-32768",
            "qwen-qwq-32b",
            "deepseek-r1-distill-llama-70b",
        ]
        .iter()
        .map(|id| ModelInfo {
            id: id.to_string(),
            display_name: None,
            context_window: None,
            max_output_tokens: None,
            supports_tools: meta.capabilities.tools,
            supports_vision: meta.capabilities.vision,
            supports_audio: meta.capabilities.audio_input,
        })
        .collect()
    }

    /// Well-known OpenRouter models (fallback)
    fn openrouter_fallback_models(meta: &ProviderMetadata) -> Vec<ModelInfo> {
        [
            "openai/gpt-4o",
            "openai/gpt-4o-mini",
            "anthropic/claude-sonnet-4",
            "anthropic/claude-3.5-haiku",
            "google/gemini-2.5-flash",
            "google/gemini-2.5-pro",
            "deepseek/deepseek-chat",
            "meta-llama/llama-3.3-70b-instruct",
        ]
        .iter()
        .map(|id| ModelInfo {
            id: id.to_string(),
            display_name: None,
            context_window: None,
            max_output_tokens: None,
            supports_tools: meta.capabilities.tools,
            supports_vision: meta.capabilities.vision,
            supports_audio: meta.capabilities.audio_input,
        })
        .collect()
    }

    /// Fallback models for providers without live API access
    fn fallback_models_for(
        provider_id: &str,
        default_model: &str,
        meta: &ProviderMetadata,
    ) -> Vec<ModelInfo> {
        let models: Vec<&str> = match provider_id {
            "openai" => vec![
                "gpt-4o",
                "gpt-4o-mini",
                "gpt-5",
                "gpt-5-mini",
                "o3",
                "o3-mini",
                "o4-mini",
            ],
            "anthropic" => vec![
                "claude-sonnet-4-20250514",
                "claude-3-5-haiku-latest",
                "claude-opus-4-20250514",
                "claude-3-5-sonnet-20241022",
            ],
            "mistral" => vec![
                "mistral-small-latest",
                "mistral-medium-latest",
                "mistral-large-latest",
                "codestral-latest",
            ],
            "cohere" => vec!["command-r7b-12-2024", "command-r-plus", "command-r"],
            "deepseek" => vec!["deepseek-chat", "deepseek-reasoner"],
            "xai" => vec!["grok-3", "grok-3-mini", "grok-3-fast"],
            "together" => vec![
                "meta-llama/Llama-3.3-70B-Instruct-Turbo",
                "meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo",
                "deepseek-ai/DeepSeek-R1",
                "Qwen/Qwen2.5-72B-Instruct-Turbo",
            ],
            "ollama" => vec![
                "llama3",
                "llama3.1",
                "llama3.2",
                "mistral",
                "codellama",
                "gemma2",
                "phi3",
                "qwen2.5",
            ],
            "github_copilot" | "github-copilot" => vec![
                "gpt-4o",
                "gpt-4o-mini",
                "gpt-5",
                "claude-sonnet-4",
                "claude-3.5-sonnet",
                "gemini-2.5-pro",
                "o3-mini",
                "o4-mini",
            ],
            _ => vec![],
        };

        if models.is_empty() {
            return vec![ModelInfo {
                id: default_model.to_string(),
                display_name: Some(meta.name.clone()),
                context_window: None,
                max_output_tokens: None,
                supports_tools: meta.capabilities.tools,
                supports_vision: meta.capabilities.vision,
                supports_audio: meta.capabilities.audio_input,
            }];
        }

        models
            .iter()
            .map(|id| ModelInfo {
                id: id.to_string(),
                display_name: None,
                context_window: None,
                max_output_tokens: None,
                supports_tools: meta.capabilities.tools,
                supports_vision: meta.capabilities.vision,
                supports_audio: meta.capabilities.audio_input,
            })
            .collect()
    }
}

#[async_trait]
impl LlmProvider for GenAiProvider {
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
        let requested_model = if request.model.trim().is_empty() {
            self.model.clone()
        } else {
            request.model.clone()
        };
        let genai_request = self.to_genai_request(request)?;
        let response =
            self.client
                .exec_chat(&requested_model, genai_request, None)
                .await
                .map_err(|err| ProviderError::InvalidResponse {
                    provider: self.id.clone(),
                    detail: format!("genai chat failed: {err}"),
                })?;

        let content = response.first_text().unwrap_or("").to_string();

        Ok(ChatResponse {
            id: None,
            model: Some(requested_model.clone()),
            content,
            finish_reason: None,
            usage: None,
            raw: json!({ "provider": self.id, "model": requested_model }),
        })
    }

    async fn stream(&self, _request: ChatRequest) -> Result<ProviderStream, ProviderError> {
        Err(ProviderError::InvalidConfig {
            provider: self.id.clone(),
            detail: "genai streaming adapter is not yet enabled in dx-onboard".to_string(),
        })
    }

    async fn get_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        if self.id == "google" {
            if let Some(api_key) = self.resolve_api_key() {
                let models = self.fetch_google_models(&api_key).await?;
                if !models.is_empty() {
                    return Ok(models);
                }
            }
            // Fallback: well-known Google Gemini models
            return Ok(Self::google_fallback_models(&self.metadata));
        }

        if self.id == "groq" {
            if let Some(api_key) = self.resolve_api_key() {
                let models =
                    self.fetch_openai_models("https://api.groq.com/openai", &api_key).await?;
                if !models.is_empty() {
                    return Ok(models);
                }
            }
            return Ok(Self::groq_fallback_models(&self.metadata));
        }

        if self.id == "openrouter" {
            if let Some(api_key) = self.resolve_api_key() {
                let models =
                    self.fetch_openai_models("https://openrouter.ai/api", &api_key).await?;
                if !models.is_empty() {
                    return Ok(models);
                }
            }
            return Ok(Self::openrouter_fallback_models(&self.metadata));
        }

        // For other providers, return well-known fallback models by provider id
        Ok(Self::fallback_models_for(&self.id, &self.model, &self.metadata))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::types::{AuthRequirement, ChatMessage, MessageContent, ProviderCapabilities};

    #[test]
    fn converts_text_messages_to_genai_request() {
        let provider = GenAiProvider::new(
            "openai",
            "gpt-4o-mini",
            ProviderMetadata {
                id: "openai".to_string(),
                name: "OpenAI".to_string(),
                category: "major-cloud".to_string(),
                auth_requirement: AuthRequirement::BearerToken,
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
                docs_url: None,
                website: None,
            },
        );

        let request = ChatRequest {
            model: "gpt-4o-mini".to_string(),
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
            extra: None,
        };

        assert!(provider.to_genai_request(request).is_ok());
    }
}
