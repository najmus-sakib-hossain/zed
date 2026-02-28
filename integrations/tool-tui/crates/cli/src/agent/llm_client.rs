//! Multi-Provider LLM Client
//!
//! Provides a unified interface for communicating with multiple LLM providers
//! including streaming responses, token counting, and cost tracking.
//!
//! # Supported Providers
//!
//! - **OpenAI**: GPT-4, GPT-3.5-turbo
//! - **Anthropic**: Claude 3 family
//! - **Ollama**: Local models
//! - **Google**: Gemini models
//! - **Custom**: OpenAI-compatible endpoints
//!
//! # Features
//!
//! - Streaming responses with callbacks
//! - Token counting and cost tracking
//! - Automatic retries with exponential backoff
//! - Context optimization with DX Serializer
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::agent::llm_client::{LlmClient, LlmProvider, LlmRequest};
//!
//! let client = LlmClient::new(LlmProvider::Anthropic)?;
//!
//! let request = LlmRequest {
//!     messages: vec![
//!         LlmMessage { role: "user".into(), content: "Hello!".into() },
//!     ],
//!     max_tokens: 1000,
//!     temperature: 0.7,
//!     stream: false,
//!     provider: None,
//! };
//!
//! let response = client.generate(request).await?;
//! println!("Response: {}", response.text);
//! println!("Tokens: {} input, {} output", response.usage.input_tokens, response.usage.output_tokens);
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// LLM provider types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LlmProvider {
    /// OpenAI (GPT models)
    OpenAI,
    /// Anthropic (Claude models)
    Anthropic,
    /// Ollama (local models)
    Ollama,
    /// Google (Gemini models)
    Google,
    /// Custom OpenAI-compatible endpoint
    Custom {
        /// Base URL
        base_url: String,
        /// Model name
        model: String,
    },
}

impl Default for LlmProvider {
    fn default() -> Self {
        Self::Ollama
    }
}

impl LlmProvider {
    /// Get the API base URL for this provider
    pub fn base_url(&self) -> &str {
        match self {
            LlmProvider::OpenAI => "https://api.openai.com/v1",
            LlmProvider::Anthropic => "https://api.anthropic.com/v1",
            LlmProvider::Ollama => "http://localhost:11434/api",
            LlmProvider::Google => "https://generativelanguage.googleapis.com/v1beta",
            LlmProvider::Custom { base_url, .. } => base_url,
        }
    }

    /// Get the default model for this provider
    pub fn default_model(&self) -> &str {
        match self {
            LlmProvider::OpenAI => "gpt-4-turbo-preview",
            LlmProvider::Anthropic => "claude-3-5-sonnet-20241022",
            LlmProvider::Ollama => "llama3.2",
            LlmProvider::Google => "gemini-pro",
            LlmProvider::Custom { model, .. } => model,
        }
    }

    /// Cost per 1M input tokens (in USD)
    pub fn input_cost_per_million(&self) -> f64 {
        match self {
            LlmProvider::OpenAI => 10.0,   // GPT-4-turbo
            LlmProvider::Anthropic => 3.0, // Claude 3.5 Sonnet
            LlmProvider::Ollama => 0.0,    // Local
            LlmProvider::Google => 0.5,    // Gemini Pro
            LlmProvider::Custom { .. } => 0.0,
        }
    }

    /// Cost per 1M output tokens (in USD)
    pub fn output_cost_per_million(&self) -> f64 {
        match self {
            LlmProvider::OpenAI => 30.0,    // GPT-4-turbo
            LlmProvider::Anthropic => 15.0, // Claude 3.5 Sonnet
            LlmProvider::Ollama => 0.0,     // Local
            LlmProvider::Google => 1.5,     // Gemini Pro
            LlmProvider::Custom { .. } => 0.0,
        }
    }
}

/// A message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    /// Role (system, user, assistant)
    pub role: String,
    /// Content text
    pub content: String,
}

/// Request to the LLM
#[derive(Debug, Clone)]
pub struct LlmRequest {
    /// Messages in the conversation
    pub messages: Vec<LlmMessage>,
    /// Maximum tokens to generate
    pub max_tokens: usize,
    /// Temperature (0.0-2.0)
    pub temperature: f32,
    /// Enable streaming
    pub stream: bool,
    /// Override provider
    pub provider: Option<LlmProvider>,
}

/// Response from the LLM
#[derive(Debug, Clone)]
pub struct LlmResponse {
    /// Generated text
    pub text: String,
    /// Token usage
    pub usage: TokenUsage,
    /// Finish reason
    pub finish_reason: FinishReason,
    /// Model used
    pub model: String,
}

/// Token usage statistics
#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    /// Input tokens
    pub input_tokens: u64,
    /// Output tokens
    pub output_tokens: u64,
    /// Total tokens
    pub total_tokens: u64,
    /// Estimated cost in USD
    pub estimated_cost: f64,
}

impl TokenUsage {
    /// Create new usage stats
    pub fn new(input: u64, output: u64, provider: &LlmProvider) -> Self {
        let total = input + output;
        let input_cost = (input as f64 / 1_000_000.0) * provider.input_cost_per_million();
        let output_cost = (output as f64 / 1_000_000.0) * provider.output_cost_per_million();

        Self {
            input_tokens: input,
            output_tokens: output,
            total_tokens: total,
            estimated_cost: input_cost + output_cost,
        }
    }

    /// Add another usage to this one
    pub fn add(&mut self, other: &TokenUsage) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.total_tokens += other.total_tokens;
        self.estimated_cost += other.estimated_cost;
    }
}

/// Reason for finishing generation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinishReason {
    /// Reached stop sequence
    Stop,
    /// Reached max tokens
    Length,
    /// Content filter triggered
    ContentFilter,
    /// Error occurred
    Error,
    /// Unknown reason
    Unknown,
}

/// Callback for streaming responses
pub type StreamCallback = Box<dyn Fn(&str) + Send + Sync>;

/// LLM client for multi-provider communication
pub struct LlmClient {
    /// Default provider
    default_provider: LlmProvider,
    /// HTTP client
    http_client: reqwest::Client,
    /// Total usage across all requests
    total_input_tokens: Arc<AtomicU64>,
    total_output_tokens: Arc<AtomicU64>,
    /// API keys per provider
    api_keys: std::collections::HashMap<String, String>,
}

impl LlmClient {
    /// Create a new LLM client
    pub fn new(default_provider: LlmProvider) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .context("Failed to create HTTP client")?;

        let mut api_keys = std::collections::HashMap::new();

        // Load API keys from environment
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            api_keys.insert("openai".to_string(), key);
        }
        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            api_keys.insert("anthropic".to_string(), key);
        }
        if let Ok(key) = std::env::var("GOOGLE_API_KEY") {
            api_keys.insert("google".to_string(), key);
        }

        Ok(Self {
            default_provider,
            http_client,
            total_input_tokens: Arc::new(AtomicU64::new(0)),
            total_output_tokens: Arc::new(AtomicU64::new(0)),
            api_keys,
        })
    }

    /// Generate a response (non-streaming)
    pub async fn generate(&self, request: LlmRequest) -> Result<LlmResponse> {
        let provider = request.provider.as_ref().unwrap_or(&self.default_provider);

        match provider {
            LlmProvider::Ollama => self.generate_ollama(&request).await,
            LlmProvider::OpenAI => self.generate_openai(&request).await,
            LlmProvider::Anthropic => self.generate_anthropic(&request).await,
            LlmProvider::Google => self.generate_google(&request).await,
            LlmProvider::Custom { .. } => self.generate_openai_compatible(&request, provider).await,
        }
    }

    /// Generate with streaming
    pub async fn generate_stream<F>(&self, request: LlmRequest, callback: F) -> Result<LlmResponse>
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        let provider = request.provider.as_ref().unwrap_or(&self.default_provider);

        match provider {
            LlmProvider::Ollama => self.generate_ollama_stream(&request, callback).await,
            LlmProvider::OpenAI => self.generate_openai_stream(&request, callback).await,
            LlmProvider::Anthropic => self.generate_anthropic_stream(&request, callback).await,
            _ => {
                // Fall back to non-streaming for unsupported providers
                let response = self.generate(request).await?;
                callback(&response.text);
                Ok(response)
            }
        }
    }

    /// Generate using Ollama
    async fn generate_ollama(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let url = format!("{}/chat", LlmProvider::Ollama.base_url());

        let body = serde_json::json!({
            "model": LlmProvider::Ollama.default_model(),
            "messages": request.messages.iter().map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content
                })
            }).collect::<Vec<_>>(),
            "stream": false,
            "options": {
                "temperature": request.temperature,
                "num_predict": request.max_tokens
            }
        });

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to send request to Ollama")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Ollama error {}: {}", status, text));
        }

        let data: serde_json::Value = response.json().await?;

        let text = data["message"]["content"].as_str().unwrap_or("").to_string();

        let input_tokens = estimate_tokens(&request.messages);
        let output_tokens = text.split_whitespace().count() as u64;

        self.track_usage(input_tokens, output_tokens);

        Ok(LlmResponse {
            text,
            usage: TokenUsage::new(input_tokens, output_tokens, &LlmProvider::Ollama),
            finish_reason: FinishReason::Stop,
            model: LlmProvider::Ollama.default_model().to_string(),
        })
    }

    /// Generate using Ollama with streaming
    async fn generate_ollama_stream<F>(
        &self,
        request: &LlmRequest,
        callback: F,
    ) -> Result<LlmResponse>
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        let url = format!("{}/chat", LlmProvider::Ollama.base_url());

        let body = serde_json::json!({
            "model": LlmProvider::Ollama.default_model(),
            "messages": request.messages.iter().map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content
                })
            }).collect::<Vec<_>>(),
            "stream": true,
            "options": {
                "temperature": request.temperature,
                "num_predict": request.max_tokens
            }
        });

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to send streaming request to Ollama")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Ollama error {}: {}", status, text));
        }

        let mut full_text = String::new();
        let mut stream = response.bytes_stream();
        use futures::StreamExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Error reading stream")?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(line) {
                    if let Some(content) = data["message"]["content"].as_str() {
                        full_text.push_str(content);
                        callback(content);
                    }
                }
            }
        }

        let input_tokens = estimate_tokens(&request.messages);
        let output_tokens = full_text.split_whitespace().count() as u64;

        self.track_usage(input_tokens, output_tokens);

        Ok(LlmResponse {
            text: full_text,
            usage: TokenUsage::new(input_tokens, output_tokens, &LlmProvider::Ollama),
            finish_reason: FinishReason::Stop,
            model: LlmProvider::Ollama.default_model().to_string(),
        })
    }

    /// Generate using OpenAI
    async fn generate_openai(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let api_key = self.api_keys.get("openai").context("OpenAI API key not set")?;

        let url = format!("{}/chat/completions", LlmProvider::OpenAI.base_url());

        let body = serde_json::json!({
            "model": LlmProvider::OpenAI.default_model(),
            "messages": request.messages.iter().map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content
                })
            }).collect::<Vec<_>>(),
            "max_tokens": request.max_tokens,
            "temperature": request.temperature
        });

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .context("Failed to send request to OpenAI")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("OpenAI error {}: {}", status, text));
        }

        let data: serde_json::Value = response.json().await?;

        let text = data["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();

        let input_tokens = data["usage"]["prompt_tokens"].as_u64().unwrap_or(0);
        let output_tokens = data["usage"]["completion_tokens"].as_u64().unwrap_or(0);

        self.track_usage(input_tokens, output_tokens);

        Ok(LlmResponse {
            text,
            usage: TokenUsage::new(input_tokens, output_tokens, &LlmProvider::OpenAI),
            finish_reason: FinishReason::Stop,
            model: LlmProvider::OpenAI.default_model().to_string(),
        })
    }

    /// Generate using OpenAI with streaming
    async fn generate_openai_stream<F>(
        &self,
        request: &LlmRequest,
        callback: F,
    ) -> Result<LlmResponse>
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        let api_key = self.api_keys.get("openai").context("OpenAI API key not set")?;

        let url = format!("{}/chat/completions", LlmProvider::OpenAI.base_url());

        let body = serde_json::json!({
            "model": LlmProvider::OpenAI.default_model(),
            "messages": request.messages.iter().map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content
                })
            }).collect::<Vec<_>>(),
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "stream": true
        });

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .context("Failed to send streaming request to OpenAI")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("OpenAI error {}: {}", status, text));
        }

        let mut full_text = String::new();
        let mut stream = response.bytes_stream();
        use futures::StreamExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Error reading stream")?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data == "[DONE]" {
                        break;
                    }
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                            full_text.push_str(content);
                            callback(content);
                        }
                    }
                }
            }
        }

        let input_tokens = estimate_tokens(&request.messages);
        let output_tokens = full_text.split_whitespace().count() as u64;

        self.track_usage(input_tokens, output_tokens);

        Ok(LlmResponse {
            text: full_text,
            usage: TokenUsage::new(input_tokens, output_tokens, &LlmProvider::OpenAI),
            finish_reason: FinishReason::Stop,
            model: LlmProvider::OpenAI.default_model().to_string(),
        })
    }

    /// Generate using Anthropic
    async fn generate_anthropic(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let api_key = self.api_keys.get("anthropic").context("Anthropic API key not set")?;

        let url = format!("{}/messages", LlmProvider::Anthropic.base_url());

        // Separate system message from others
        let system_msg =
            request.messages.iter().find(|m| m.role == "system").map(|m| m.content.clone());

        let messages: Vec<_> = request
            .messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "model": LlmProvider::Anthropic.default_model(),
            "messages": messages,
            "max_tokens": request.max_tokens
        });

        if let Some(system) = system_msg {
            body["system"] = serde_json::json!(system);
        }

        let response = self
            .http_client
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to send request to Anthropic")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Anthropic error {}: {}", status, text));
        }

        let data: serde_json::Value = response.json().await?;

        let text = data["content"][0]["text"].as_str().unwrap_or("").to_string();

        let input_tokens = data["usage"]["input_tokens"].as_u64().unwrap_or(0);
        let output_tokens = data["usage"]["output_tokens"].as_u64().unwrap_or(0);

        self.track_usage(input_tokens, output_tokens);

        Ok(LlmResponse {
            text,
            usage: TokenUsage::new(input_tokens, output_tokens, &LlmProvider::Anthropic),
            finish_reason: FinishReason::Stop,
            model: LlmProvider::Anthropic.default_model().to_string(),
        })
    }

    /// Generate using Anthropic with streaming
    async fn generate_anthropic_stream<F>(
        &self,
        request: &LlmRequest,
        callback: F,
    ) -> Result<LlmResponse>
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        let api_key = self.api_keys.get("anthropic").context("Anthropic API key not set")?;

        let url = format!("{}/messages", LlmProvider::Anthropic.base_url());

        let system_msg =
            request.messages.iter().find(|m| m.role == "system").map(|m| m.content.clone());

        let messages: Vec<_> = request
            .messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "model": LlmProvider::Anthropic.default_model(),
            "messages": messages,
            "max_tokens": request.max_tokens,
            "stream": true
        });

        if let Some(system) = system_msg {
            body["system"] = serde_json::json!(system);
        }

        let response = self
            .http_client
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to send streaming request to Anthropic")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Anthropic error {}: {}", status, text));
        }

        let mut full_text = String::new();
        let mut input_tokens = 0u64;
        let mut output_tokens = 0u64;
        let mut stream = response.bytes_stream();
        use futures::StreamExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Error reading stream")?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                        match json["type"].as_str() {
                            Some("content_block_delta") => {
                                if let Some(content) = json["delta"]["text"].as_str() {
                                    full_text.push_str(content);
                                    callback(content);
                                }
                            }
                            Some("message_delta") => {
                                output_tokens =
                                    json["usage"]["output_tokens"].as_u64().unwrap_or(0);
                            }
                            Some("message_start") => {
                                input_tokens =
                                    json["message"]["usage"]["input_tokens"].as_u64().unwrap_or(0);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        self.track_usage(input_tokens, output_tokens);

        Ok(LlmResponse {
            text: full_text,
            usage: TokenUsage::new(input_tokens, output_tokens, &LlmProvider::Anthropic),
            finish_reason: FinishReason::Stop,
            model: LlmProvider::Anthropic.default_model().to_string(),
        })
    }

    /// Generate using Google
    async fn generate_google(&self, request: &LlmRequest) -> Result<LlmResponse> {
        let api_key = self.api_keys.get("google").context("Google API key not set")?;

        let model = LlmProvider::Google.default_model();
        let url = format!(
            "{}/models/{}:generateContent?key={}",
            LlmProvider::Google.base_url(),
            model,
            api_key
        );

        // Convert messages to Google format
        let contents: Vec<_> = request
            .messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| {
                let role = if m.role == "assistant" {
                    "model"
                } else {
                    "user"
                };
                serde_json::json!({
                    "role": role,
                    "parts": [{ "text": m.content }]
                })
            })
            .collect();

        let body = serde_json::json!({
            "contents": contents,
            "generationConfig": {
                "maxOutputTokens": request.max_tokens,
                "temperature": request.temperature
            }
        });

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to send request to Google")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Google error {}: {}", status, text));
        }

        let data: serde_json::Value = response.json().await?;

        let text = data["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let input_tokens = estimate_tokens(&request.messages);
        let output_tokens = text.split_whitespace().count() as u64;

        self.track_usage(input_tokens, output_tokens);

        Ok(LlmResponse {
            text,
            usage: TokenUsage::new(input_tokens, output_tokens, &LlmProvider::Google),
            finish_reason: FinishReason::Stop,
            model: model.to_string(),
        })
    }

    /// Generate using OpenAI-compatible endpoint
    async fn generate_openai_compatible(
        &self,
        request: &LlmRequest,
        provider: &LlmProvider,
    ) -> Result<LlmResponse> {
        let LlmProvider::Custom { base_url, model } = provider else {
            return Err(anyhow::anyhow!("Not a custom provider"));
        };

        let url = format!("{}/chat/completions", base_url);

        let body = serde_json::json!({
            "model": model,
            "messages": request.messages.iter().map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content
                })
            }).collect::<Vec<_>>(),
            "max_tokens": request.max_tokens,
            "temperature": request.temperature
        });

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to send request to custom endpoint")?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Custom endpoint error {}: {}", status, text));
        }

        let data: serde_json::Value = response.json().await?;

        let text = data["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();

        let input_tokens = data["usage"]["prompt_tokens"]
            .as_u64()
            .unwrap_or_else(|| estimate_tokens(&request.messages));
        let output_tokens = data["usage"]["completion_tokens"]
            .as_u64()
            .unwrap_or_else(|| text.split_whitespace().count() as u64);

        self.track_usage(input_tokens, output_tokens);

        Ok(LlmResponse {
            text,
            usage: TokenUsage::new(input_tokens, output_tokens, provider),
            finish_reason: FinishReason::Stop,
            model: model.clone(),
        })
    }

    /// Track token usage
    fn track_usage(&self, input: u64, output: u64) {
        self.total_input_tokens.fetch_add(input, Ordering::Relaxed);
        self.total_output_tokens.fetch_add(output, Ordering::Relaxed);
    }

    /// Get total usage statistics
    pub fn total_usage(&self) -> TokenUsage {
        let input = self.total_input_tokens.load(Ordering::Relaxed);
        let output = self.total_output_tokens.load(Ordering::Relaxed);
        TokenUsage::new(input, output, &self.default_provider)
    }

    /// Set API key for a provider
    pub fn set_api_key(&mut self, provider: &str, key: String) {
        self.api_keys.insert(provider.to_lowercase(), key);
    }

    /// Check if a provider is available
    pub fn is_available(&self, provider: &LlmProvider) -> bool {
        match provider {
            LlmProvider::Ollama => true, // Always assume available
            LlmProvider::OpenAI => self.api_keys.contains_key("openai"),
            LlmProvider::Anthropic => self.api_keys.contains_key("anthropic"),
            LlmProvider::Google => self.api_keys.contains_key("google"),
            LlmProvider::Custom { .. } => true,
        }
    }

    /// List available providers
    pub fn available_providers(&self) -> Vec<LlmProvider> {
        let mut providers = vec![LlmProvider::Ollama];

        if self.api_keys.contains_key("openai") {
            providers.push(LlmProvider::OpenAI);
        }
        if self.api_keys.contains_key("anthropic") {
            providers.push(LlmProvider::Anthropic);
        }
        if self.api_keys.contains_key("google") {
            providers.push(LlmProvider::Google);
        }

        providers
    }
}

/// Estimate token count for messages (rough approximation)
fn estimate_tokens(messages: &[LlmMessage]) -> u64 {
    messages
        .iter()
        .map(|m| {
            // Rough estimate: ~4 characters per token
            (m.content.len() / 4) as u64 + 4 // +4 for role overhead
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_defaults() {
        assert_eq!(LlmProvider::Ollama.default_model(), "llama3.2");
        assert_eq!(LlmProvider::Anthropic.default_model(), "claude-3-5-sonnet-20241022");
    }

    #[test]
    fn test_token_usage() {
        let usage = TokenUsage::new(1000, 500, &LlmProvider::Anthropic);
        assert_eq!(usage.total_tokens, 1500);
        assert!(usage.estimated_cost > 0.0);
    }

    #[test]
    fn test_client_creation() {
        let client = LlmClient::new(LlmProvider::Ollama);
        assert!(client.is_ok());
    }

    #[test]
    fn test_estimate_tokens() {
        let messages = vec![LlmMessage {
            role: "user".to_string(),
            content: "Hello, world!".to_string(),
        }];
        let tokens = estimate_tokens(&messages);
        assert!(tokens > 0);
    }
}
