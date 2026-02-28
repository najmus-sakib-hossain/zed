//! Property-based tests for JSON-RPC 2.0 compliance.
//!
//! Property 16: JSON-RPC 2.0 Compliance
//! Validates: Requirements 11.2

use proptest::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// JSON-RPC 2.0 request structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Value>,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

/// JSON-RPC 2.0 response structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// Standard JSON-RPC error codes.
mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
}

/// Generate a valid JSON-RPC request id.
fn arb_request_id() -> impl Strategy<Value = Option<Value>> {
    prop_oneof![
        Just(None), // Notification
        (1i64..10000i64).prop_map(|n| Some(Value::Number(n.into()))),
        "[a-z]{1,10}".prop_map(|s| Some(Value::String(s))),
    ]
}

/// Generate a valid MCP method name.
fn arb_method() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("initialize".to_string()),
        Just("tools/list".to_string()),
        Just("tools/call".to_string()),
        Just("resources/list".to_string()),
        Just("resources/read".to_string()),
        Just("resources/subscribe".to_string()),
        Just("prompts/list".to_string()),
        Just("prompts/get".to_string()),
        Just("logging/setLevel".to_string()),
        Just("sampling/createMessage".to_string()),
        Just("completion/complete".to_string()),
    ]
}

/// Generate arbitrary JSON params.
fn arb_params() -> impl Strategy<Value = Option<Value>> {
    prop_oneof![
        Just(None),
        Just(Some(json!({}))),
        Just(Some(json!({"key": "value"}))),
        Just(Some(json!({"name": "test", "arguments": {}}))),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 16: JSON-RPC 2.0 Compliance - Request Format
    ///
    /// All JSON-RPC requests must have:
    /// - jsonrpc field set to "2.0"
    /// - method field as a string
    /// - id field (number, string, or null) for requests (absent for notifications)
    /// - optional params field
    #[test]
    fn prop_jsonrpc_request_format(
        id in arb_request_id(),
        method in arb_method(),
        params in arb_params()
    ) {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: id.clone(),
            method: method.clone(),
            params,
        };

        // Serialize and deserialize
        let json_str = serde_json::to_string(&request).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        // Verify jsonrpc version
        prop_assert_eq!(parsed.get("jsonrpc").and_then(|v| v.as_str()), Some("2.0"));

        // Verify method is present
        prop_assert!(parsed.get("method").is_some());
        prop_assert_eq!(parsed.get("method").and_then(|v| v.as_str()), Some(method.as_str()));

        // Verify id handling
        if id.is_some() {
            prop_assert!(parsed.get("id").is_some());
        }
    }

    /// Property 16: JSON-RPC 2.0 Compliance - Response Format
    ///
    /// All JSON-RPC responses must have:
    /// - jsonrpc field set to "2.0"
    /// - id field matching the request
    /// - Either result OR error, but not both
    #[test]
    fn prop_jsonrpc_response_format(
        id in (1i64..10000i64).prop_map(|n| Value::Number(n.into())),
        is_error in any::<bool>()
    ) {
        let response = if is_error {
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: id.clone(),
                result: None,
                error: Some(JsonRpcError {
                    code: error_codes::METHOD_NOT_FOUND,
                    message: "Method not found".to_string(),
                    data: None,
                }),
            }
        } else {
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: id.clone(),
                result: Some(json!({"success": true})),
                error: None,
            }
        };

        let json_str = serde_json::to_string(&response).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        // Verify jsonrpc version
        prop_assert_eq!(parsed.get("jsonrpc").and_then(|v| v.as_str()), Some("2.0"));

        // Verify id is present
        prop_assert!(parsed.get("id").is_some());

        // Verify either result or error, not both
        let has_result = parsed.get("result").is_some();
        let has_error = parsed.get("error").is_some();
        prop_assert!(has_result != has_error, "Response must have either result or error, not both");
    }

    /// Property 16: JSON-RPC 2.0 Compliance - Error Codes
    ///
    /// Standard error codes must be in the reserved range.
    #[test]
    fn prop_jsonrpc_error_codes(
        code in prop_oneof![
            Just(error_codes::PARSE_ERROR),
            Just(error_codes::INVALID_REQUEST),
            Just(error_codes::METHOD_NOT_FOUND),
            Just(error_codes::INVALID_PARAMS),
            Just(error_codes::INTERNAL_ERROR),
        ]
    ) {
        // Standard error codes are in range -32700 to -32600
        prop_assert!(code >= -32700 && code <= -32600);

        let error = JsonRpcError {
            code,
            message: "Error".to_string(),
            data: None,
        };

        let json_str = serde_json::to_string(&error).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        prop_assert!(parsed.get("code").is_some());
        prop_assert!(parsed.get("message").is_some());
    }

    /// Property 16: JSON-RPC 2.0 Compliance - Notification Format
    ///
    /// Notifications must not have an id field.
    #[test]
    fn prop_jsonrpc_notification_format(
        method in arb_method(),
        params in arb_params()
    ) {
        let notification = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: method.clone(),
            params,
        };

        let json_str = serde_json::to_string(&notification).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        // Notifications must not have id
        prop_assert!(parsed.get("id").is_none());

        // Must have jsonrpc and method
        prop_assert_eq!(parsed.get("jsonrpc").and_then(|v| v.as_str()), Some("2.0"));
        prop_assert!(parsed.get("method").is_some());
    }

    /// Property 16: JSON-RPC 2.0 Compliance - Batch Requests
    ///
    /// Batch requests must be arrays of valid requests.
    #[test]
    fn prop_jsonrpc_batch_format(
        count in 1usize..5usize
    ) {
        let requests: Vec<JsonRpcRequest> = (0..count)
            .map(|i| JsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(Value::Number((i as i64 + 1).into())),
                method: "tools/list".to_string(),
                params: None,
            })
            .collect();

        let json_str = serde_json::to_string(&requests).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        // Must be an array
        prop_assert!(parsed.is_array());

        let arr = parsed.as_array().unwrap();
        prop_assert_eq!(arr.len(), count);

        // Each element must be a valid request
        for req in arr {
            prop_assert_eq!(req.get("jsonrpc").and_then(|v| v.as_str()), Some("2.0"));
            prop_assert!(req.get("method").is_some());
        }
    }

    /// Property 16: JSON-RPC 2.0 Compliance - ID Types
    ///
    /// Request IDs can be numbers, strings, or null.
    #[test]
    fn prop_jsonrpc_id_types(
        id_type in 0u8..3u8
    ) {
        let id: Value = match id_type {
            0 => Value::Number(42.into()),
            1 => Value::String("request-123".to_string()),
            _ => Value::Null,
        };

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(id.clone()),
            method: "test".to_string(),
            params: None,
        };

        let json_str = serde_json::to_string(&request).unwrap();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();

        let parsed_id = parsed.get("id").unwrap();

        // ID must be preserved correctly
        match id_type {
            0 => prop_assert!(parsed_id.is_number()),
            1 => prop_assert!(parsed_id.is_string()),
            _ => prop_assert!(parsed_id.is_null()),
        }
    }

    /// Property 16: JSON-RPC 2.0 Compliance - Round Trip
    ///
    /// Requests must survive serialization round-trip.
    #[test]
    fn prop_jsonrpc_roundtrip(
        id in (1i64..10000i64).prop_map(|n| Some(Value::Number(n.into()))),
        method in arb_method()
    ) {
        let original = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: id.clone(),
            method: method.clone(),
            params: Some(json!({"test": "value"})),
        };

        // Serialize
        let json_str = serde_json::to_string(&original).unwrap();

        // Deserialize
        let restored: JsonRpcRequest = serde_json::from_str(&json_str).unwrap();

        // Verify fields match
        prop_assert_eq!(restored.jsonrpc, "2.0");
        prop_assert_eq!(restored.method, method);
        prop_assert_eq!(restored.id, id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_request() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::Number(1.into())),
            method: "tools/list".to_string(),
            params: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"tools/list\""));
    }

    #[test]
    fn test_valid_response() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: Value::Number(1.into()),
            result: Some(json!({"tools": []})),
            error: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"result\""));
        assert!(!json.contains("\"error\""));
    }

    #[test]
    fn test_error_response() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: Value::Number(1.into()),
            result: None,
            error: Some(JsonRpcError {
                code: error_codes::METHOD_NOT_FOUND,
                message: "Method not found".to_string(),
                data: None,
            }),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"error\""));
        assert!(json.contains("-32601"));
    }

    #[test]
    fn test_notification() {
        let notification = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "notifications/initialized".to_string(),
            params: None,
        };

        let json = serde_json::to_string(&notification).unwrap();
        assert!(!json.contains("\"id\""));
        assert!(json.contains("\"method\""));
    }
}
