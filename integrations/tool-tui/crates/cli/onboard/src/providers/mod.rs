pub mod generic;
pub mod native;
pub mod custom;
pub mod discovery;

pub trait LlmProvider {
    fn base_url(&self) -> &str;
    fn api_key(&self) -> &str;
    // async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;
    // async fn stream(&self, request: ChatRequest) -> impl Stream<Item = Result<ChatChunk>>;
    // async fn get_models(&self) -> Result<Vec<ModelInfo>>;
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("HTTP error: {0}")]
    HttpRequest(#[from] reqwest::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("Other error: {0}")]
    Other(String),
}
