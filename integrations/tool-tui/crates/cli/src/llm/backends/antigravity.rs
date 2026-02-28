//! Antigravity backend for Claude Opus 4.5, Gemini 3.0, etc. via Google OAuth
//! Antigravity models are accessed through the Gemini API with special model names

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{Backend, BackendType};

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GenerationConfig>,
}

#[derive(Serialize)]
struct Content {
    role: String,
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    temperature: f32,
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: usize,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: ResponseContent,
}

#[derive(Deserialize)]
struct ResponseContent {
    parts: Vec<ResponsePart>,
}

#[derive(Deserialize)]
struct ResponsePart {
    text: String,
}

pub struct AntigravityBackend {
    client: reqwest::Client,
    oauth_token: Option<String>,
    model: String,
}

impl AntigravityBackend {
    pub fn new(oauth_token: Option<String>, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            oauth_token,
            model,
        }
    }

    pub fn set_oauth_token(&mut self, token: String) {
        self.oauth_token = Some(token);
    }
}

#[async_trait]
impl Backend for AntigravityBackend {
    async fn initialize(&mut self) -> Result<()> {
        if self.oauth_token.is_none() {
            anyhow::bail!("OAuth token not set. Please authenticate with Google first.");
        }
        Ok(())
    }

    async fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String> {
        let token = self
            .oauth_token
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("OAuth token not set"))?;

        // Use Gemini API endpoint - Antigravity models are accessed through this
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            self.model
        );

        let request = GeminiRequest {
            contents: vec![Content {
                role: "user".to_string(),
                parts: vec![Part {
                    text: prompt.to_string(),
                }],
            }],
            generation_config: Some(GenerationConfig {
                temperature: 0.7,
                max_output_tokens: max_tokens,
            }),
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Antigravity (via Gemini API)")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Antigravity API error {}: {}", status, error_text);
        }

        let result: GeminiResponse =
            response.json().await.context("Failed to parse Antigravity response")?;

        let text = result
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .unwrap_or_default();

        Ok(text)
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        max_tokens: usize,
        callback: Box<dyn Fn(String) + Send>,
    ) -> Result<()> {
        // For now, simulate streaming by word-splitting the response
        let response = self.generate(prompt, max_tokens).await?;

        let words: Vec<&str> = response.split_whitespace().collect();
        for (i, word) in words.iter().enumerate() {
            if i > 0 {
                callback(" ".to_string());
            }
            callback(word.to_string());
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        Ok(())
    }

    fn is_available(&self) -> bool {
        self.oauth_token.is_some()
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Antigravity
    }
}
