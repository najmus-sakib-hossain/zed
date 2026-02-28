use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("http transport error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("json serialization/deserialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("provider `{provider}` returned an unsuccessful status `{status}`: {body}")]
    HttpStatus {
        provider: String,
        status: u16,
        body: String,
    },
    #[error(transparent)]
    ModelNotFound(#[from] ModelNotFoundError),
    #[error(transparent)]
    RateLimit(#[from] RateLimitError),
    #[error("invalid response format from provider `{provider}`: {detail}")]
    InvalidResponse { provider: String, detail: String },
    #[error("invalid configuration for provider `{provider}`: {detail}")]
    InvalidConfig { provider: String, detail: String },
}

#[derive(Debug, Error)]
pub enum ModelNotFoundError {
    #[error("model not found: {model}")]
    NotFound { model: String },
}

#[derive(Debug, Error)]
pub enum RateLimitError {
    #[error("rate limit exceeded for provider `{provider}`")]
    Exceeded { provider: String },
}
