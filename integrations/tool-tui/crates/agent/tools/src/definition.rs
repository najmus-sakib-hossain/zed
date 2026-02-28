//! Tool definition types and traits.

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Parameter type for tool inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterType {
    String,
    Integer,
    Boolean,
    Number,
    Array,
    Object,
}

/// A single parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub description: String,
    pub param_type: ParameterType,
    pub required: bool,
    pub default: Option<serde_json::Value>,
    pub enum_values: Option<Vec<String>>,
}

/// Tool definition (for LLM function calling)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
    /// Category (file, shell, browser, search, etc.)
    pub category: String,
    /// Whether this tool requires confirmation
    pub requires_confirmation: bool,
}

/// A tool invocation from the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    /// Optional structured data
    pub data: Option<serde_json::Value>,
}

impl ToolResult {
    pub fn success(tool_call_id: String, output: String) -> Self {
        Self {
            tool_call_id,
            success: true,
            output,
            error: None,
            data: None,
        }
    }

    pub fn error(tool_call_id: String, error: String) -> Self {
        Self {
            tool_call_id,
            success: false,
            output: String::new(),
            error: Some(error),
            data: None,
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
}

/// Trait for implementing tools
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool definition
    fn definition(&self) -> ToolDefinition;

    /// Execute the tool with given arguments
    async fn execute(&self, call: ToolCall) -> Result<ToolResult>;

    /// Check if this tool is available in the current environment
    fn is_available(&self) -> bool {
        true
    }
}
