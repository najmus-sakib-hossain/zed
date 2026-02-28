//! Core model types for LLM interactions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Chat message role
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// A chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Tool call in assistant messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

/// Function call details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

/// Function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// Chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(default)]
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,
}

/// Chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
    pub created: DateTime<Utc>,
    pub provider: String,
    pub latency_ms: u64,
}

/// Response choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

/// Token usage information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub input_cost_per_1k: f64,
    pub output_cost_per_1k: f64,
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub supports_vision: bool,
}

impl ModelInfo {
    /// Calculate cost for given token usage
    pub fn calculate_cost(&self, usage: &Usage) -> f64 {
        let input_cost = (usage.prompt_tokens as f64 / 1000.0) * self.input_cost_per_1k;
        let output_cost = (usage.completion_tokens as f64 / 1000.0) * self.output_cost_per_1k;
        input_cost + output_cost
    }
}

/// Well-known models with pricing (as of 2026)
pub fn known_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "gpt-4o".into(),
            name: "GPT-4o".into(),
            provider: "openai".into(),
            context_window: 128_000,
            max_output_tokens: 16_384,
            input_cost_per_1k: 0.0025,
            output_cost_per_1k: 0.01,
            supports_streaming: true,
            supports_tools: true,
            supports_vision: true,
        },
        ModelInfo {
            id: "gpt-4o-mini".into(),
            name: "GPT-4o Mini".into(),
            provider: "openai".into(),
            context_window: 128_000,
            max_output_tokens: 16_384,
            input_cost_per_1k: 0.00015,
            output_cost_per_1k: 0.0006,
            supports_streaming: true,
            supports_tools: true,
            supports_vision: true,
        },
        ModelInfo {
            id: "claude-sonnet-4-20250514".into(),
            name: "Claude Sonnet 4".into(),
            provider: "anthropic".into(),
            context_window: 200_000,
            max_output_tokens: 8_192,
            input_cost_per_1k: 0.003,
            output_cost_per_1k: 0.015,
            supports_streaming: true,
            supports_tools: true,
            supports_vision: true,
        },
        ModelInfo {
            id: "claude-opus-4-20250514".into(),
            name: "Claude Opus 4".into(),
            provider: "anthropic".into(),
            context_window: 200_000,
            max_output_tokens: 8_192,
            input_cost_per_1k: 0.015,
            output_cost_per_1k: 0.075,
            supports_streaming: true,
            supports_tools: true,
            supports_vision: true,
        },
        ModelInfo {
            id: "claude-haiku-3.5-20241022".into(),
            name: "Claude Haiku 3.5".into(),
            provider: "anthropic".into(),
            context_window: 200_000,
            max_output_tokens: 8_192,
            input_cost_per_1k: 0.0008,
            output_cost_per_1k: 0.004,
            supports_streaming: true,
            supports_tools: true,
            supports_vision: true,
        },
        ModelInfo {
            id: "gemini-2.0-flash".into(),
            name: "Gemini 2.0 Flash".into(),
            provider: "google".into(),
            context_window: 1_000_000,
            max_output_tokens: 8_192,
            input_cost_per_1k: 0.00015,
            output_cost_per_1k: 0.0006,
            supports_streaming: true,
            supports_tools: true,
            supports_vision: true,
        },
        ModelInfo {
            id: "gemini-2.0-pro".into(),
            name: "Gemini 2.0 Pro".into(),
            provider: "google".into(),
            context_window: 2_000_000,
            max_output_tokens: 8_192,
            input_cost_per_1k: 0.00125,
            output_cost_per_1k: 0.005,
            supports_streaming: true,
            supports_tools: true,
            supports_vision: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_calculation() {
        let model = ModelInfo {
            id: "test".into(),
            name: "Test".into(),
            provider: "test".into(),
            context_window: 128_000,
            max_output_tokens: 4_096,
            input_cost_per_1k: 0.01,
            output_cost_per_1k: 0.03,
            supports_streaming: true,
            supports_tools: true,
            supports_vision: false,
        };

        let usage = Usage {
            prompt_tokens: 1000,
            completion_tokens: 500,
            total_tokens: 1500,
        };

        let cost = model.calculate_cost(&usage);
        assert!((cost - 0.025).abs() < 0.0001);
    }

    #[test]
    fn test_known_models() {
        let models = known_models();
        assert!(models.len() >= 5);
        assert!(models.iter().any(|m| m.provider == "openai"));
        assert!(models.iter().any(|m| m.provider == "anthropic"));
        assert!(models.iter().any(|m| m.provider == "google"));
    }

    #[test]
    fn test_chat_message_serialization() {
        let msg = ChatMessage {
            role: Role::User,
            content: "Hello".into(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello\""));
    }
}
