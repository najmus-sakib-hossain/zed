use super::chat::{ChatMessage, ChatRole};
use super::provider::AiProviderKind;
use super::settings::AiSettings;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// AI Client - makes real API calls to LLM providers.
///
/// Based on the API structures from Zed's provider crates
/// (anthropic, open_ai, google_ai, etc.)
pub struct AiClient;

impl AiClient {
    /// Send a chat completion request to the specified provider.
    pub fn send_message_blocking(
        provider: AiProviderKind,
        model: &str,
        api_key: &str,
        messages: &[ChatMessage],
        settings: &AiSettings,
    ) -> Result<String> {
        match provider {
            AiProviderKind::Anthropic => Self::send_anthropic(model, api_key, messages, settings),
            AiProviderKind::OpenAi => Self::send_openai(model, api_key, messages, settings),
            AiProviderKind::GoogleAi => Self::send_google(model, api_key, messages, settings),
            AiProviderKind::DeepSeek => Self::send_openai_compatible(
                "https://api.deepseek.com/v1/chat/completions",
                model,
                api_key,
                messages,
                settings,
            ),
            AiProviderKind::Mistral => Self::send_openai_compatible(
                "https://api.mistral.ai/v1/chat/completions",
                model,
                api_key,
                messages,
                settings,
            ),
            AiProviderKind::XAi => Self::send_openai_compatible(
                "https://api.x.ai/v1/chat/completions",
                model,
                api_key,
                messages,
                settings,
            ),
            AiProviderKind::OpenRouter => Self::send_openai_compatible(
                "https://openrouter.ai/api/v1/chat/completions",
                model,
                api_key,
                messages,
                settings,
            ),
            AiProviderKind::LmStudio => Self::send_openai_compatible(
                "http://localhost:1234/v1/chat/completions",
                model,
                api_key,
                messages,
                settings,
            ),
            AiProviderKind::Ollama => Self::send_ollama(model, messages, settings),
            _ => Err(anyhow!("Provider {} is not yet supported for chat", provider.display_name())),
        }
    }

    /// Anthropic Messages API (Claude)
    fn send_anthropic(
        model: &str,
        api_key: &str,
        messages: &[ChatMessage],
        settings: &AiSettings,
    ) -> Result<String> {
        let client = reqwest::blocking::Client::new();

        // Extract system message if present
        let system_msg =
            messages.iter().find(|m| m.role == ChatRole::System).map(|m| m.content.clone());

        let api_messages: Vec<AnthropicMessage> = messages
            .iter()
            .filter(|m| m.role != ChatRole::System)
            .map(|m| AnthropicMessage {
                role: match m.role {
                    ChatRole::User => "user".to_string(),
                    ChatRole::Assistant => "assistant".to_string(),
                    ChatRole::System => "user".to_string(),
                },
                content: m.content.clone(),
            })
            .collect();

        let mut body = serde_json::json!({
            "model": model,
            "max_tokens": settings.max_tokens,
            "messages": api_messages,
        });

        if let Some(sys) = system_msg {
            body["system"] = serde_json::Value::String(sys);
        }
        if settings.temperature != 0.7 {
            body["temperature"] = serde_json::json!(settings.temperature);
        }

        let resp = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            return Err(anyhow!("Anthropic API error ({}): {}", status, text));
        }

        let data: AnthropicResponse = resp.json()?;
        let content = data
            .content
            .into_iter()
            .filter(|c| c.content_type == "text")
            .map(|c| c.text.unwrap_or_default())
            .collect::<Vec<_>>()
            .join("");

        Ok(content)
    }

    /// OpenAI Chat Completions API
    fn send_openai(
        model: &str,
        api_key: &str,
        messages: &[ChatMessage],
        settings: &AiSettings,
    ) -> Result<String> {
        Self::send_openai_compatible(
            "https://api.openai.com/v1/chat/completions",
            model,
            api_key,
            messages,
            settings,
        )
    }

    /// OpenAI-compatible chat completions (works for OpenAI, DeepSeek,
    /// Mistral, xAI, etc.)
    fn send_openai_compatible(
        endpoint: &str,
        model: &str,
        api_key: &str,
        messages: &[ChatMessage],
        settings: &AiSettings,
    ) -> Result<String> {
        let client = reqwest::blocking::Client::new();

        let api_messages: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        ChatRole::User => "user",
                        ChatRole::Assistant => "assistant",
                        ChatRole::System => "system",
                    },
                    "content": m.content,
                })
            })
            .collect();

        let body = serde_json::json!({
            "model": model,
            "messages": api_messages,
            "max_tokens": settings.max_tokens,
            "temperature": settings.temperature,
            "top_p": settings.top_p,
            "stream": false,
        });

        let resp = client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            return Err(anyhow!("API error ({}): {}", status, text));
        }

        let data: OpenAiResponse = resp.json()?;
        let content =
            data.choices.first().and_then(|c| c.message.content.clone()).unwrap_or_default();

        Ok(content)
    }

    /// Google AI (Gemini) API
    fn send_google(
        model: &str,
        api_key: &str,
        messages: &[ChatMessage],
        settings: &AiSettings,
    ) -> Result<String> {
        let client = reqwest::blocking::Client::new();

        let contents: Vec<serde_json::Value> = messages
            .iter()
            .filter(|m| m.role != ChatRole::System)
            .map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        ChatRole::User => "user",
                        ChatRole::Assistant => "model",
                        ChatRole::System => "user",
                    },
                    "parts": [{"text": m.content}]
                })
            })
            .collect();

        let mut body = serde_json::json!({
            "contents": contents,
            "generationConfig": {
                "temperature": settings.temperature,
                "maxOutputTokens": settings.max_tokens,
                "topP": settings.top_p,
            }
        });

        // Add system instruction if present
        if let Some(sys) = messages.iter().find(|m| m.role == ChatRole::System) {
            body["systemInstruction"] = serde_json::json!({
                "parts": [{"text": sys.content}]
            });
        }

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, api_key
        );

        let resp = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            return Err(anyhow!("Google AI API error ({}): {}", status, text));
        }

        let data: GoogleResponse = resp.json()?;
        let content = data
            .candidates
            .and_then(|c| c.into_iter().next())
            .and_then(|c| c.content)
            .and_then(|c| c.parts)
            .and_then(|p| p.into_iter().next())
            .and_then(|p| p.text)
            .unwrap_or_default();

        Ok(content)
    }

    /// Ollama API (local)
    fn send_ollama(model: &str, messages: &[ChatMessage], settings: &AiSettings) -> Result<String> {
        let client = reqwest::blocking::Client::new();

        let api_messages: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| {
                serde_json::json!({
                    "role": match m.role {
                        ChatRole::User => "user",
                        ChatRole::Assistant => "assistant",
                        ChatRole::System => "system",
                    },
                    "content": m.content,
                })
            })
            .collect();

        let body = serde_json::json!({
            "model": model,
            "messages": api_messages,
            "stream": false,
            "options": {
                "temperature": settings.temperature,
                "num_predict": settings.max_tokens,
                "top_p": settings.top_p,
            }
        });

        let resp = client
            .post("http://localhost:11434/api/chat")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            return Err(anyhow!("Ollama API error ({}): {}", status, text));
        }

        let data: OllamaResponse = resp.json()?;
        Ok(data.message.map(|m| m.content).unwrap_or_default())
    }
}

// ── API Response Types ──────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContentBlock>,
}

#[derive(Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Deserialize)]
struct OpenAiMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct GoogleResponse {
    candidates: Option<Vec<GoogleCandidate>>,
}

#[derive(Deserialize)]
struct GoogleCandidate {
    content: Option<GoogleContent>,
}

#[derive(Deserialize)]
struct GoogleContent {
    parts: Option<Vec<GooglePart>>,
}

#[derive(Deserialize)]
struct GooglePart {
    text: Option<String>,
}

#[derive(Deserialize)]
struct OllamaResponse {
    message: Option<OllamaMessage>,
}

#[derive(Deserialize)]
struct OllamaMessage {
    content: String,
}
