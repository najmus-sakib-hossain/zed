use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuthRequirement {
    None,
    #[default]
    BearerToken,
    HeaderApiKey,
    OAuth,
    AwsSigV4,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct RateLimitInfo {
    pub requests_per_minute: Option<u32>,
    pub requests_per_day: Option<u32>,
    pub tokens_per_minute: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct ProviderCapabilities {
    pub chat: bool,
    pub streaming: bool,
    pub tools: bool,
    pub vision: bool,
    pub audio_input: bool,
    pub audio_output: bool,
    pub model_listing: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct ProviderMetadata {
    pub id: String,
    pub name: String,
    pub category: String,
    pub auth_requirement: AuthRequirement,
    pub capabilities: ProviderCapabilities,
    pub rate_limits: Option<RateLimitInfo>,
    pub docs_url: Option<String>,
    pub website: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: Option<String>,
    pub context_window: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub supports_audio: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ImageUrl {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct InputAudioData {
    pub data: String,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
    InputAudio { input_audio: InputAudioData },
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ChatMessage {
    pub role: String,
    pub content: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct ToolFunction {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<Value>,
    #[serde(default)]
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct UsageInfo {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ChatResponse {
    pub id: Option<String>,
    pub model: Option<String>,
    pub content: String,
    pub finish_reason: Option<String>,
    pub usage: Option<UsageInfo>,
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ChatChunk {
    pub id: Option<String>,
    pub model: Option<String>,
    pub delta: String,
    pub finish_reason: Option<String>,
    pub raw: Value,
}
