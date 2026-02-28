//! Google AI Studio (Gemini) backend using direct REST API

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{Backend, BackendType};

#[derive(Serialize)]
struct GenerateRequest {
    contents: Vec<Content>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    temperature: f32,
    #[serde(rename = "topP")]
    top_p: f32,
    #[serde(rename = "topK")]
    top_k: i32,
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: usize,
}

#[derive(Deserialize)]
struct GenerateResponse {
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

pub struct GoogleBackend {
    client: reqwest::Client,
    api_key: Option<String>,
    model: String,
}

impl GoogleBackend {
    pub fn new(api_key: Option<String>, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
        }
    }

    pub fn set_api_key(&mut self, api_key: String) {
        self.api_key = Some(api_key);
    }

    pub fn has_api_key(&self) -> bool {
        self.api_key.is_some()
    }
}

#[async_trait]
impl Backend for GoogleBackend {
    async fn initialize(&mut self) -> Result<()> {
        // Fast initialization - no API validation
        // API key will be checked on first generation attempt
        Ok(())
    }

    async fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String> {
        let api_key = self.api_key.as_ref().context("Google API key not configured")?;

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, api_key
        );

        let request = GenerateRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: prompt.to_string(),
                }],
            }],
            generation_config: GenerationConfig {
                temperature: 0.7,
                top_p: 0.9,
                top_k: 40,
                max_output_tokens: max_tokens,
            },
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Google AI")?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();

            // Provide more helpful error messages
            let error_msg = if status.as_u16() == 400 {
                format!(
                    "Invalid request (400). Check if model '{}' exists. Error: {}",
                    self.model, error_text
                )
            } else if status.as_u16() == 401 || status.as_u16() == 403 {
                format!(
                    "Authentication failed ({}). Check your API key. Error: {}",
                    status, error_text
                )
            } else if status.as_u16() == 404 {
                format!(
                    "Model '{}' not found (404). Try 'gemini-1.5-flash' or 'gemini-1.5-pro'. Error: {}",
                    self.model, error_text
                )
            } else {
                format!("Google AI API error {}: {}", status, error_text)
            };

            anyhow::bail!(error_msg);
        }

        let result: GenerateResponse =
            response.json().await.context("Failed to parse Google AI response")?;

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
        let api_key = self.api_key.as_ref().context("Google API key not configured")?;

        // Use streamGenerateContent endpoint for real streaming
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}",
            self.model, api_key
        );

        let request = GenerateRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: prompt.to_string(),
                }],
            }],
            generation_config: GenerationConfig {
                temperature: 0.7,
                top_p: 0.9,
                top_k: 40,
                max_output_tokens: max_tokens,
            },
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Google AI")?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();

            // Provide more helpful error messages
            let error_msg = if status.as_u16() == 400 {
                format!(
                    "Invalid request (400). Check if model '{}' exists. Error: {}",
                    self.model, error_text
                )
            } else if status.as_u16() == 401 || status.as_u16() == 403 {
                format!(
                    "Authentication failed ({}). Check your API key. Error: {}",
                    status, error_text
                )
            } else if status.as_u16() == 404 {
                format!(
                    "Model '{}' not found (404). Try 'gemini-1.5-flash' or 'gemini-1.5-pro'. Error: {}",
                    self.model, error_text
                )
            } else {
                format!("Google AI API error {}: {}", status, error_text)
            };

            anyhow::bail!(error_msg);
        }

        // Stream the response
        use futures::StreamExt;
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Failed to read stream chunk")?;
            let text = String::from_utf8_lossy(&chunk);

            // Parse JSON chunks (Google returns newline-delimited JSON)
            for line in text.lines() {
                if line.trim().is_empty() {
                    continue;
                }

                // Try to parse as JSON
                if let Ok(result) = serde_json::from_str::<GenerateResponse>(line) {
                    if let Some(text) = result
                        .candidates
                        .first()
                        .and_then(|c| c.content.parts.first())
                        .map(|p| p.text.clone())
                    {
                        // Send each word as it comes
                        for word in text.split_whitespace() {
                            if !buffer.is_empty() {
                                callback(" ".to_string());
                            }
                            callback(word.to_string());
                            buffer.push_str(word);
                            buffer.push(' ');
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn is_available(&self) -> bool {
        self.api_key.is_some()
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Google
    }
}
