//! Property-based tests for MCP adapter and protocol support.
//!
//! Tests for resource routing, prompt substitution, and protocol compliance.

use proptest::prelude::*;
use std::collections::HashMap;

use dcp::compat::{
    json_rpc::RequestId, CompleteMcpAdapter, JsonRpcParser, JsonRpcRequest, PromptTemplate,
};
use dcp::dispatch::BinaryTrieRouter;
use dcp::resource::{
    uri_matches_template, MemoryResourceHandler, ResourceContent, ResourceRegistry,
};

// ============================================================================
// Property 8: Resource URI Routing
// **Validates: Requirements 4.1, 4.2**
// For any registered resource handler with URI template, and any URI matching
// that template, the server SHALL route the request to the correct handler.
// ============================================================================

fn arb_uri_template() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("file:///{path}".to_string()),
        Just("http://example.com/{resource}".to_string()),
        Just("db:///{table}/{id}".to_string()),
        Just("custom://{type}/{name}".to_string()),
    ]
}

fn arb_matching_uri(template: &str) -> impl Strategy<Value = String> {
    // Generate URIs that match the template
    let prefix = template.split('{').next().unwrap_or("").to_string();
    prop::string::string_regex(&format!("{}[a-z0-9/]+", regex::escape(&prefix)))
        .unwrap()
        .prop_map(|s| s.chars().take(100).collect())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-production, Property 8: Resource URI Routing
    /// For any URI template and matching URI, the handler should be found.
    #[test]
    fn prop_resource_uri_routing(
        template_idx in 0usize..3,
        suffix in "[a-z0-9]{1,20}"
    ) {
        let templates = [
            "file:///{path}",
            "http://example.com/{resource}",
            "custom://{name}",
        ];
        let template = templates[template_idx];
        let prefix = template.split('{').next().unwrap_or("");
        let uri = format!("{}{}", prefix, suffix);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut registry = ResourceRegistry::new();
            let mut handler = MemoryResourceHandler::new(template);
            handler.add_resource(&uri, ResourceContent::text(&uri, "text/plain", "content"));
            registry.register(handler);

            // Should find handler
            let found = registry.match_uri(&uri);
            prop_assert!(found.is_some(), "Should find handler for URI: {}", uri);

            // Should read content
            let content = registry.read(&uri);
            prop_assert!(content.is_ok(), "Should read content for URI: {}", uri);

            Ok(())
        })?;
    }

    /// Feature: dcp-production, Property 8: Resource URI Routing (non-matching)
    /// For URIs that don't match any template, no handler should be found.
    #[test]
    fn prop_resource_uri_routing_no_match(
        uri in "ftp://[a-z]{1,10}/[a-z]{1,10}"
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut registry = ResourceRegistry::new();
            let handler = MemoryResourceHandler::new("file:///{path}");
            registry.register(handler);

            // Should not find handler for non-matching URI
            let found = registry.match_uri(&uri);
            prop_assert!(found.is_none(), "Should not find handler for non-matching URI: {}", uri);

            Ok(())
        })?;
    }
}

// ============================================================================
// Property 11: Prompt Parameter Substitution
// **Validates: Requirements 5.2**
// For any prompt template with placeholders and valid arguments, rendering
// the prompt SHALL substitute all placeholders with their corresponding values.
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-production, Property 11: Prompt Parameter Substitution
    /// For any template and arguments, all placeholders should be substituted.
    #[test]
    fn prop_prompt_parameter_substitution(
        name in "[a-z]{1,10}",
        value in "[a-zA-Z0-9 ]{1,50}"
    ) {
        let template = PromptTemplate::new(
            "test",
            "Test prompt",
            format!("Hello {{{{{}}}}}!", name)
        ).with_argument(&name, "Test arg", true);

        let mut args = HashMap::new();
        args.insert(name.clone(), value.clone());

        let rendered = template.render(&args).unwrap();

        // Should contain the value
        prop_assert!(rendered.contains(&value),
            "Rendered template should contain value '{}', got: {}", value, rendered);

        // Should not contain the placeholder
        let placeholder = format!("{{{{{}}}}}", name);
        prop_assert!(!rendered.contains(&placeholder),
            "Rendered template should not contain placeholder '{}'", placeholder);
    }

    /// Feature: dcp-production, Property 11: Multiple parameters
    /// Multiple placeholders should all be substituted.
    #[test]
    fn prop_prompt_multiple_parameters(
        values in prop::collection::vec("[a-zA-Z0-9]{1,10}", 1..5)
    ) {
        let mut template_str = String::new();
        let mut template = PromptTemplate::new("test", "Test", "");
        let mut args = HashMap::new();

        for (i, value) in values.iter().enumerate() {
            let name = format!("arg{}", i);
            template_str.push_str(&format!("{{{{{}}}}} ", name));
            template = template.with_argument(&name, "Arg", true);
            args.insert(name, value.clone());
        }

        template.template = template_str;
        let rendered = template.render(&args).unwrap();

        // All values should be present
        for value in &values {
            prop_assert!(rendered.contains(value),
                "Rendered should contain '{}'", value);
        }
    }
}

// ============================================================================
// Property 12: Prompt Validation
// **Validates: Requirements 5.3, 5.4, 5.5**
// For any prompt template with required parameters, a request missing any
// required parameter SHALL return a validation error.
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-production, Property 12: Prompt Validation (missing required)
    /// Missing required arguments should cause an error.
    #[test]
    fn prop_prompt_validation_missing_required(
        required_args in prop::collection::vec("[a-z]{1,10}", 1..5)
    ) {
        let mut template = PromptTemplate::new("test", "Test", "template");
        for arg in &required_args {
            template = template.with_argument(arg, "Required arg", true);
        }

        // Empty args should fail
        let args = HashMap::new();
        let result = template.render(&args);
        prop_assert!(result.is_err(), "Should fail with missing required args");
    }

    /// Feature: dcp-production, Property 12: Prompt Validation (optional ok)
    /// Optional arguments can be omitted without error.
    #[test]
    fn prop_prompt_validation_optional_ok(
        optional_args in prop::collection::vec("[a-z]{1,10}", 1..5)
    ) {
        let mut template = PromptTemplate::new("test", "Test", "template");
        for arg in &optional_args {
            template = template.with_argument(arg, "Optional arg", false);
        }

        // Empty args should succeed for optional
        let args = HashMap::new();
        let result = template.render(&args);
        prop_assert!(result.is_ok(), "Should succeed with missing optional args");
    }
}

// ============================================================================
// Property 6: Unknown Method Error
// **Validates: Requirements 3.8**
// For any JSON-RPC request with a method name not in the supported set,
// the server SHALL return error code -32601 (Method not found).
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-production, Property 6: Unknown Method Error
    /// Unknown methods should return -32601.
    #[test]
    fn prop_unknown_method_error(
        method in "unknown/[a-z]{1,20}"
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let adapter = CompleteMcpAdapter::new();
            let router = BinaryTrieRouter::new();

            let json = format!(r#"{{"jsonrpc":"2.0","method":"{}","id":1}}"#, method);
            let result = adapter.handle_request(&json, &router).await.unwrap();

            prop_assert!(result.is_some(), "Should return response for unknown method");

            let response = JsonRpcParser::parse_response(&result.unwrap()).unwrap();
            prop_assert!(response.is_error(), "Should be error response");
            prop_assert_eq!(response.error.unwrap().code, -32601,
                "Should return -32601 for unknown method");

            Ok(())
        })?;
    }
}

// ============================================================================
// Property 7: Notification No-Response
// **Validates: Requirements 3.9**
// For any valid JSON-RPC notification (request without `id` field),
// the server SHALL process it without sending any response.
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-production, Property 7: Notification No-Response
    /// Notifications should not produce a response.
    #[test]
    fn prop_notification_no_response(
        _dummy in 0..10i32
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let adapter = CompleteMcpAdapter::new();
            let router = BinaryTrieRouter::new();

            // Notification has no id
            let json = r#"{"jsonrpc":"2.0","method":"initialized"}"#;
            let result = adapter.handle_request(json, &router).await.unwrap();

            prop_assert!(result.is_none(), "Notification should not produce response");

            Ok(())
        })?;
    }
}

// ============================================================================
// URI Template Matching Properties
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// URI template matching is consistent.
    #[test]
    fn prop_uri_template_matching_consistent(
        prefix in "[a-z]{3,10}://",
        suffix in "[a-z0-9/]{1,20}"
    ) {
        let template = format!("{}{{param}}", prefix);
        let uri = format!("{}{}", prefix, suffix);

        let matches = uri_matches_template(&uri, &template);

        // If prefix matches, should match
        if uri.starts_with(&prefix) {
            prop_assert!(matches, "URI {} should match template {}", uri, template);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uri_template_basic() {
        assert!(uri_matches_template("file:///test.txt", "file:///{path}"));
        assert!(uri_matches_template(
            "http://example.com/users/123",
            "http://example.com/users/{id}"
        ));
        assert!(!uri_matches_template("ftp://test", "http://{host}"));
    }

    #[tokio::test]
    async fn test_resource_routing() {
        let mut registry = ResourceRegistry::new();
        let mut handler = MemoryResourceHandler::new("file:///{path}");
        handler.add_resource(
            "file:///test.txt",
            ResourceContent::text("file:///test.txt", "text/plain", "Hello"),
        );
        registry.register(handler);

        let found = registry.match_uri("file:///test.txt");
        assert!(found.is_some());
    }
}

// ============================================================================
// Property 9: Subscription Notification
// **Validates: Requirements 4.5**
// For any resource with active subscriptions, when the resource changes,
// ALL subscribers SHALL receive a notification containing the resource URI.
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-production, Property 9: Subscription Notification
    /// All subscribers should be notified when a resource changes.
    #[test]
    fn prop_subscription_notification(
        num_subscribers in 1usize..10
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut registry = ResourceRegistry::new();
            let handler = MemoryResourceHandler::new("file:///{path}").with_subscriptions();
            registry.register(handler);

            let uri = "file:///test.txt";
            let notified_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

            // Subscribe multiple times
            let mut sub_ids = Vec::new();
            for _ in 0..num_subscribers {
                let count = std::sync::Arc::clone(&notified_count);
                let sub_id = registry.subscribe(uri, move |_| {
                    count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }).await.unwrap();
                sub_ids.push(sub_id);
            }

            prop_assert_eq!(registry.subscription_count(uri).await, num_subscribers,
                "Should have {} subscriptions", num_subscribers);

            // Notify change
            registry.notify_change(uri).await;

            // All subscribers should be notified
            prop_assert_eq!(notified_count.load(std::sync::atomic::Ordering::SeqCst), num_subscribers,
                "All {} subscribers should be notified", num_subscribers);

            Ok(())
        })?;
    }
}

// ============================================================================
// Property 10: Resource Pagination
// **Validates: Requirements 4.6**
// For any resource list with more items than the page size, pagination with
// cursor SHALL eventually return all items exactly once.
// ============================================================================

// Note: The current implementation doesn't have pagination across handlers,
// but we test that list_all returns all resources from all handlers.

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-production, Property 10: Resource Pagination (all items returned)
    /// All registered resources should be returned in the list.
    #[test]
    fn prop_resource_pagination_all_items(
        num_resources in 1usize..20
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut registry = ResourceRegistry::new();
            let mut handler = MemoryResourceHandler::new("file:///{path}");

            // Add resources
            for i in 0..num_resources {
                let uri = format!("file:///resource{}.txt", i);
                handler.add_resource(&uri, ResourceContent::text(&uri, "text/plain", "content"));
            }
            registry.register(handler);

            // List all
            let list = registry.list_all(None).unwrap();

            prop_assert_eq!(list.resources.len(), num_resources,
                "Should return all {} resources", num_resources);

            Ok(())
        })?;
    }
}
