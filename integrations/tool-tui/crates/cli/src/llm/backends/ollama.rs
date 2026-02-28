//! Ollama backend for API-based inference with remote AI support

use anyhow::{Context, Result};
use async_trait::async_trait;
use ollama_rs::{
    Ollama,
    generation::completion::{GenerationResponse, request::GenerationRequest},
};

use super::{Backend, BackendType};

pub struct OllamaBackend {
    client: Ollama,
    model: String,
    enable_remote: bool,
    remote_api_key: Option<String>,
    remote_model: Option<String>,
}

impl OllamaBackend {
    pub fn new(url: String, model: String) -> Self {
        Self::with_config(url, model, None, None, false, None, None)
    }

    pub fn with_models_dirs(
        url: String,
        model: String,
        models_dir: Option<String>,
        models_remote_dir: Option<String>,
    ) -> Self {
        Self::with_config(url, model, models_dir, models_remote_dir, false, None, None)
    }

    pub fn with_config(
        url: String,
        model: String,
        models_dir: Option<String>,
        models_remote_dir: Option<String>,
        enable_remote: bool,
        remote_api_key: Option<String>,
        remote_model: Option<String>,
    ) -> Self {
        // SAFETY: Setting environment variables for Ollama configuration
        // These are only used by Ollama and don't affect other parts of the system
        unsafe {
            if let Some(dir) = &models_dir {
                std::env::set_var("OLLAMA_MODELS", dir);
            }
            if let Some(dir) = &models_remote_dir {
                std::env::set_var("OLLAMA_MODELS_REMOTE", dir);
            }

            // Set remote config env vars
            if enable_remote {
                std::env::set_var("OLLAMA_REMOTE_ENABLED", "1");
                if let Some(key) = &remote_api_key {
                    std::env::set_var("OLLAMA_REMOTE_API_KEY", key);
                }
                if let Some(model) = &remote_model {
                    std::env::set_var("OLLAMA_REMOTE_MODEL", model);
                }
            }
        }

        let client = Ollama::new(url, 11434);
        Self {
            client,
            model,
            enable_remote,
            remote_api_key,
            remote_model,
        }
    }

    /// Get the model to use (remote or local)
    fn get_active_model(&self) -> String {
        if self.enable_remote {
            self.remote_model.clone().unwrap_or_else(|| self.model.clone())
        } else {
            self.model.clone()
        }
    }

    /// Check if using remote AI
    pub fn is_remote(&self) -> bool {
        self.enable_remote
    }
}

#[async_trait]
impl Backend for OllamaBackend {
    async fn initialize(&mut self) -> Result<()> {
        // Check if Ollama is running
        self.client.list_local_models().await.context("Failed to connect to Ollama")?;
        Ok(())
    }

    async fn generate(&self, prompt: &str, _max_tokens: usize) -> Result<String> {
        let model = self.get_active_model();
        let request = GenerationRequest::new(model, prompt.to_string());

        let response: GenerationResponse =
            self.client.generate(request).await.context("Ollama generation failed")?;

        Ok(response.response)
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        _max_tokens: usize,
        callback: Box<dyn Fn(String) + Send>,
    ) -> Result<()> {
        // ollama-rs 0.2 doesn't support true streaming, so we'll chunk the response
        let response = self.generate(prompt, _max_tokens).await?;

        // Split response into words and send them progressively for a streaming effect
        let words: Vec<&str> = response.split_whitespace().collect();
        for (i, word) in words.iter().enumerate() {
            if i > 0 {
                callback(" ".to_string());
            }
            callback(word.to_string());

            // Small delay to simulate streaming (optional, can be removed for max speed)
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        Ok(())
    }

    fn is_available(&self) -> bool {
        true // Will fail at initialize if not available
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Ollama
    }
}
