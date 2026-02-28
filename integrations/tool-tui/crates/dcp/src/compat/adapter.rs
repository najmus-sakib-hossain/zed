//! MCP to DCP adapter for backward compatibility.
//!
//! Translates JSON-RPC 2.0 MCP messages to DCP binary format and back.

use std::collections::HashMap;

use serde_json::Value;

use crate::dispatch::{BinaryTrieRouter, ToolResult};
use crate::DCPError;

use super::json_rpc::{
    JsonRpcError, JsonRpcParseError, JsonRpcParser, JsonRpcRequest, JsonRpcResponse, RequestId,
};

/// Adapter errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum AdapterError {
    #[error("JSON-RPC parse error: {0}")]
    ParseError(#[from] JsonRpcParseError),
    #[error("unknown tool: {0}")]
    UnknownTool(String),
    #[error("DCP error: {0}")]
    DcpError(#[from] DCPError),
    #[error("serialization error: {0}")]
    SerializationError(String),
}

/// MCP to DCP adapter
pub struct McpAdapter {
    /// Tool name to ID cache
    tool_cache: HashMap<String, u16>,
    /// ID to tool name reverse mapping
    id_to_name: HashMap<u16, String>,
}

impl McpAdapter {
    /// Create a new adapter
    pub fn new() -> Self {
        Self {
            tool_cache: HashMap::new(),
            id_to_name: HashMap::new(),
        }
    }

    /// Register a tool mapping
    pub fn register_tool(&mut self, name: impl Into<String>, tool_id: u16) {
        let name = name.into();
        self.tool_cache.insert(name.clone(), tool_id);
        self.id_to_name.insert(tool_id, name);
    }

    /// Resolve MCP tool name to DCP tool_id
    pub fn resolve_tool_name(&self, name: &str) -> Option<u16> {
        self.tool_cache.get(name).copied()
    }

    /// Resolve DCP tool_id to MCP tool name
    pub fn resolve_tool_id(&self, tool_id: u16) -> Option<&str> {
        self.id_to_name.get(&tool_id).map(|s| s.as_str())
    }

    /// Parse an MCP JSON-RPC request
    pub fn parse_request(&self, json: &str) -> Result<JsonRpcRequest, AdapterError> {
        Ok(JsonRpcParser::parse_request(json)?)
    }

    /// Translate MCP request params to DCP arguments
    pub fn translate_params(&self, params: &Option<Value>) -> Vec<u8> {
        match params {
            Some(value) => {
                // For now, serialize params as JSON bytes
                // In a full implementation, this would convert to binary format
                serde_json::to_vec(value).unwrap_or_default()
            }
            None => Vec::new(),
        }
    }

    /// Translate DCP result to MCP response value
    pub fn translate_result(&self, result: &ToolResult) -> Value {
        match result {
            ToolResult::Success(data) => {
                // Try to parse as JSON, otherwise return as string
                serde_json::from_slice(data)
                    .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(data).to_string()))
            }
            ToolResult::Empty => Value::Null,
            ToolResult::Error(err) => {
                serde_json::json!({
                    "error": {
                        "code": *err as i32,
                        "message": err.to_string()
                    }
                })
            }
        }
    }

    /// Format a success response
    pub fn format_success_response(
        &self,
        id: RequestId,
        result: Value,
    ) -> Result<String, AdapterError> {
        let response = JsonRpcResponse::success(id, result);
        JsonRpcParser::format_response(&response)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }

    /// Format an error response
    pub fn format_error_response(
        &self,
        id: RequestId,
        error: JsonRpcError,
    ) -> Result<String, AdapterError> {
        let response = JsonRpcResponse::error(id, error);
        JsonRpcParser::format_response(&response)
            .map_err(|e| AdapterError::SerializationError(e.to_string()))
    }

    /// Handle an MCP initialize request
    pub fn handle_initialize(&self, request: &JsonRpcRequest) -> Result<String, AdapterError> {
        // Build capabilities response
        let result = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {
                    "listChanged": false
                },
                "resources": {
                    "subscribe": false,
                    "listChanged": false
                },
                "prompts": {
                    "listChanged": false
                }
            },
            "serverInfo": {
                "name": "dcp-server",
                "version": "0.1.0"
            }
        });

        self.format_success_response(request.id.clone(), result)
    }

    /// Handle an MCP tools/list request
    pub fn handle_tools_list(&self, request: &JsonRpcRequest) -> Result<String, AdapterError> {
        let tools: Vec<Value> = self
            .tool_cache
            .keys()
            .map(|name| {
                serde_json::json!({
                    "name": name,
                    "description": format!("Tool: {}", name),
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                })
            })
            .collect();

        let result = serde_json::json!({
            "tools": tools
        });

        self.format_success_response(request.id.clone(), result)
    }

    /// Handle an MCP tools/call request
    pub fn handle_tools_call(
        &self,
        request: &JsonRpcRequest,
        router: &BinaryTrieRouter,
    ) -> Result<String, AdapterError> {
        // Extract tool name and arguments from params
        let params = request
            .params
            .as_ref()
            .ok_or(AdapterError::ParseError(JsonRpcParseError::InvalidStructure))?;

        let tool_name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(AdapterError::ParseError(JsonRpcParseError::InvalidStructure))?;

        let arguments = params.get("arguments").cloned();

        // Resolve tool name to ID
        let tool_id = self
            .resolve_tool_name(tool_name)
            .ok_or_else(|| AdapterError::UnknownTool(tool_name.to_string()))?;

        // Execute via router
        let args_bytes = self.translate_params(&arguments);
        let shared_args = crate::dispatch::SharedArgs::new(&args_bytes, 0);

        let result = router.execute(tool_id, &shared_args).map_err(AdapterError::DcpError)?;

        // Translate result
        let result_value = self.translate_result(&result);

        // Format MCP response
        let response_result = serde_json::json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string(&result_value).unwrap_or_default()
            }]
        });

        self.format_success_response(request.id.clone(), response_result)
    }

    /// Handle a generic MCP request
    pub fn handle_request(
        &self,
        json: &str,
        router: &BinaryTrieRouter,
    ) -> Result<String, AdapterError> {
        let request = self.parse_request(json)?;

        match request.method.as_str() {
            "initialize" => self.handle_initialize(&request),
            "tools/list" => self.handle_tools_list(&request),
            "tools/call" => self.handle_tools_call(&request, router),
            _ => {
                // Unknown method
                self.format_error_response(request.id, JsonRpcError::method_not_found())
            }
        }
    }

    /// Get the number of registered tools
    pub fn tool_count(&self) -> usize {
        self.tool_cache.len()
    }
}

impl Default for McpAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_tool() {
        let mut adapter = McpAdapter::new();
        adapter.register_tool("read_file", 1);
        adapter.register_tool("write_file", 2);

        assert_eq!(adapter.resolve_tool_name("read_file"), Some(1));
        assert_eq!(adapter.resolve_tool_name("write_file"), Some(2));
        assert_eq!(adapter.resolve_tool_name("unknown"), None);

        assert_eq!(adapter.resolve_tool_id(1), Some("read_file"));
        assert_eq!(adapter.resolve_tool_id(2), Some("write_file"));
        assert_eq!(adapter.resolve_tool_id(99), None);
    }

    #[test]
    fn test_parse_request() {
        let adapter = McpAdapter::new();
        let json = r#"{"jsonrpc":"2.0","method":"initialize","id":1}"#;

        let request = adapter.parse_request(json).unwrap();
        assert_eq!(request.method, "initialize");
    }

    #[test]
    fn test_translate_params() {
        let adapter = McpAdapter::new();

        let params = Some(serde_json::json!({"path": "/tmp/test.txt"}));
        let bytes = adapter.translate_params(&params);

        assert!(!bytes.is_empty());
        let parsed: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["path"], "/tmp/test.txt");
    }

    #[test]
    fn test_translate_result_success() {
        let adapter = McpAdapter::new();

        let result = ToolResult::Success(b"hello world".to_vec());
        let value = adapter.translate_result(&result);

        assert_eq!(value, Value::String("hello world".to_string()));
    }

    #[test]
    fn test_translate_result_json() {
        let adapter = McpAdapter::new();

        let json_bytes = serde_json::to_vec(&serde_json::json!({"key": "value"})).unwrap();
        let result = ToolResult::Success(json_bytes);
        let value = adapter.translate_result(&result);

        assert_eq!(value["key"], "value");
    }

    #[test]
    fn test_translate_result_error() {
        let adapter = McpAdapter::new();

        let result = ToolResult::Error(DCPError::ToolNotFound);
        let value = adapter.translate_result(&result);

        assert_eq!(value["error"]["code"], DCPError::ToolNotFound as i32);
        assert!(value["error"]["message"].as_str().unwrap().contains("not found"));
    }

    #[test]
    fn test_handle_initialize() {
        let adapter = McpAdapter::new();
        let request = JsonRpcRequest::new("initialize", None, RequestId::Number(1));

        let response_json = adapter.handle_initialize(&request).unwrap();
        let response = JsonRpcParser::parse_response(&response_json).unwrap();

        assert!(response.is_success());
        let result = response.result.unwrap();
        assert!(result["capabilities"]["tools"].is_object());
    }

    #[test]
    fn test_handle_tools_list() {
        let mut adapter = McpAdapter::new();
        adapter.register_tool("read_file", 1);
        adapter.register_tool("write_file", 2);

        let request = JsonRpcRequest::new("tools/list", None, RequestId::Number(1));
        let response_json = adapter.handle_tools_list(&request).unwrap();
        let response = JsonRpcParser::parse_response(&response_json).unwrap();

        assert!(response.is_success());
        let result = response.result.unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 2);
    }

    #[test]
    fn test_format_error_response() {
        let adapter = McpAdapter::new();

        let response = adapter
            .format_error_response(RequestId::Number(1), JsonRpcError::method_not_found())
            .unwrap();

        let parsed = JsonRpcParser::parse_response(&response).unwrap();
        assert!(parsed.is_error());
        assert_eq!(parsed.error.unwrap().code, -32601);
    }
}
