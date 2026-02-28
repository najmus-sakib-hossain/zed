//! LLM configuration management

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Default backend to use
    pub default_backend: String,

    /// Model cache directory
    pub cache_dir: PathBuf,

    /// Hugging Face configuration
    pub huggingface: HuggingFaceConfig,

    /// Ollama configuration
    pub ollama: OllamaConfig,

    /// Khroma API configuration
    pub khroma: KhromaConfig,

    /// Google AI Studio configuration
    pub google: GoogleConfig,

    /// ElevenLabs TTS configuration
    pub elevenlabs: ElevenLabsConfig,

    /// Antigravity proxy configuration
    pub antigravity: AntigravityConfig,

    /// Inference settings
    pub inference: InferenceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuggingFaceConfig {
    /// HF token for private models
    pub token: Option<String>,

    /// Default model to use
    pub default_model: String,

    /// Model revision/branch
    pub revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    /// Ollama server URL
    pub url: String,

    /// Default model
    pub default_model: String,

    /// Local models directory (OLLAMA_MODELS)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models_dir: Option<String>,

    /// Remote models directory (OLLAMA_MODELS_REMOTE)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models_remote_dir: Option<String>,

    /// Enable remote AI (GPT-4, Claude, etc.)
    #[serde(default)]
    pub enable_remote: bool,

    /// Remote API key for authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_api_key: Option<String>,

    /// Remote model name (e.g., "gpt-4", "claude-3")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KhromaConfig {
    /// API endpoint
    pub endpoint: String,

    /// API key
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleConfig {
    /// Google AI Studio API key
    pub api_key: Option<String>,

    /// Default model
    pub default_model: String,

    // Commented out for minimal build - references ui::chat
    // /// Cached Google models (to avoid API calls on every startup)
    // #[serde(default)]
    // pub cached_models: Vec<crate::ui::chat::app_state::GoogleModel>,
    /// Cache timestamp (Unix timestamp)
    #[serde(default)]
    pub cache_timestamp: u64,

    /// Cache validity duration in seconds (default: 24 hours)
    #[serde(default = "default_cache_duration")]
    pub cache_duration: u64,
}

fn default_cache_duration() -> u64 {
    24 * 60 * 60 // 24 hours in seconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElevenLabsConfig {
    /// ElevenLabs API key
    pub api_key: Option<String>,

    /// Default voice ID
    #[serde(default = "default_elevenlabs_voice")]
    pub voice_id: String,

    /// Default model ID
    #[serde(default = "default_elevenlabs_model")]
    pub model_id: String,
}

fn default_elevenlabs_voice() -> String {
    "pMsXgVXv3BLzUgSXRplE".to_string() // Adam voice
}

fn default_elevenlabs_model() -> String {
    "eleven_multilingual_v2".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntigravityConfig {
    /// OAuth access token (from Google OAuth)
    pub oauth_token: Option<String>,

    /// Default model
    pub default_model: String,

    /// Available models
    #[serde(default = "default_antigravity_models")]
    pub available_models: Vec<String>,
}

fn default_antigravity_models() -> Vec<String> {
    vec![
        "claude-opus-4.5".to_string(),
        "claude-sonnet-4.5".to_string(),
        "claude-haiku-4.5".to_string(),
        "gemini-3.0-pro".to_string(),
        "gemini-2.5-flash".to_string(),
        "gpt-oss-120b".to_string(),
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    /// Maximum context length
    pub max_context: usize,

    /// Temperature for sampling
    pub temperature: f32,

    /// Top-p sampling
    pub top_p: f32,

    /// Top-k sampling
    pub top_k: usize,

    /// Repetition penalty
    pub repetition_penalty: f32,

    /// Maximum tokens to generate
    pub max_tokens: usize,
}

impl Default for LlmConfig {
    fn default() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("dx")
            .join("models");

        Self {
            default_backend: "google".to_string(),
            cache_dir,
            huggingface: HuggingFaceConfig::default(),
            ollama: OllamaConfig::default(),
            khroma: KhromaConfig::default(),
            google: GoogleConfig::default(),
            elevenlabs: ElevenLabsConfig::default(),
            antigravity: AntigravityConfig::default(),
            inference: InferenceConfig::default(),
        }
    }
}

impl Default for HuggingFaceConfig {
    fn default() -> Self {
        Self {
            token: std::env::var("HF_TOKEN").ok(),
            default_model: "google/gemma-2-2b-it".to_string(),
            revision: "main".to_string(),
        }
    }
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            url: std::env::var("OLLAMA_HOST")
                .unwrap_or_else(|_| "http://localhost:11434".to_string()),
            default_model: "qwen2.5-coder:3b".to_string(),
            models_dir: std::env::var("OLLAMA_MODELS").ok(),
            models_remote_dir: std::env::var("OLLAMA_MODELS_REMOTE").ok(),
            enable_remote: std::env::var("OLLAMA_REMOTE_ENABLED")
                .map(|v| v == "1" || v.to_lowercase() == "true")
                .unwrap_or(false),
            remote_api_key: std::env::var("OLLAMA_REMOTE_API_KEY").ok(),
            remote_model: std::env::var("OLLAMA_REMOTE_MODEL").ok(),
        }
    }
}

impl Default for KhromaConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:8080".to_string(),
            api_key: std::env::var("KHROMA_API_KEY").ok(),
        }
    }
}

impl Default for GoogleConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("GOOGLE_API_KEY").ok(),
            default_model: "gemini-1.5-flash".to_string(),
            // cached_models: Vec::new(),  // Commented out for minimal build
            cache_timestamp: 0,
            cache_duration: default_cache_duration(),
        }
    }
}

impl Default for ElevenLabsConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("ELEVENLABS_API_KEY").ok(),
            voice_id: default_elevenlabs_voice(),
            model_id: default_elevenlabs_model(),
        }
    }
}

impl Default for AntigravityConfig {
    fn default() -> Self {
        Self {
            oauth_token: None,
            default_model: "claude-opus-4.5".to_string(),
            available_models: default_antigravity_models(),
        }
    }
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            max_context: 8192,
            temperature: 0.7,
            top_p: 0.9,
            top_k: 50,
            repetition_penalty: 1.1,
            max_tokens: 2048,
        }
    }
}

impl LlmConfig {
    /// Load configuration from file
    pub fn load(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path).context("Failed to read LLM config file")?;

        toml::from_str(&content).context("Failed to parse LLM config")
    }

    /// Save configuration to file
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let content = toml::to_string_pretty(self).context("Failed to serialize LLM config")?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
        }

        std::fs::write(path, content)
            .with_context(|| format!("Failed to write LLM config file: {:?}", path))
    }

    /// Get default config path
    pub fn default_path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
            .unwrap_or_else(|| PathBuf::from(".config"));

        config_dir.join("dx").join("llm.toml")
    }
}
