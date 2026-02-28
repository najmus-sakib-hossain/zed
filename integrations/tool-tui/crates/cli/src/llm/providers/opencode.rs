// OpenCode provider for free LLM access
// Powered by OpenCode Zen: https://opencode.ai

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const MODELS_API: &str = "https://models.dev/api.json";
const OPENCODE_API: &str = "https://api.opencode.ai/v1";

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub context_limit: Option<u32>,
}

pub struct OpenCodeProvider {
    client: Client,
}

impl OpenCodeProvider {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client })
    }

    /// List all free models available through OpenCode
    pub async fn list_free_models(&self) -> Result<Vec<ModelInfo>> {
        let response = self
            .client
            .get(MODELS_API)
            .send()
            .await
            .context("Failed to fetch models catalog")?
            .json::<serde_json::Value>()
            .await
            .context("Failed to parse models catalog")?;

        let mut free_models = Vec::new();

        if let Some(providers) = response.as_object() {
            for (provider_id, provider_data) in providers {
                if let Some(models) = provider_data.get("models").and_then(|m| m.as_object()) {
                    for (model_id, model_data) in models {
                        // Check if model is free (cost.input === 0 && cost.output === 0)
                        if let Some(cost) = model_data.get("cost") {
                            let input_cost = cost.get("input").and_then(|v| v.as_f64());
                            let output_cost = cost.get("output").and_then(|v| v.as_f64());

                            if input_cost == Some(0.0) && output_cost == Some(0.0) {
                                let name = model_data
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or(model_id)
                                    .to_string();

                                let context_limit = model_data
                                    .get("limit")
                                    .and_then(|l| l.get("context"))
                                    .and_then(|v| v.as_u64())
                                    .map(|v| v as u32);

                                free_models.push(ModelInfo {
                                    id: format!("{}/{}", provider_id, model_id),
                                    name,
                                    provider: provider_id.clone(),
                                    context_limit,
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(free_models)
    }

    /// Send a chat completion request to OpenCode
    pub async fn chat(
        &self,
        model: &str,
        messages: Vec<Message>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> Result<String> {
        let request = ChatRequest {
            model: model.to_string(),
            messages,
            temperature,
            max_tokens,
        };

        let response = self
            .client
            .post(&format!("{}/chat/completions", OPENCODE_API))
            .header("Authorization", "Bearer public")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send chat request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("OpenCode API error ({}): {}", status, body);
        }

        let chat_response = response
            .json::<ChatResponse>()
            .await
            .context("Failed to parse chat response")?;

        chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .context("No response from model")
    }

    /// Simple helper for single-turn conversations
    pub async fn ask(&self, model: &str, prompt: &str) -> Result<String> {
        let messages = vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }];

        self.chat(model, messages, None, None).await
    }
}

impl Default for OpenCodeProvider {
    fn default() -> Self {
        Self::new().expect("Failed to create OpenCode provider")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_free_models() {
        let provider = OpenCodeProvider::new().unwrap();
        let models = provider.list_free_models().await.unwrap();
        
        assert!(!models.is_empty(), "Should find at least one free model");
        
        for model in &models {
            println!("Free model: {} ({})", model.name, model.id);
        }
    }

    #[tokio::test]
    #[ignore] // Only run when testing actual API
    async fn test_chat() {
        let provider = OpenCodeProvider::new().unwrap();
        let response = provider
            .ask("zai/glm-4.7-flash", "Say 'Hello from DX!' in one sentence")
            .await
            .unwrap();
        
        assert!(!response.is_empty());
        println!("Response: {}", response);
    }
}
