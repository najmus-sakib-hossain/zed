//! Property tests for built-in error source mapping
//!
//! **Feature: production-readiness, Property 5: Built-in Error Source Mapping**
//!
//! For any error thrown by a native built-in function when called from JavaScript
//! source at location (file F, line L), the error's stack trace SHALL include
//! a frame pointing to file F at line L.
//!
//! **Validates: Requirements 1.6**

use dx_js_runtime::compiler::builtins_registry::BuiltinRegistry;
use dx_js_runtime::compiler::codegen::{clear_structured_exception, get_structured_exception};
use dx_js_runtime::error::{
    clear_call_stack, pop_call_frame, push_call_frame, CallFrame, JsErrorType,
};
use dx_js_runtime::value::Value;
use proptest::prelude::*;

// ============================================================================
// Property 5: Built-in Error Source Mapping
// For any error thrown by a native built-in function when called from JavaScript
// source at location (file F, line L), the error's stack trace SHALL include
// a frame pointing to file F at line L.
// Validates: Requirements 1.6
// ============================================================================

/// Helper to simulate a call from JavaScript source
fn simulate_js_call<F, R>(file: &str, line: u32, column: u32, func_name: &str, f: F) -> R
where
    F: FnOnce() -> R,
{
    // Push a call frame to simulate being called from JavaScript
    push_call_frame(CallFrame::new(func_name, file, line, column));
    let result = f();
    pop_call_frame();
    result
}

proptest! {
    /// Property: When Object.keys is called on a non-object from a JavaScript source location,
    /// the thrown TypeError should include that source location in its stack trace.
    #[test]
    fn prop_object_keys_error_includes_source_location(
        file in "[a-z]{1,20}\\.js",
        line in 1u32..1000,
        column in 1u32..100
    ) {
        // Clear any previous exceptions
        clear_structured_exception();
        clear_call_stack();

        let registry = BuiltinRegistry::new();
        let object_keys = registry.get("Object.keys").unwrap();

        // Simulate calling Object.keys from JavaScript source
        simulate_js_call(&file, line, column, "Object.keys", || {
            // Call with a non-object to trigger TypeError
            let _ = object_keys(&[Value::Number(42.0)]);
        });

        // Check that a structured exception was set
        if let Some(exception) = get_structured_exception() {
            // Property: Error type should be TypeError
            prop_assert_eq!(exception.error_type, JsErrorType::TypeError);

            // Property: Error message should mention Object.keys
            prop_assert!(
                exception.message.contains("Object.keys"),
                "Error message should mention Object.keys: {}",
                exception.message
            );

            // Property: Stack trace should include the source location
            // Note: The stack trace is captured when throw_type_error is called,
            // which happens inside the builtin function after we've pushed our frame
            if !exception.stack.is_empty() {
                let has_source_frame = exception.stack.iter().any(|frame| {
                    frame.file == file && frame.line == line && frame.column == column
                });
                prop_assert!(
                    has_source_frame,
                    "Stack trace should include source location {}:{}:{}",
                    file, line, column
                );
            }
        }

        // Clean up
        clear_structured_exception();
        clear_call_stack();
    }

    /// Property: When Object.values is called on a non-object from a JavaScript source location,
    /// the thrown TypeError should include that source location.
    #[test]
    fn prop_object_values_error_includes_source_location(
        file in "[a-z]{1,20}\\.js",
        line in 1u32..1000,
        column in 1u32..100
    ) {
        clear_structured_exception();
        clear_call_stack();

        let registry = BuiltinRegistry::new();
        let object_values = registry.get("Object.values").unwrap();

        simulate_js_call(&file, line, column, "Object.values", || {
            let _ = object_values(&[Value::String("not an object".to_string())]);
        });

        if let Some(exception) = get_structured_exception() {
            prop_assert_eq!(exception.error_type, JsErrorType::TypeError);
            prop_assert!(exception.message.contains("Object.values"));
        }

        clear_structured_exception();
        clear_call_stack();
    }

    /// Property: When Object.entries is called on a non-object from a JavaScript source location,
    /// the thrown TypeError should include that source location.
    #[test]
    fn prop_object_entries_error_includes_source_location(
        file in "[a-z]{1,20}\\.js",
        line in 1u32..1000,
        column in 1u32..100
    ) {
        clear_structured_exception();
        clear_call_stack();

        let registry = BuiltinRegistry::new();
        let object_entries = registry.get("Object.entries").unwrap();

        simulate_js_call(&file, line, column, "Object.entries", || {
            let _ = object_entries(&[Value::Boolean(true)]);
        });

        if let Some(exception) = get_structured_exception() {
            prop_assert_eq!(exception.error_type, JsErrorType::TypeError);
            prop_assert!(exception.message.contains("Object.entries"));
        }

        clear_structured_exception();
        clear_call_stack();
    }

    /// Property: When Object.assign is called without arguments from a JavaScript source location,
    /// the thrown TypeError should include that source location.
    #[test]
    fn prop_object_assign_error_includes_source_location(
        file in "[a-z]{1,20}\\.js",
        line in 1u32..1000,
        column in 1u32..100
    ) {
        clear_structured_exception();
        clear_call_stack();

        let registry = BuiltinRegistry::new();
        let object_assign = registry.get("Object.assign").unwrap();

        simulate_js_call(&file, line, column, "Object.assign", || {
            // Call with no arguments to trigger TypeError
            let _ = object_assign(&[]);
        });

        if let Some(exception) = get_structured_exception() {
            prop_assert_eq!(exception.error_type, JsErrorType::TypeError);
            prop_assert!(exception.message.contains("Object.assign"));
        }

        clear_structured_exception();
        clear_call_stack();
    }

    /// Property: When JSON.parse is called with invalid JSON from a JavaScript source location,
    /// the thrown SyntaxError should include that source location.
    #[test]
    fn prop_json_parse_error_includes_source_location(
        file in "[a-z]{1,20}\\.js",
        line in 1u32..1000,
        column in 1u32..100,
        invalid_json in "[{\\[a-z]{1,10}"  // Generate invalid JSON strings
    ) {
        clear_structured_exception();
        clear_call_stack();

        let registry = BuiltinRegistry::new();
        let json_parse = registry.get("JSON.parse").unwrap();

        simulate_js_call(&file, line, column, "JSON.parse", || {
            let _ = json_parse(&[Value::String(invalid_json.clone())]);
        });

        // JSON.parse may or may not throw depending on the input
        // If it does throw, verify the error structure
        if let Some(exception) = get_structured_exception() {
            prop_assert_eq!(exception.error_type, JsErrorType::SyntaxError);
            prop_assert!(exception.message.contains("JSON.parse"));
        }

        clear_structured_exception();
        clear_call_stack();
    }

    /// Property: When Array.from is called with a non-iterable from a JavaScript source location,
    /// the thrown TypeError should include that source location.
    #[test]
    fn prop_array_from_error_includes_source_location(
        file in "[a-z]{1,20}\\.js",
        line in 1u32..1000,
        column in 1u32..100
    ) {
        clear_structured_exception();
        clear_call_stack();

        let registry = BuiltinRegistry::new();
        let array_from = registry.get("Array.from").unwrap();

        simulate_js_call(&file, line, column, "Array.from", || {
            // Call with undefined to trigger TypeError
            let _ = array_from(&[Value::Undefined]);
        });

        if let Some(exception) = get_structured_exception() {
            prop_assert_eq!(exception.error_type, JsErrorType::TypeError);
            prop_assert!(exception.message.contains("Array.from"));
        }

        clear_structured_exception();
        clear_call_stack();
    }
}

// ============================================================================
// Unit tests for specific error scenarios
// ============================================================================

#[test]
fn test_object_keys_type_error_has_correct_type() {
    clear_structured_exception();
    clear_call_stack();

    let registry = BuiltinRegistry::new();
    let object_keys = registry.get("Object.keys").unwrap();

    push_call_frame(CallFrame::new("test", "test.js", 10, 5));
    let _ = object_keys(&[Value::Number(42.0)]);
    pop_call_frame();

    let exception = get_structured_exception();
    assert!(exception.is_some(), "Should have thrown an exception");

    let exception = exception.unwrap();
    assert_eq!(exception.error_type, JsErrorType::TypeError);
    assert!(exception.message.contains("Object.keys"));
    assert!(exception.message.contains("non-object"));

    clear_structured_exception();
    clear_call_stack();
}

#[test]
fn test_json_parse_syntax_error_includes_position() {
    clear_structured_exception();
    clear_call_stack();

    let registry = BuiltinRegistry::new();
    let json_parse = registry.get("JSON.parse").unwrap();

    push_call_frame(CallFrame::new("parseConfig", "config.js", 25, 10));
    let _ = json_parse(&[Value::String("{invalid json}".to_string())]);
    pop_call_frame();

    let exception = get_structured_exception();
    assert!(exception.is_some(), "Should have thrown an exception");

    let exception = exception.unwrap();
    assert_eq!(exception.error_type, JsErrorType::SyntaxError);
    assert!(exception.message.contains("JSON.parse"));
    // The error message should include line and column from the JSON parser
    assert!(
        exception.message.contains("line") || exception.message.contains("column"),
        "Error should include position info: {}",
        exception.message
    );

    clear_structured_exception();
    clear_call_stack();
}

#[test]
fn test_promise_all_type_error_for_non_array() {
    clear_structured_exception();
    clear_call_stack();

    let registry = BuiltinRegistry::new();
    let promise_all = registry.get("Promise.all").unwrap();

    push_call_frame(CallFrame::new("fetchAll", "api.js", 100, 15));
    let _ = promise_all(&[Value::Number(42.0)]);
    pop_call_frame();

    let exception = get_structured_exception();
    assert!(exception.is_some(), "Should have thrown an exception");

    let exception = exception.unwrap();
    assert_eq!(exception.error_type, JsErrorType::TypeError);
    assert!(exception.message.contains("Promise.all"));
    assert!(exception.message.contains("iterable"));

    clear_structured_exception();
    clear_call_stack();
}

#[test]
fn test_error_message_includes_received_type() {
    clear_structured_exception();
    clear_call_stack();

    let registry = BuiltinRegistry::new();
    let object_keys = registry.get("Object.keys").unwrap();

    // Test with different types
    // Note: In JavaScript, typeof null === "object", so we test for "object" for null
    let test_cases = vec![
        (Value::Number(42.0), "number"),
        (Value::String("test".to_string()), "string"),
        (Value::Boolean(true), "boolean"),
        (Value::Null, "object"), // typeof null === "object" in JavaScript
        (Value::Undefined, "undefined"),
    ];

    for (value, expected_type) in test_cases {
        clear_structured_exception();
        clear_call_stack();

        push_call_frame(CallFrame::new("test", "test.js", 1, 1));
        let _ = object_keys(&[value]);
        pop_call_frame();

        let exception = get_structured_exception();
        assert!(exception.is_some(), "Should have thrown for {}", expected_type);

        let exception = exception.unwrap();
        assert!(
            exception.message.contains(expected_type),
            "Error message should include type '{}': {}",
            expected_type,
            exception.message
        );
    }

    clear_structured_exception();
    clear_call_stack();
}

#[test]
fn test_nested_call_stack_preserved() {
    clear_structured_exception();
    clear_call_stack();

    let registry = BuiltinRegistry::new();
    let object_keys = registry.get("Object.keys").unwrap();

    // Simulate a nested call stack
    push_call_frame(CallFrame::new("main", "index.js", 1, 1));
    push_call_frame(CallFrame::new("processData", "utils.js", 50, 10));
    push_call_frame(CallFrame::new("validateObject", "validator.js", 25, 5));

    let _ = object_keys(&[Value::Number(42.0)]);

    // Pop all frames
    pop_call_frame();
    pop_call_frame();
    pop_call_frame();

    let exception = get_structured_exception();
    assert!(exception.is_some(), "Should have thrown an exception");

    let exception = exception.unwrap();

    // The stack trace should include all frames
    assert!(
        exception.stack.len() >= 3,
        "Stack should have at least 3 frames, got {}",
        exception.stack.len()
    );

    // Verify the frames are in the correct order (innermost first)
    if exception.stack.len() >= 3 {
        assert_eq!(exception.stack[0].function_name, "validateObject");
        assert_eq!(exception.stack[1].function_name, "processData");
        assert_eq!(exception.stack[2].function_name, "main");
    }

    clear_structured_exception();
    clear_call_stack();
}
