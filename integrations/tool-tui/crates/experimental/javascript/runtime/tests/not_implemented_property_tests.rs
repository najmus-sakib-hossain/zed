//! Property-based tests for unimplemented API error format
//!
//! Property 11: Unimplemented API Error Format
//! Validates: Requirements 4.6
//!
//! These tests verify that the "Not implemented" error messages
//! follow a consistent format as required by the specification.

use dx_js_runtime::error::{not_implemented, not_implemented_with_context, DxError};
use proptest::prelude::*;

/// Strategy to generate valid API names
fn arb_api_name() -> impl Strategy<Value = String> {
    prop_oneof![
        // Simple API names
        "[a-z][a-zA-Z0-9_]{0,20}",
        // Namespaced API names (e.g., fs.watch)
        "[a-z][a-zA-Z0-9_]{0,10}\\.[a-z][a-zA-Z0-9_]{0,10}",
        // Deeply nested API names (e.g., crypto.subtle.encrypt)
        "[a-z][a-zA-Z0-9_]{0,8}\\.[a-z][a-zA-Z0-9_]{0,8}\\.[a-z][a-zA-Z0-9_]{0,8}",
    ]
}

/// Strategy to generate reason strings
fn arb_reason() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 ]{1,50}"
}

/// Strategy to generate alternative strings
fn arb_alternative() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 ._]{1,50}"
}

proptest! {
    /// Property: Error message contains "Not implemented:" prefix
    ///
    /// All not_implemented errors should:
    /// 1. Start with "Not implemented:"
    /// 2. Contain the API name
    #[test]
    fn prop_error_message_format(
        api_name in arb_api_name(),
    ) {
        let err = not_implemented(&api_name);
        let message = err.to_string();

        // Property 1: Message should contain "Not implemented:"
        prop_assert!(
            message.contains("Not implemented:"),
            "Error message '{}' should contain 'Not implemented:'",
            message
        );

        // Property 2: Message should contain the API name
        prop_assert!(
            message.contains(&api_name),
            "Error message '{}' should contain API name '{}'",
            message,
            api_name
        );
    }

    /// Property: Error message format is consistent
    ///
    /// The error message should follow the exact format:
    /// "Not implemented: [api_name]"
    #[test]
    fn prop_error_message_exact_format(
        api_name in arb_api_name(),
    ) {
        let err = not_implemented(&api_name);
        let message = err.to_string();

        // The message should contain the exact format
        let expected_substring = format!("Not implemented: {}", api_name);
        prop_assert!(
            message.contains(&expected_substring),
            "Error message '{}' should contain exact format '{}'",
            message,
            expected_substring
        );
    }

    /// Property: DxError::not_implemented produces same result as function
    ///
    /// Both ways of creating the error should produce identical messages
    #[test]
    fn prop_error_creation_consistency(
        api_name in arb_api_name(),
    ) {
        let err1 = not_implemented(&api_name);
        let err2 = DxError::not_implemented(&api_name);

        // Property: Both should produce the same message
        prop_assert_eq!(
            err1.to_string(),
            err2.to_string(),
            "Function and method should produce identical errors"
        );
    }

    /// Property: Error with context contains all parts
    ///
    /// When creating an error with context, it should:
    /// 1. Contain the API name
    /// 2. Contain the reason
    /// 3. Contain the alternative (if provided)
    #[test]
    fn prop_error_with_context_contains_all_parts(
        api_name in arb_api_name(),
        reason in arb_reason(),
        alternative in prop::option::of(arb_alternative()),
    ) {
        let err = not_implemented_with_context(&api_name, &reason, alternative.as_deref());
        let message = err.to_string();

        // Property 1: Should contain API name
        prop_assert!(
            message.contains(&api_name),
            "Error message should contain API name"
        );

        // Property 2: Should contain reason
        prop_assert!(
            message.contains(&reason),
            "Error message should contain reason"
        );

        // Property 3: Should contain alternative if provided
        if let Some(alt) = &alternative {
            prop_assert!(
                message.contains(alt),
                "Error message should contain alternative"
            );
        }
    }

    /// Property: Error is a RuntimeError variant
    ///
    /// The not_implemented error should be a RuntimeError
    #[test]
    fn prop_error_is_runtime_error(
        api_name in arb_api_name(),
    ) {
        let err = not_implemented(&api_name);

        // Property: Should be a RuntimeError
        match err {
            DxError::RuntimeError(_) => {
                // Expected
            }
            _ => {
                prop_assert!(false, "not_implemented should return RuntimeError");
            }
        }
    }

    /// Property: API names with special characters are preserved
    ///
    /// API names containing dots (for namespacing) should be preserved
    #[test]
    fn prop_namespaced_api_names_preserved(
        namespace in "[a-z][a-zA-Z0-9_]{0,10}",
        method in "[a-z][a-zA-Z0-9_]{0,10}",
    ) {
        let api_name = format!("{}.{}", namespace, method);
        let err = not_implemented(&api_name);
        let message = err.to_string();

        // Property: Full namespaced name should be in message
        prop_assert!(
            message.contains(&api_name),
            "Namespaced API name '{}' should be preserved in message '{}'",
            api_name,
            message
        );
    }
}

// ============================================================================
// Unit tests for specific cases
// ============================================================================

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_simple_api_name() {
        let err = not_implemented("watch");
        assert!(err.to_string().contains("Not implemented: watch"));
    }

    #[test]
    fn test_namespaced_api_name() {
        let err = not_implemented("fs.watch");
        assert!(err.to_string().contains("Not implemented: fs.watch"));
    }

    #[test]
    fn test_deeply_nested_api_name() {
        let err = not_implemented("crypto.subtle.encrypt");
        assert!(err.to_string().contains("Not implemented: crypto.subtle.encrypt"));
    }

    #[test]
    fn test_error_with_context() {
        let err = not_implemented_with_context(
            "fs.watch",
            "requires native file system events",
            Some("use polling with fs.watchFile instead"),
        );
        let message = err.to_string();

        assert!(message.contains("fs.watch"));
        assert!(message.contains("requires native file system events"));
        assert!(message.contains("use polling with fs.watchFile instead"));
    }

    #[test]
    fn test_error_with_context_no_alternative() {
        let err = not_implemented_with_context(
            "process.dlopen",
            "native module loading not supported",
            None,
        );
        let message = err.to_string();

        assert!(message.contains("process.dlopen"));
        assert!(message.contains("native module loading not supported"));
        assert!(!message.contains("Alternative:"));
    }

    #[test]
    fn test_error_is_runtime_error() {
        let err = not_implemented("test");
        match err {
            DxError::RuntimeError(msg) => {
                assert!(msg.contains("Not implemented: test"));
            }
            _ => panic!("Expected RuntimeError"),
        }
    }
}
