use crate::llm::error::ProviderError;
use crate::llm::types::{ChatChunk, ChatRequest, ChatResponse, ModelInfo, ProviderMetadata};
use async_trait::async_trait;
use futures_core::Stream;
use std::pin::Pin;

pub type ProviderStream = Pin<Box<dyn Stream<Item = Result<ChatChunk, ProviderError>> + Send>>;

#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn id(&self) -> &str;
    fn base_url(&self) -> &str;
    fn api_key(&self) -> &str;
    fn metadata(&self) -> &ProviderMetadata;

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, ProviderError>;
    async fn stream(&self, request: ChatRequest) -> Result<ProviderStream, ProviderError>;
    async fn get_models(&self) -> Result<Vec<ModelInfo>, ProviderError>;
}
