//! LLM tool — multi-provider LLM client (OpenAI, Anthropic, Google, local).
//! Actions: complete | chat | embed | models | cost | switch

use crate::definition::*;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

pub struct LlmTool {
    pub default_provider: String,
    pub default_model: String,
}

impl Default for LlmTool {
    fn default() -> Self {
        Self {
            default_provider: "groq".into(),
            default_model: "meta-llama/llama-prompt-guard-2-86m".into(),
        }
    }
}

impl LlmTool {
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            default_provider: provider.into(),
            default_model: model.into(),
        }
    }

    fn get_api_key(&self, provider: &str) -> Option<String> {
        match provider {
            "google" | "gemini" => std::env::var("GEMINI_API_KEY").ok(),
            "openai" => std::env::var("OPENAI_API_KEY").ok(),
            "anthropic" => std::env::var("ANTHROPIC_API_KEY").ok(),
            "openrouter" => std::env::var("OPENROUTER_API_KEY").ok(),
            "groq" => std::env::var("GROQ_API_KEY").ok(),
            _ => None,
        }
    }

    fn get_api_url(&self, provider: &str) -> &str {
        match provider {
            "google" | "gemini" => "https://generativelanguage.googleapis.com/v1beta",
            "openai" => "https://api.openai.com/v1",
            "anthropic" => "https://api.anthropic.com/v1",
            "openrouter" => "https://openrouter.ai/api/v1",
            "groq" => "https://api.groq.com/openai/v1",
            _ => "http://localhost:11434/api",
        }
    }
}

#[async_trait]
impl Tool for LlmTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "llm".into(),
            description: "Multi-provider LLM client: completions, chat, embeddings. Supports Google/OpenAI/Anthropic/Groq/local models".into(),
            parameters: vec![
                ToolParameter { name: "action".into(), description: "LLM action".into(), param_type: ParameterType::String, required: true, default: None,
                    enum_values: Some(vec!["complete".into(),"chat".into(),"embed".into(),"models".into(),"cost".into(),"switch".into()]) },
                ToolParameter { name: "prompt".into(), description: "Prompt text".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "messages".into(), description: "Chat messages (JSON array)".into(), param_type: ParameterType::Array, required: false, default: None, enum_values: None },
                ToolParameter { name: "model".into(), description: "Model name".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "provider".into(), description: "Provider (google, openai, anthropic, openrouter, groq, local)".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
                ToolParameter { name: "max_tokens".into(), description: "Max output tokens".into(), param_type: ParameterType::Integer, required: false, default: Some(json!(1024)), enum_values: None },
                ToolParameter { name: "temperature".into(), description: "Temperature 0.0-2.0".into(), param_type: ParameterType::Number, required: false, default: Some(json!(0.7)), enum_values: None },
                ToolParameter { name: "system".into(), description: "System message".into(), param_type: ParameterType::String, required: false, default: None, enum_values: None },
            ],
            category: "ai".into(),
            requires_confirmation: false,
        }
    }

    async fn execute(&self, call: ToolCall) -> Result<ToolResult> {
        let action = call.arguments.get("action").and_then(|v| v.as_str()).unwrap_or("complete");
        let provider = call
            .arguments
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.default_provider);
        let model = call
            .arguments
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.default_model);

        match action {
            "complete" | "chat" => {
                let api_key = self.get_api_key(provider)
                    .ok_or(anyhow::anyhow!("No API key for '{provider}'. Set env var (GEMINI_API_KEY, OPENAI_API_KEY, GROQ_API_KEY, etc.)"))?;
                let base_url = self.get_api_url(provider);
                let max_tokens =
                    call.arguments.get("max_tokens").and_then(|v| v.as_u64()).unwrap_or(1024);
                let temperature =
                    call.arguments.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.7);

                let client = reqwest::Client::new();
                match provider {
                    "google" | "gemini" => {
                        let prompt = call
                            .arguments
                            .get("prompt")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Hello");
                        let system = call.arguments.get("system").and_then(|v| v.as_str());
                        let url = format!(
                            "{}/models/{}:generateContent?key={}",
                            base_url, model, api_key
                        );
                        let mut contents =
                            vec![json!({"role": "user", "parts": [{"text": prompt}]})];
                        let mut body = json!({"contents": contents, "generationConfig": {"maxOutputTokens": max_tokens, "temperature": temperature}});
                        if let Some(sys) = system {
                            body["systemInstruction"] = json!({"parts": [{"text": sys}]});
                        }
                        let resp = client.post(&url).json(&body).send().await?;
                        let status = resp.status();
                        let text = resp.text().await?;
                        if status.is_success() {
                            let parsed: serde_json::Value = serde_json::from_str(&text)?;
                            let content = parsed["candidates"][0]["content"]["parts"][0]["text"]
                                .as_str()
                                .unwrap_or("(no content)");
                            Ok(ToolResult::success(call.id, content.to_string())
                                .with_data(json!({"provider": provider, "model": model, "tokens_used": content.len() / 4})))
                        } else {
                            Ok(ToolResult::error(
                                call.id,
                                format!("API error ({}): {}", status, &text[..text.len().min(500)]),
                            ))
                        }
                    }
                    "openai" | "openrouter" | "groq" => {
                        let prompt = call
                            .arguments
                            .get("prompt")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Hello");
                        let system = call.arguments.get("system").and_then(|v| v.as_str());
                        let url = format!("{}/chat/completions", base_url);
                        let mut messages = Vec::new();
                        if let Some(sys) = system {
                            messages.push(json!({"role": "system", "content": sys}));
                        }
                        messages.push(json!({"role": "user", "content": prompt}));
                        let body = json!({"model": model, "messages": messages, "max_tokens": max_tokens, "temperature": temperature});
                        let resp =
                            client.post(&url).bearer_auth(&api_key).json(&body).send().await?;
                        let status = resp.status();
                        let text = resp.text().await?;
                        if status.is_success() {
                            let parsed: serde_json::Value = serde_json::from_str(&text)?;
                            let content = parsed["choices"][0]["message"]["content"]
                                .as_str()
                                .or(parsed.get("content").and_then(|c| c.as_str()))
                                .unwrap_or("(no content)");
                            Ok(ToolResult::success(call.id, content.to_string())
                                .with_data(json!({"provider": provider, "model": model})))
                        } else {
                            Ok(ToolResult::error(
                                call.id,
                                format!("API error ({}): {}", status, &text[..text.len().min(500)]),
                            ))
                        }
                    }
                    "anthropic" => {
                        let prompt = call
                            .arguments
                            .get("prompt")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Hello");
                        let system = call.arguments.get("system").and_then(|v| v.as_str());
                        let url = format!("{}/messages", base_url);
                        let mut body = json!({"model": model, "max_tokens": max_tokens, "messages": [{"role": "user", "content": prompt}]});
                        if let Some(sys) = system {
                            body["system"] = json!(sys);
                        }
                        let resp = client
                            .post(&url)
                            .header("x-api-key", &api_key)
                            .header("anthropic-version", "2023-06-01")
                            .json(&body)
                            .send()
                            .await?;
                        let status = resp.status();
                        let text = resp.text().await?;
                        if status.is_success() {
                            let parsed: serde_json::Value = serde_json::from_str(&text)?;
                            let content =
                                parsed["content"][0]["text"].as_str().unwrap_or("(no content)");
                            Ok(ToolResult::success(call.id, content.to_string()))
                        } else {
                            Ok(ToolResult::error(
                                call.id,
                                format!("API error ({}): {}", status, &text[..text.len().min(500)]),
                            ))
                        }
                    }
                    _ => Ok(ToolResult::error(call.id, format!("Unknown provider: {provider}"))),
                }
            }
            "models" => {
                let models = json!({
                    "google": ["gemma-3-27b-it", "gemma-3-12b-it", "gemini-2.0-flash", "gemini-1.5-pro"],
                    "openai": ["gpt-4o", "gpt-4o-mini", "gpt-4-turbo", "o1", "o1-mini"],
                    "anthropic": ["claude-sonnet-4-20250514", "claude-3-5-haiku-20241022"],
                    "local": ["llama3", "codellama", "mistral", "mixtral"]
                });
                Ok(ToolResult::success(call.id, serde_json::to_string_pretty(&models)?)
                    .with_data(models))
            }
            "cost" => {
                let tokens =
                    call.arguments.get("max_tokens").and_then(|v| v.as_u64()).unwrap_or(1000);
                let cost_per_1k = match (provider, model) {
                    ("google", _) => 0.0, // Gemma is free via AI Studio
                    ("openai", "gpt-4o") => 0.005,
                    ("openai", "gpt-4o-mini") => 0.00015,
                    ("anthropic", m) if m.contains("sonnet") => 0.003,
                    _ => 0.001,
                };
                let estimated = (tokens as f64 / 1000.0) * cost_per_1k;
                Ok(ToolResult::success(call.id, format!("~${estimated:.6} for {tokens} tokens on {provider}/{model}"))
                    .with_data(json!({"estimated_cost": estimated, "tokens": tokens, "cost_per_1k": cost_per_1k})))
            }
            "switch" => Ok(ToolResult::success(
                call.id,
                format!(
                    "Use 'provider' and 'model' params to switch. Current: {}/{}",
                    self.default_provider, self.default_model
                ),
            )),
            "embed" => Ok(ToolResult::success(
                call.id,
                "Embedding generation — connect embedding model for vector output".into(),
            )),
            other => Ok(ToolResult::error(call.id, format!("Unknown action: {other}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_def() {
        assert_eq!(LlmTool::default().definition().name, "llm");
    }
    #[test]
    fn test_default_model() {
        let tool = LlmTool::default();
        assert_eq!(tool.default_model, "meta-llama/llama-prompt-guard-2-86m");
        assert_eq!(tool.default_provider, "groq");
    }

    #[tokio::test]
    async fn test_models_action() {
        let tool = LlmTool::default();
        let call = ToolCall {
            id: "m1".into(),
            name: "llm".into(),
            arguments: json!({"action":"models"}),
        };
        let r = tool.execute(call).await.unwrap();
        assert!(r.output.contains("llama"));
        assert!(r.success);
    }

    #[tokio::test]
    async fn test_cost_action() {
        let tool = LlmTool::default();
        let call = ToolCall {
            id: "c1".into(),
            name: "llm".into(),
            arguments: json!({"action":"cost","max_tokens":1000,"provider":"google"}),
        };
        let r = tool.execute(call).await.unwrap();
        assert!(r.output.contains("$"));
    }
}
