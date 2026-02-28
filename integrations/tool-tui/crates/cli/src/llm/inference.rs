//! High-level inference engine

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{backends::Backend, config::LlmConfig, model_manager::ModelManager};

#[derive(Debug, Clone)]
pub struct InferenceRequest {
    pub prompt: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub stream: bool,
}

#[derive(Debug, Clone)]
pub struct InferenceResponse {
    pub text: String,
    pub tokens_generated: usize,
}

pub struct InferenceEngine {
    config: LlmConfig,
    backend: Arc<RwLock<Box<dyn Backend>>>,
    model_manager: ModelManager,
}

impl InferenceEngine {
    pub async fn new(config: LlmConfig) -> Result<Self> {
        let model_manager = ModelManager::new(config.cache_dir.clone())?;

        // Initialize backend based on config
        let backend = Self::create_backend(&config, &model_manager).await?;

        Ok(Self {
            config,
            backend: Arc::new(RwLock::new(backend)),
            model_manager,
        })
    }

    async fn create_backend(
        config: &LlmConfig,
        _model_manager: &ModelManager,
    ) -> Result<Box<dyn Backend>> {
        match config.default_backend.as_str() {
            "ollama" => {
                let mut backend = Box::new(super::backends::ollama::OllamaBackend::with_config(
                    config.ollama.url.clone(),
                    config.ollama.default_model.clone(),
                    config.ollama.models_dir.clone(),
                    config.ollama.models_remote_dir.clone(),
                    config.ollama.enable_remote,
                    config.ollama.remote_api_key.clone(),
                    config.ollama.remote_model.clone(),
                ));
                backend.initialize().await?;
                Ok(backend)
            }
            "khroma" => {
                let mut backend = Box::new(super::backends::khroma::KhromaBackend::new(
                    config.khroma.endpoint.clone(),
                    config.khroma.api_key.clone(),
                ));
                backend.initialize().await?;
                Ok(backend)
            }
            "google" => {
                let mut backend = Box::new(super::backends::google::GoogleBackend::new(
                    config.google.api_key.clone(),
                    config.google.default_model.clone(),
                ));
                backend.initialize().await?;
                Ok(backend)
            }
            "antigravity" => {
                let mut backend = Box::new(super::backends::antigravity::AntigravityBackend::new(
                    config.antigravity.oauth_token.clone(),
                    config.antigravity.default_model.clone(),
                ));
                backend.initialize().await?;
                Ok(backend)
            }
            _ => anyhow::bail!("Unknown backend: {}", config.default_backend),
        }
    }

    pub async fn generate(&self, request: InferenceRequest) -> Result<InferenceResponse> {
        let backend = self.backend.read().await;

        let text = backend
            .generate(&request.prompt, request.max_tokens)
            .await
            .context("Generation failed")?;

        Ok(InferenceResponse {
            tokens_generated: text.split_whitespace().count(),
            text,
        })
    }

    pub async fn generate_stream<F>(&self, request: InferenceRequest, callback: F) -> Result<()>
    where
        F: Fn(String) + Send + 'static,
    {
        let backend = self.backend.read().await;

        backend
            .generate_stream(&request.prompt, request.max_tokens, Box::new(callback))
            .await
            .context("Streaming generation failed")
    }

    pub fn list_models(&self) -> Result<Vec<String>> {
        self.model_manager.list_models()
    }
}
