//! Property-based tests for MCP compatibility layer.
//!
//! Feature: dcp-protocol, Property 14: MCP Translation Round-Trip

use dcp::compat::json_rpc::{
    JsonRpcError, JsonRpcParser, JsonRpcRequest, JsonRpcResponse, RequestId,
};
use dcp::dispatch::ToolResult;
use dcp::{DCPError, McpAdapter};
use proptest::prelude::*;
use serde_json::Value;

/// Strategy to generate a valid method name
fn arb_method() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z]+(/[a-z]+)?")
        .unwrap()
        .prop_filter("non-empty method", |s| !s.is_empty())
}

/// Strategy to generate a request ID
fn arb_request_id() -> impl Strategy<Value = RequestId> {
    prop_oneof![
        any::<i64>().prop_map(RequestId::Number),
        "[a-zA-Z0-9-]{1,20}".prop_map(RequestId::String),
    ]
}

/// Strategy to generate simple JSON params
fn arb_params() -> impl Strategy<Value = Option<Value>> {
    prop_oneof![
        Just(None),
        Just(Some(serde_json::json!({}))),
        "[a-z]{1,10}".prop_map(|s| Some(serde_json::json!({"key": s}))),
        any::<i32>().prop_map(|n| Some(serde_json::json!({"number": n}))),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-protocol, Property 14: MCP Translation Round-Trip
    /// For any valid JSON-RPC request, formatting and parsing SHALL preserve
    /// the semantic meaning.
    /// **Validates: Requirements 11.1, 11.3**
    #[test]
    fn prop_request_round_trip(
        method in arb_method(),
        params in arb_params(),
        id in arb_request_id(),
    ) {
        let original = JsonRpcRequest::new(method.clone(), params.clone(), id.clone());

        // Format to JSON
        let json = JsonRpcParser::format_request(&original).unwrap();

        // Parse back
        let parsed = JsonRpcParser::parse_request(&json).unwrap();

        // Verify semantic equivalence
        prop_assert_eq!(parsed.jsonrpc, "2.0");
        prop_assert_eq!(parsed.method, method);
        prop_assert_eq!(parsed.id, id);
        prop_assert_eq!(parsed.params, params);
    }

    /// Feature: dcp-protocol, Property 14: MCP Translation Round-Trip
    /// For any valid JSON-RPC success response, formatting and parsing SHALL
    /// preserve the result.
    /// **Validates: Requirements 11.3, 11.4**
    #[test]
    fn prop_success_response_round_trip(
        id in arb_request_id(),
        result_key in "[a-z]{1,10}",
        result_value in "[a-z]{1,20}",
    ) {
        let result = serde_json::json!({result_key: result_value});
        let original = JsonRpcResponse::success(id.clone(), result.clone());

        // Format to JSON
        let json = JsonRpcParser::format_response(&original).unwrap();

        // Parse back
        let parsed = JsonRpcParser::parse_response(&json).unwrap();

        // Verify semantic equivalence
        prop_assert!(parsed.is_success());
        prop_assert_eq!(parsed.id, id);
        prop_assert_eq!(parsed.result, Some(result));
    }

    /// Feature: dcp-protocol, Property 14: MCP Translation Round-Trip
    /// For any valid JSON-RPC error response, formatting and parsing SHALL
    /// preserve the error.
    /// **Validates: Requirements 11.3, 11.4**
    #[test]
    fn prop_error_response_round_trip(
        id in arb_request_id(),
        code in -32700i32..0,
        message in "[a-zA-Z ]{1,50}",
    ) {
        let error = JsonRpcError::new(code, message.clone());
        let original = JsonRpcResponse::error(id.clone(), error.clone());

        // Format to JSON
        let json = JsonRpcParser::format_response(&original).unwrap();

        // Parse back
        let parsed = JsonRpcParser::parse_response(&json).unwrap();

        // Verify semantic equivalence
        prop_assert!(parsed.is_error());
        prop_assert_eq!(parsed.id, id);
        let parsed_error = parsed.error.unwrap();
        prop_assert_eq!(parsed_error.code, code);
        prop_assert_eq!(parsed_error.message, message);
    }

    /// Feature: dcp-protocol, Property 14: MCP Translation Round-Trip
    /// For any tool registration, name resolution SHALL be bidirectional.
    /// **Validates: Requirements 11.4**
    #[test]
    fn prop_tool_name_resolution_bidirectional(
        tool_name in "[a-z_]{1,20}",
        tool_id in 1u16..65535,
    ) {
        let mut adapter = McpAdapter::new();
        adapter.register_tool(tool_name.clone(), tool_id);

        // Name -> ID -> Name should be consistent
        let resolved_id = adapter.resolve_tool_name(&tool_name);
        prop_assert_eq!(resolved_id, Some(tool_id));

        let resolved_name = adapter.resolve_tool_id(tool_id);
        prop_assert_eq!(resolved_name, Some(tool_name.as_str()));
    }

    /// Feature: dcp-protocol, Property 14: MCP Translation Round-Trip
    /// For any params, translation to bytes and back SHALL preserve content.
    /// **Validates: Requirements 11.1, 11.3**
    #[test]
    fn prop_params_translation_preserves_content(
        key in "[a-z]{1,10}",
        value in "[a-zA-Z0-9]{1,20}",
    ) {
        let adapter = McpAdapter::new();
        let params = Some(serde_json::json!({key.clone(): value.clone()}));

        // Translate to bytes
        let bytes = adapter.translate_params(&params);

        // Parse bytes back as JSON
        let parsed: Value = serde_json::from_slice(&bytes).unwrap();

        // Verify content preserved
        prop_assert_eq!(parsed[&key].as_str(), Some(value.as_str()));
    }

    /// Feature: dcp-protocol, Property 14: MCP Translation Round-Trip
    /// For any ToolResult::Success with text, translation SHALL produce valid JSON.
    /// **Validates: Requirements 11.3, 11.4**
    #[test]
    fn prop_result_translation_success(
        content in "[a-zA-Z ]{1,50}",  // Only letters and spaces to avoid JSON parsing
    ) {
        let adapter = McpAdapter::new();
        let result = ToolResult::Success(content.as_bytes().to_vec());

        let value = adapter.translate_result(&result);

        // Should be a string containing the content
        prop_assert_eq!(value.as_str(), Some(content.as_str()));
    }

    /// Feature: dcp-protocol, Property 14: MCP Translation Round-Trip
    /// For any ToolResult::Success with JSON, translation SHALL preserve structure.
    /// **Validates: Requirements 11.3, 11.4**
    #[test]
    fn prop_result_translation_json(
        key in "[a-z]{1,10}",
        value in "[a-zA-Z0-9]{1,20}",
    ) {
        let adapter = McpAdapter::new();
        let json_content = serde_json::json!({key.clone(): value.clone()});
        let bytes = serde_json::to_vec(&json_content).unwrap();
        let result = ToolResult::Success(bytes);

        let translated = adapter.translate_result(&result);

        // Should preserve JSON structure
        prop_assert_eq!(translated[&key].as_str(), Some(value.as_str()));
    }

    /// Feature: dcp-protocol, Property 14: MCP Translation Round-Trip
    /// For any ToolResult::Error, translation SHALL include error info.
    /// **Validates: Requirements 11.3, 11.4**
    #[test]
    fn prop_result_translation_error(
        error_variant in 1u8..12,
    ) {
        let adapter = McpAdapter::new();
        let dcp_error = match error_variant {
            1 => DCPError::InsufficientData,
            2 => DCPError::InvalidMagic,
            3 => DCPError::UnknownMessageType,
            4 => DCPError::ToolNotFound,
            5 => DCPError::ValidationFailed,
            6 => DCPError::HashMismatch,
            7 => DCPError::SignatureInvalid,
            8 => DCPError::NonceReused,
            9 => DCPError::TimestampExpired,
            10 => DCPError::ChecksumMismatch,
            11 => DCPError::Backpressure,
            _ => DCPError::OutOfBounds,
        };

        let result = ToolResult::Error(dcp_error);
        let value = adapter.translate_result(&result);

        // Should have error structure
        prop_assert!(value["error"].is_object());
        prop_assert!(value["error"]["code"].is_number());
        prop_assert!(value["error"]["message"].is_string());
    }

    /// Feature: dcp-protocol, Property 14: MCP Translation Round-Trip
    /// For any notification (no id), parsing SHALL recognize it as notification.
    /// **Validates: Requirements 11.1**
    #[test]
    fn prop_notification_detection(
        method in arb_method(),
        params in arb_params(),
    ) {
        let notification = JsonRpcRequest::notification(method, params);

        prop_assert!(notification.is_notification());

        let json = JsonRpcParser::format_request(&notification).unwrap();
        let parsed = JsonRpcParser::parse_request(&json).unwrap();

        prop_assert!(parsed.is_notification());
    }

    /// Feature: dcp-protocol, Property 14: MCP Translation Round-Trip
    /// Multiple tool registrations SHALL be independent.
    /// **Validates: Requirements 11.4**
    #[test]
    fn prop_multiple_tool_registrations(
        tools in prop::collection::vec(
            ("[a-z_]{1,10}", 1u16..1000),
            1..20
        ),
    ) {
        let mut adapter = McpAdapter::new();

        // Deduplicate by name and id
        let mut seen_names = std::collections::HashSet::new();
        let mut seen_ids = std::collections::HashSet::new();
        let unique_tools: Vec<_> = tools.into_iter()
            .filter(|(name, id)| seen_names.insert(name.clone()) && seen_ids.insert(*id))
            .collect();

        // Register all tools
        for (name, id) in &unique_tools {
            adapter.register_tool(name.clone(), *id);
        }

        // Verify all registrations
        for (name, id) in &unique_tools {
            prop_assert_eq!(adapter.resolve_tool_name(name), Some(*id));
            prop_assert_eq!(adapter.resolve_tool_id(*id), Some(name.as_str()));
        }

        prop_assert_eq!(adapter.tool_count(), unique_tools.len());
    }
}
