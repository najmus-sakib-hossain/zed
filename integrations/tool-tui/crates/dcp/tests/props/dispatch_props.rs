//! Property-based tests for dispatch layer.
//!
//! Feature: dcp-protocol, Property 6: Tool Dispatch Correctness

use dcp::dispatch::{BinaryTrieRouter, SharedArgs, ToolHandler, ToolResult};
use dcp::protocol::schema::{InputSchema, ToolSchema};
use dcp::DCPError;
use proptest::prelude::*;

// Test handler implementation for property tests
struct PropTestHandler {
    schema: ToolSchema,
}

impl PropTestHandler {
    fn new(id: u16, name: String) -> Self {
        // Leak the string to get a 'static lifetime (acceptable in tests)
        let name: &'static str = Box::leak(name.into_boxed_str());
        Self {
            schema: ToolSchema {
                name,
                id,
                description: "Property test tool",
                input: InputSchema::new(),
            },
        }
    }
}

impl ToolHandler for PropTestHandler {
    fn execute(&self, _args: &SharedArgs) -> Result<ToolResult, DCPError> {
        // Return the tool ID as the result for verification
        Ok(ToolResult::success(self.schema.id.to_le_bytes().to_vec()))
    }

    fn schema(&self) -> &ToolSchema {
        &self.schema
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-protocol, Property 6: Tool Dispatch Correctness
    /// For any valid tool_id in range [0, max_registered_id], the Binary_Trie_Router
    /// SHALL dispatch to the correct handler.
    /// **Validates: Requirements 4.2, 4.3**
    #[test]
    fn prop_dispatch_valid_tool_id(
        tool_ids in prop::collection::vec(0u16..1000, 1..20),
    ) {
        let mut router = BinaryTrieRouter::new();

        // Register all tools
        for &id in &tool_ids {
            let name = format!("tool_{}", id);
            let handler = Box::new(PropTestHandler::new(id, name));
            let _ = router.register(handler); // Ignore duplicate registrations
        }

        // Verify dispatch returns correct handler for each registered tool
        for &id in &tool_ids {
            if let Some(handler) = router.dispatch(id) {
                prop_assert_eq!(handler.tool_id(), id);
            }
        }
    }

    /// Feature: dcp-protocol, Property 6: Tool Dispatch Correctness
    /// For any invalid tool_id, the Binary_Trie_Router SHALL return an error
    /// without panicking.
    /// **Validates: Requirements 4.5**
    #[test]
    fn prop_dispatch_invalid_tool_id(
        registered_ids in prop::collection::vec(0u16..100, 1..10),
        query_id in 0u16..1000,
    ) {
        let mut router = BinaryTrieRouter::new();

        // Register some tools
        for &id in &registered_ids {
            let name = format!("tool_{}", id);
            let handler = Box::new(PropTestHandler::new(id, name));
            let _ = router.register(handler);
        }

        // Query should not panic
        let result = router.dispatch(query_id);

        // If query_id is in registered_ids, should find handler
        // Otherwise, should return None
        if registered_ids.contains(&query_id) {
            prop_assert!(result.is_some());
            prop_assert_eq!(result.unwrap().tool_id(), query_id);
        } else {
            prop_assert!(result.is_none());
        }
    }

    /// Feature: dcp-protocol, Property 6: Tool Dispatch Correctness
    /// Execute on invalid tool_id SHALL return ToolNotFound error.
    /// **Validates: Requirements 4.5**
    #[test]
    fn prop_execute_invalid_returns_error(
        registered_ids in prop::collection::vec(0u16..50, 0..5),
        query_id in 100u16..200, // Always outside registered range
    ) {
        let mut router = BinaryTrieRouter::new();

        for &id in &registered_ids {
            let name = format!("tool_{}", id);
            let handler = Box::new(PropTestHandler::new(id, name));
            let _ = router.register(handler);
        }

        let args = SharedArgs::new(&[], 0);
        let result = router.execute(query_id, &args);

        prop_assert_eq!(result, Err(DCPError::ToolNotFound));
    }

    /// Feature: dcp-protocol, Property 6: Tool Dispatch Correctness
    /// Name resolution SHALL return correct tool_id for registered tools.
    /// **Validates: Requirements 4.3**
    #[test]
    fn prop_name_resolution(
        tool_ids in prop::collection::vec(0u16..500, 1..15),
    ) {
        let mut router = BinaryTrieRouter::new();

        // Register tools and track names
        let mut registered: Vec<(u16, String)> = Vec::new();
        for &id in &tool_ids {
            let name = format!("tool_{}", id);
            let handler = Box::new(PropTestHandler::new(id, name.clone()));
            if router.register(handler).is_ok() {
                registered.push((id, name));
            }
        }

        // Verify name resolution
        for (id, name) in &registered {
            let resolved = router.resolve_name(name);
            prop_assert_eq!(resolved, Some(*id));
        }

        // Unknown names should return None
        prop_assert_eq!(router.resolve_name("unknown_tool"), None);
    }

    /// Feature: dcp-protocol, Property 6: Tool Dispatch Correctness
    /// Execute on valid tool_id SHALL return correct result.
    /// **Validates: Requirements 4.2**
    #[test]
    fn prop_execute_valid_returns_result(
        tool_id in 0u16..100,
    ) {
        let mut router = BinaryTrieRouter::new();

        let name = format!("tool_{}", tool_id);
        let handler = Box::new(PropTestHandler::new(tool_id, name));
        router.register(handler).unwrap();

        let args = SharedArgs::new(&[], 0);
        let result = router.execute(tool_id, &args).unwrap();

        prop_assert!(result.is_success());
        // Verify the result contains the tool_id
        let payload = result.payload().unwrap();
        let returned_id = u16::from_le_bytes([payload[0], payload[1]]);
        prop_assert_eq!(returned_id, tool_id);
    }
}
