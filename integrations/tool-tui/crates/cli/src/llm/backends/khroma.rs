//! Khroma API backend

use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{Backend, BackendType};

#[derive(Serialize)]
struct KhromaRequest {
    prompt: String,
    max_tokens: usize,
    temperature: f32,
}

#[derive(Deserialize)]
struct KhromaResponse {
    text: String,
}

pub struct KhromaBackend {
    client: Client,
    endpoint: String,
    api_key: Option<String>,
}

impl KhromaBackend {
    pub fn new(endpoint: String, api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            endpoint,
            api_key,
        }
    }
}

#[async_trait]
impl Backend for KhromaBackend {
    async fn initialize(&mut self) -> Result<()> {
        // Test connection
        let response = self.client.get(&self.endpoint).send().await?;
        if !response.status().is_success() {
            anyhow::bail!("Khroma API not available");
        }
        Ok(())
    }

    async fn generate(&self, prompt: &str, max_tokens: usize) -> Result<String> {
        let request = KhromaRequest {
            prompt: prompt.to_string(),
            max_tokens,
            temperature: 0.7,
        };

        let mut req = self.client.post(format!("{}/generate", self.endpoint)).json(&request);

        if let Some(key) = &self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let response: KhromaResponse =
            req.send().await?.json().await.context("Failed to parse Khroma response")?;

        Ok(response.text)
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        max_tokens: usize,
        callback: Box<dyn Fn(String) + Send>,
    ) -> Result<()> {
        // For now, use non-streaming
        let text = self.generate(prompt, max_tokens).await?;
        callback(text);
        Ok(())
    }

    fn is_available(&self) -> bool {
        true
    }

    fn backend_type(&self) -> BackendType {
        BackendType::Khroma
    }
}
