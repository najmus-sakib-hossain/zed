use crate::llm::error::ProviderError;
use crate::llm::generic::GenericProvider;
use crate::llm::models_dev::ModelsDevProvider;
use crate::llm::presets::{ProviderPreset, openai_compatible_provider_presets};
use crate::llm::provider::{LlmProvider, ProviderStream};
use crate::llm::types::{ChatRequest, ChatResponse, ModelInfo, ProviderMetadata};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleProvider {
    inner: GenericProvider,
}

impl OpenAiCompatibleProvider {
    pub fn new(
        id: impl Into<String>,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self {
            inner: GenericProvider::new(id, base_url, api_key),
        }
    }

    pub fn from_preset(preset: &ProviderPreset, api_key: String) -> Self {
        Self {
            inner: GenericProvider::new(preset.id, preset.base_url, api_key)
                .with_metadata(preset.metadata()),
        }
    }

    pub fn from_models_dev(provider: ModelsDevProvider, api_key: String) -> Option<Self> {
        let inner = provider.into_generic_provider(api_key)?;
        Some(Self { inner })
    }

    pub fn specific_provider(provider_id: &str, api_key: String) -> Option<Self> {
        let preset = openai_compatible_provider_presets()
            .into_iter()
            .find(|item| item.id == provider_id)?;
        Some(Self::from_preset(&preset, api_key))
    }

    pub fn with_metadata(mut self, metadata: ProviderMetadata) -> Self {
        self.inner = self.inner.with_metadata(metadata);
        self
    }

    pub fn with_custom_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner = self.inner.with_custom_header(key, value);
        self
    }

    pub fn into_inner(self) -> GenericProvider {
        self.inner
    }
}

#[async_trait]
impl LlmProvider for OpenAiCompatibleProvider {
    fn id(&self) -> &str {
        self.inner.id()
    }

    fn base_url(&self) -> &str {
        self.inner.base_url()
    }

    fn api_key(&self) -> &str {
        self.inner.api_key()
    }

    fn metadata(&self) -> &ProviderMetadata {
        self.inner.metadata()
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError> {
        self.inner.chat(request).await
    }

    async fn stream(&self, request: ChatRequest) -> Result<ProviderStream, ProviderError> {
        self.inner.stream(request).await
    }

    async fn get_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        self.inner.get_models().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::provider::LlmProvider;
    use crate::llm::types::{ChatMessage, MessageContent};
    use httpmock::Method::POST;
    use httpmock::MockServer;
    use std::collections::BTreeSet;

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
            extra: None,
        }
    }

    #[tokio::test]
    async fn supports_custom_base_url_and_api_key() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(POST)
                .path("/v1/chat/completions")
                .header("authorization", "Bearer token-1");
            then.status(200)
                .header("content-type", "application/json")
                .body(
                    r#"{"id":"1","model":"test","choices":[{"message":{"content":"ok"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}"#,
                );
        });

        let provider = OpenAiCompatibleProvider::new("test", server.base_url(), "token-1");
        let response = provider.chat(sample_request()).await.expect("chat response");
        mock.assert();
        assert_eq!(response.content, "ok");
    }

    #[test]
    fn supports_specific_provider_factory_batch() {
        let supported: BTreeSet<&str> = [
            "cerebras",
            "deepinfra",
            "cloudflare-workers-ai",
            "cloudflare-ai-gateway",
            "sap-ai-core",
            "vercel-ai-gateway",
            "huggingface",
            "together",
            "fireworks",
            "nebius",
            "deepseek",
            "puter",
        ]
        .into_iter()
        .collect();

        for provider_id in supported {
            let provider =
                OpenAiCompatibleProvider::specific_provider(provider_id, "token".to_string());
            assert!(provider.is_some(), "provider factory missing: {provider_id}");
        }
    }
}
