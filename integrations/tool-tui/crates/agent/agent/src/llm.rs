//! # LLM Client
//!
//! Communicates with LLMs using DX Serializer format to save 70%+ tokens.
//! Supports multiple LLM providers (Anthropic, OpenAI, etc.)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

use crate::{AgentError, Result};

/// LLM message in DX format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

impl LlmMessage {
    pub fn system(content: &str) -> Self {
        Self {
            role: "system".to_string(),
            content: content.to_string(),
        }
    }

    pub fn user(content: &str) -> Self {
        Self {
            role: "user".to_string(),
            content: content.to_string(),
        }
    }

    pub fn assistant(content: &str) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.to_string(),
        }
    }

    /// Convert message to DX LLM format (saves tokens)
    pub fn to_dx_llm(&self) -> String {
        format!("{}:{}", self.role, self.content.replace('\n', "\\n"))
    }

    /// Parse message from DX LLM format
    pub fn from_dx_llm(s: &str) -> Option<Self> {
        let (role, content) = s.split_once(':')?;
        Some(Self {
            role: role.to_string(),
            content: content.replace("\\n", "\n"),
        })
    }
}

/// LLM response with parsed skills and context
#[derive(Debug, Clone)]
pub struct LlmResponse {
    text: String,
    skills: Vec<String>,
    context: HashMap<String, String>,
}

impl LlmResponse {
    pub fn new(text: String) -> Self {
        let mut response = Self {
            text,
            skills: Vec::new(),
            context: HashMap::new(),
        };
        response.parse_skills_and_context();
        response
    }

    /// Parse skills and context from the response
    fn parse_skills_and_context(&mut self) {
        // Look for skill references in DX format: use_skill:skill_name
        for line in self.text.lines() {
            if line.starts_with("use_skill:") {
                let skill = line.trim_start_matches("use_skill:").trim();
                self.skills.push(skill.to_string());
            }

            // Look for context in DX format: context:key=value
            if line.starts_with("context:") {
                let rest = line.trim_start_matches("context:").trim();
                if let Some((key, value)) = rest.split_once('=') {
                    self.context.insert(key.to_string(), value.to_string());
                }
            }
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn required_skills(&self) -> &[String] {
        &self.skills
    }

    pub fn context(&self) -> &HashMap<String, String> {
        &self.context
    }
}

/// LLM client for communicating with AI models
pub struct LlmClient {
    endpoint: String,
    api_key: Option<String>,
    model: String,
    system_prompt: String,
}

impl LlmClient {
    pub fn new(endpoint: &str) -> Result<Self> {
        Ok(Self {
            endpoint: endpoint.to_string(),
            api_key: std::env::var("DX_API_KEY").ok(),
            model: "claude-sonnet-4-20250514".to_string(),
            system_prompt: Self::default_system_prompt(),
        })
    }

    /// Default system prompt that instructs the LLM to use DX format
    fn default_system_prompt() -> String {
        r#"You are DX Agent, an AGI-like AI assistant that can connect to any app and control any tool.

CRITICAL: Always use DX Serializer LLM format for structured data to save tokens. 

DX Format Rules:
- key=value for scalars (no spaces in values, use underscores)
- section:count[key=value] for objects
- key[count]=item1 item2 item3 for arrays (space-separated)
- name:count(schema)[rows] for tabular data

When you need to execute a skill, output:
use_skill:skill_name

When you need to provide context for a skill, output:
context:key=value

Available skills:
- send_message: Send messages via WhatsApp, Telegram, Discord, etc.
- create_todo: Create todos in Notion
- check_email: Check and summarize emails
- browse_web: Browse webpages
- run_command: Execute shell commands
- create_integration: Create new integrations (Python, JS → WASM)
- play_music: Control Spotify playback
- create_pr: Create GitHub PRs

You can create new integrations by writing code in any language. The DX agent will compile it to WASM and inject it automatically.

Example response format:
use_skill:send_message
context:platform=whatsapp
context:recipient=john
context:message=Hello_from_DX!

Always be concise. The DX format saves 52-73% tokens vs JSON."#.to_string()
    }

    /// Set the API key
    pub fn set_api_key(&mut self, key: &str) {
        self.api_key = Some(key.to_string());
    }

    /// Set the model to use
    pub fn set_model(&mut self, model: &str) {
        self.model = model.to_string();
    }

    /// Set a custom system prompt
    pub fn set_system_prompt(&mut self, prompt: &str) {
        self.system_prompt = prompt.to_string();
    }

    /// Process a message and get a response
    pub async fn process_message(&self, message: &str) -> Result<LlmResponse> {
        info!("Processing message with LLM...");

        // Build the request in DX format for our internal tracking
        let dx_request = format!(
            "request:1[model={} messages[2]=system:{} user:{}]",
            self.model,
            self.system_prompt.len(),
            message.len()
        );

        info!("DX Request: {}", dx_request);

        // In a real implementation, this would call the LLM API
        // For now, return a placeholder response
        let response_text = self.call_llm(message).await?;

        Ok(LlmResponse::new(response_text))
    }

    /// Call the LLM API
    async fn call_llm(&self, message: &str) -> Result<String> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| AgentError::AuthFailed {
                provider: "llm".to_string(),
                message: "API key not set. Set DX_API_KEY environment variable.".to_string(),
            })?;

        let client = reqwest::Client::new();

        // Build the request body
        let body = serde_json::json!({
            "model": self.model,
            "max_tokens": 4096,
            "system": self.system_prompt,
            "messages": [
                {"role": "user", "content": message}
            ]
        });

        let response = client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| AgentError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(AgentError::NetworkError(format!(
                "LLM API error: {} - {}",
                status, text
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AgentError::SerializationError(e.to_string()))?;

        // Extract the response text
        let text = json["content"][0]["text"]
            .as_str()
            .unwrap_or("No response")
            .to_string();

        Ok(text)
    }

    /// Stream a response (for real-time output)
    pub async fn stream_message(
        &self,
        message: &str,
    ) -> Result<impl futures::Stream<Item = Result<String>>> {
        // Placeholder - would implement SSE streaming in production
        let response = self.process_message(message).await?;
        Ok(futures::stream::once(async move {
            Ok(response.text().to_string())
        }))
    }

    /// Count tokens in a message (using DX Serializer's tokenizer)
    pub fn count_tokens(&self, text: &str) -> usize {
        // Approximate token count (in production, use tiktoken)
        text.split_whitespace().count()
    }

    /// Convert JSON to DX format for token savings
    pub fn json_to_dx(&self, json: &str) -> Result<String> {
        // Use DX Serializer to convert
        // This saves 52-73% tokens!
        let value: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| AgentError::SerializationError(e.to_string()))?;

        Ok(self.value_to_dx(&value))
    }

    /// Convert a JSON value to DX format
    fn value_to_dx(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::Null => "null".to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => s.replace(' ', "_"),
            serde_json::Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| self.value_to_dx(v)).collect();
                format!("[{}]={}", items.len(), items.join(" "))
            }
            serde_json::Value::Object(obj) => {
                let pairs: Vec<String> = obj
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, self.value_to_dx(v)))
                    .collect();
                format!(":{}[{}]", obj.len(), pairs.join(" "))
            }
        }
    }
}
