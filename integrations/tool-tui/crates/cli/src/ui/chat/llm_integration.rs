//! LLM integration for chat UI

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::llm::{InferenceEngine, InferenceRequest, LlmConfig};
use crate::ui::chat::app_state::GoogleModel;

#[derive(Clone)]
pub struct ChatLlm {
    engine: Arc<RwLock<Option<InferenceEngine>>>,
    config: Arc<RwLock<LlmConfig>>,
}

impl ChatLlm {
    pub fn new() -> Self {
        let config =
            LlmConfig::load(&LlmConfig::default_path()).unwrap_or_else(|_| LlmConfig::default());

        Self {
            engine: Arc::new(RwLock::new(None)),
            config: Arc::new(RwLock::new(config)),
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        let config = self.config.read().await.clone();
        let engine = InferenceEngine::new(config).await?;
        *self.engine.write().await = Some(engine);
        Ok(())
    }

    pub async fn generate(&self, prompt: &str) -> Result<String> {
        let engine = self.engine.read().await;
        let engine =
            engine.as_ref().ok_or_else(|| anyhow::anyhow!("LLM engine not initialized"))?;

        let config = self.config.read().await;
        let request = InferenceRequest {
            prompt: prompt.to_string(),
            max_tokens: config.inference.max_tokens,
            temperature: config.inference.temperature,
            stream: false,
        };

        let response = engine.generate(request).await?;
        Ok(response.text)
    }

    pub async fn generate_stream<F>(&self, prompt: &str, callback: F) -> Result<()>
    where
        F: Fn(String) + Send + 'static,
    {
        let engine = self.engine.read().await;
        let engine =
            engine.as_ref().ok_or_else(|| anyhow::anyhow!("LLM engine not initialized"))?;

        let config = self.config.read().await;
        let request = InferenceRequest {
            prompt: prompt.to_string(),
            max_tokens: config.inference.max_tokens,
            temperature: config.inference.temperature,
            stream: true,
        };

        engine.generate_stream(request, callback).await
    }

    pub fn is_initialized(&self) -> bool {
        // Check synchronously without blocking
        matches!(self.engine.try_read(), Ok(guard) if guard.is_some())
    }

    pub fn get_model_name(&self) -> String {
        // Use try_read to avoid blocking
        if let Ok(config) = self.config.try_read() {
            match config.default_backend.as_str() {
                "ollama" => config.ollama.default_model.clone(),
                "huggingface" => config.huggingface.default_model.clone(),
                "khroma" => "Khroma".to_string(),
                "google" => {
                    // Convert Google model name to camel case for display
                    config
                        .google
                        .default_model
                        .split('-')
                        .map(|word| {
                            let mut chars = word.chars();
                            match chars.next() {
                                None => String::new(),
                                Some(first) => {
                                    first.to_uppercase().collect::<String>() + chars.as_str()
                                }
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("-")
                }
                "antigravity" => {
                    // Convert Antigravity model name to title case
                    config
                        .antigravity
                        .default_model
                        .split('-')
                        .map(|word| {
                            let mut chars = word.chars();
                            match chars.next() {
                                None => String::new(),
                                Some(first) => {
                                    first.to_uppercase().collect::<String>() + chars.as_str()
                                }
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                }
                _ => "Unknown".to_string(),
            }
        } else {
            "Unknown".to_string()
        }
    }

    pub fn get_google_api_key(&self) -> Option<String> {
        if let Ok(config) = self.config.try_read() {
            config.google.api_key.clone()
        } else {
            None
        }
    }

    pub async fn set_google_api_key(&self, api_key: String) -> Result<()> {
        let mut config_lock = self.config.write().await;
        config_lock.google.api_key = Some(api_key);
        config_lock.save(&LlmConfig::default_path())?;
        drop(config_lock);

        // Reinitialize engine with new API key
        self.reinitialize().await?;
        Ok(())
    }

    pub fn get_elevenlabs_api_key(&self) -> Option<String> {
        if let Ok(config) = self.config.try_read() {
            config.elevenlabs.api_key.clone()
        } else {
            None
        }
    }

    pub async fn set_elevenlabs_api_key(&self, api_key: String) -> Result<()> {
        let mut config_lock = self.config.write().await;
        config_lock.elevenlabs.api_key = Some(api_key);
        config_lock.save(&LlmConfig::default_path())?;
        Ok(())
    }

    pub fn get_backend(&self) -> String {
        if let Ok(config) = self.config.try_read() {
            config.default_backend.clone()
        } else {
            "ollama".to_string()
        }
    }

    pub async fn set_backend(&self, backend: String) -> Result<()> {
        let mut config_lock = self.config.write().await;
        config_lock.default_backend = backend;
        config_lock.save(&LlmConfig::default_path())?;
        drop(config_lock);

        // Reinitialize engine with new backend
        self.reinitialize().await?;
        Ok(())
    }

    pub async fn set_google_model(&self, model: String) -> Result<()> {
        let mut config_lock = self.config.write().await;
        config_lock.google.default_model = model;
        config_lock.save(&LlmConfig::default_path())?;
        drop(config_lock);

        // Reinitialize engine with new model
        self.reinitialize().await?;
        Ok(())
    }

    pub async fn set_antigravity_oauth_token(&self, token: String) -> Result<()> {
        let mut config_lock = self.config.write().await;
        config_lock.antigravity.oauth_token = Some(token);
        config_lock.save(&LlmConfig::default_path())?;
        drop(config_lock);

        // Reinitialize engine with new token
        self.reinitialize().await?;
        Ok(())
    }

    pub async fn set_antigravity_model(&self, model: String) -> Result<()> {
        let mut config_lock = self.config.write().await;
        config_lock.antigravity.default_model = model;
        config_lock.save(&LlmConfig::default_path())?;
        drop(config_lock);

        // Reinitialize engine with new model
        self.reinitialize().await?;
        Ok(())
    }

    async fn reinitialize(&self) -> Result<()> {
        let config = self.config.read().await.clone();
        let engine = InferenceEngine::new(config).await?;
        *self.engine.write().await = Some(engine);
        Ok(())
    }

    pub fn get_cached_google_models(&self) -> Vec<GoogleModel> {
        if let Ok(config) = self.config.try_read() {
            config.google.cached_models.clone()
        } else {
            Vec::new()
        }
    }

    pub async fn fetch_google_models(&self) -> Result<Vec<GoogleModel>> {
        let config = self.config.write().await;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if !config.google.cached_models.is_empty()
            && (now - config.google.cache_timestamp) < config.google.cache_duration
        {
            // Return cached models
            return Ok(config.google.cached_models.clone());
        }

        // Cache is invalid or empty, fetch from API
        let api_key = config
            .google
            .api_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Google API key not configured"))?
            .clone();
        drop(config);

        let client = reqwest::Client::new();
        let url =
            format!("https://generativelanguage.googleapis.com/v1beta/models?key={}", api_key);

        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to fetch models: {} - {}", status, error_text);
        }

        #[derive(serde::Deserialize)]
        struct ModelsResponse {
            models: Vec<Model>,
        }

        #[derive(serde::Deserialize)]
        struct Model {
            name: String,
        }

        let models_response: ModelsResponse = response.json().await?;

        // Extract model names and filter for generateContent-capable models
        let models: Vec<GoogleModel> = models_response
            .models
            .into_iter()
            .filter_map(|m| {
                // Extract just the model ID from "models/gemini-pro" format
                m.name.strip_prefix("models/").map(|s| s.to_string())
            })
            .filter(|name| {
                // Include models that support generateContent (gemini and gemma)
                name.contains("gemini") || name.contains("gemma")
            })
            .map(|api_name| {
                // Convert to camel case for display
                let display_name = api_name
                    .split('-')
                    .map(|word| {
                        let mut chars = word.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(first) => {
                                first.to_uppercase().collect::<String>() + chars.as_str()
                            }
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("-");

                GoogleModel {
                    display_name,
                    api_name,
                }
            })
            .collect();

        // Cache the models
        let mut config = self.config.write().await;
        config.google.cached_models = models.clone();
        config.google.cache_timestamp = now;
        let _ = config.save(&LlmConfig::default_path());

        Ok(models)
    }
}

impl Default for ChatLlm {
    fn default() -> Self {
        Self::new()
    }
}
