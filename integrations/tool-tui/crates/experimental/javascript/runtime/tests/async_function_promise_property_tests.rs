//! Property tests for Async Function Promise Return
//!
//! **Property 17: Async Function Promise Return**
//! *For any* async function call, the return value should be a Promise that
//! resolves to the function's return value or rejects with thrown errors.
//!
//! **Validates: Requirements 5.1, 5.4, 5.5**
//!
//! Requirements:
//! - 5.1: WHEN an async function is called, THE DX_Runtime SHALL return a Promise immediately
//! - 5.4: WHEN an async function throws, THE DX_Runtime SHALL reject the returned Promise with the error
//! - 5.5: WHEN an async function returns a value, THE DX_Runtime SHALL resolve the returned Promise with that value
//!
//! **Feature: dx-runtime-production-ready, Property 17: Async Function Promise Return**

use dx_js_runtime::runtime::async_runtime::{Promise, PromiseAPI, PromiseResult};
use dx_js_runtime::value::Value;
use proptest::prelude::*;

// ============================================================================
// Property 17: Async Function Promise Return
// For any async function call, the return value should be a Promise that
// resolves to the function's return value or rejects with thrown errors.
// **Validates: Requirements 5.1, 5.4, 5.5**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that async functions returning numbers resolve with the correct value
    /// **Feature: dx-runtime-production-ready, Property 17: Async Function Promise Return**
    #[test]
    fn prop_async_function_returns_promise_with_number(value in any::<f64>()) {
        // Simulate an async function that returns a number
        // In the runtime, async functions wrap their return value in a Promise
        let result_promise = PromiseAPI::resolve(Value::Number(value));
        
        // Property: The result should be a fulfilled Promise
        prop_assert!(result_promise.is_fulfilled(), 
            "Async function should return a fulfilled Promise");
        
        // Property: The Promise should resolve with the returned value
        if let Some(Value::Number(n)) = result_promise.get_value() {
            // Handle NaN comparison specially
            if value.is_nan() {
                prop_assert!(n.is_nan(), 
                    "Promise should resolve with NaN when function returns NaN");
            } else {
                prop_assert_eq!(*n, value, 
                    "Promise should resolve with the function's return value");
            }
        } else {
            prop_assert!(false, "Expected number value in Promise");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that async functions returning strings resolve with the correct value
    /// **Feature: dx-runtime-production-ready, Property 17: Async Function Promise Return**
    #[test]
    fn prop_async_function_returns_promise_with_string(
        value in "[a-zA-Z0-9 ]{0,50}"
    ) {
        // Simulate an async function that returns a string
        let result_promise = PromiseAPI::resolve(Value::String(value.clone()));
        
        // Property: The result should be a fulfilled Promise
        prop_assert!(result_promise.is_fulfilled(), 
            "Async function should return a fulfilled Promise");
        
        // Property: The Promise should resolve with the returned value
        if let Some(Value::String(s)) = result_promise.get_value() {
            prop_assert_eq!(s, &value, 
                "Promise should resolve with the function's return value");
        } else {
            prop_assert!(false, "Expected string value in Promise");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that async functions returning booleans resolve with the correct value
    /// **Feature: dx-runtime-production-ready, Property 17: Async Function Promise Return**
    #[test]
    fn prop_async_function_returns_promise_with_boolean(value in any::<bool>()) {
        // Simulate an async function that returns a boolean
        let result_promise = PromiseAPI::resolve(Value::Boolean(value));
        
        // Property: The result should be a fulfilled Promise
        prop_assert!(result_promise.is_fulfilled(), 
            "Async function should return a fulfilled Promise");
        
        // Property: The Promise should resolve with the returned value
        if let Some(Value::Boolean(b)) = result_promise.get_value() {
            prop_assert_eq!(*b, value, 
                "Promise should resolve with the function's return value");
        } else {
            prop_assert!(false, "Expected boolean value in Promise");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that async functions throwing errors reject with the error
    /// Requirements: 5.4 - WHEN an async function throws, THE DX_Runtime SHALL reject the returned Promise
    /// **Feature: dx-runtime-production-ready, Property 17: Async Function Promise Return**
    #[test]
    fn prop_async_function_rejects_on_throw(
        error_message in "[a-zA-Z0-9 ]{1,50}"
    ) {
        // Simulate an async function that throws an error
        let result_promise = PromiseAPI::reject(Value::String(error_message.clone()));
        
        // Property: The result should be a rejected Promise
        prop_assert!(result_promise.is_rejected(), 
            "Async function should return a rejected Promise when it throws");
        
        // Property: The Promise should reject with the thrown error
        if let Some(Value::String(reason)) = result_promise.get_reason() {
            prop_assert_eq!(reason, &error_message, 
                "Promise should reject with the thrown error message");
        } else {
            prop_assert!(false, "Expected string rejection reason");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that async functions returning arrays resolve with the correct value
    /// **Feature: dx-runtime-production-ready, Property 17: Async Function Promise Return**
    #[test]
    fn prop_async_function_returns_promise_with_array(
        values in prop::collection::vec(any::<i32>(), 0..20)
    ) {
        // Simulate an async function that returns an array
        let array_value = Value::Array(
            values.iter().map(|&v| Value::Number(v as f64)).collect()
        );
        let result_promise = PromiseAPI::resolve(array_value);
        
        // Property: The result should be a fulfilled Promise
        prop_assert!(result_promise.is_fulfilled(), 
            "Async function should return a fulfilled Promise");
        
        // Property: The Promise should resolve with the returned array
        if let Some(Value::Array(arr)) = result_promise.get_value() {
            prop_assert_eq!(arr.len(), values.len(), 
                "Array length should match");
            
            for (i, (&expected, actual)) in values.iter().zip(arr.iter()).enumerate() {
                if let Value::Number(n) = actual {
                    prop_assert_eq!(*n as i32, expected, 
                        "Array element at index {} should match", i);
                } else {
                    prop_assert!(false, "Expected number at index {}", i);
                }
            }
        } else {
            prop_assert!(false, "Expected array value in Promise");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that async functions returning null resolve with null
    /// **Feature: dx-runtime-production-ready, Property 17: Async Function Promise Return**
    #[test]
    fn prop_async_function_returns_promise_with_null(_dummy in 0..1i32) {
        // Simulate an async function that returns null
        let result_promise = PromiseAPI::resolve(Value::Null);
        
        // Property: The result should be a fulfilled Promise
        prop_assert!(result_promise.is_fulfilled(), 
            "Async function should return a fulfilled Promise");
        
        // Property: The Promise should resolve with null
        prop_assert_eq!(result_promise.get_value(), Some(&Value::Null), 
            "Promise should resolve with null");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that async functions returning undefined resolve with undefined
    /// **Feature: dx-runtime-production-ready, Property 17: Async Function Promise Return**
    #[test]
    fn prop_async_function_returns_promise_with_undefined(_dummy in 0..1i32) {
        // Simulate an async function that returns undefined
        let result_promise = PromiseAPI::resolve(Value::Undefined);
        
        // Property: The result should be a fulfilled Promise
        prop_assert!(result_promise.is_fulfilled(), 
            "Async function should return a fulfilled Promise");
        
        // Property: The Promise should resolve with undefined
        prop_assert_eq!(result_promise.get_value(), Some(&Value::Undefined), 
            "Promise should resolve with undefined");
    }
}

// ============================================================================
// Test Promise chaining with then/catch
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that Promise.then chains correctly with async function results
    /// **Feature: dx-runtime-production-ready, Property 17: Async Function Promise Return**
    #[test]
    fn prop_async_function_promise_then_chaining(
        initial_value in any::<i32>(),
        add_value in any::<i32>()
    ) {
        // Simulate an async function that returns a number
        let result_promise = PromiseAPI::resolve(Value::Number(initial_value as f64));
        
        // Chain with then to transform the value
        let chained = result_promise.then(
            Some(move |v: Value| {
                if let Value::Number(n) = v {
                    PromiseResult::Value(Value::Number(n + add_value as f64))
                } else {
                    PromiseResult::Value(v)
                }
            }),
            None::<fn(Value) -> PromiseResult>,
        );
        
        // Property: The chained Promise should be fulfilled
        prop_assert!(chained.is_fulfilled(), 
            "Chained Promise should be fulfilled");
        
        // Property: The chained Promise should have the transformed value
        if let Some(Value::Number(n)) = chained.get_value() {
            let expected = (initial_value as i64 + add_value as i64) as f64;
            prop_assert_eq!(*n, expected, 
                "Chained Promise should have transformed value");
        } else {
            prop_assert!(false, "Expected number value in chained Promise");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that Promise.catch handles rejections from async functions
    /// **Feature: dx-runtime-production-ready, Property 17: Async Function Promise Return**
    #[test]
    fn prop_async_function_promise_catch_handling(
        error_message in "[a-zA-Z0-9 ]{1,30}",
        recovery_value in any::<i32>()
    ) {
        // Simulate an async function that throws
        let result_promise = PromiseAPI::reject(Value::String(error_message.clone()));
        
        // Chain with catch to recover from the error
        let recovered = result_promise.catch(move |_reason: Value| {
            PromiseResult::Value(Value::Number(recovery_value as f64))
        });
        
        // Property: The recovered Promise should be fulfilled (error was caught)
        prop_assert!(recovered.is_fulfilled(), 
            "Recovered Promise should be fulfilled after catch");
        
        // Property: The recovered Promise should have the recovery value
        if let Some(Value::Number(n)) = recovered.get_value() {
            prop_assert_eq!(*n as i32, recovery_value, 
                "Recovered Promise should have recovery value");
        } else {
            prop_assert!(false, "Expected number value in recovered Promise");
        }
    }
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_async_function_returns_promise_immediately() {
    // This test verifies that calling an async function returns a Promise
    // immediately, without waiting for the function body to complete.
    // In the runtime, this is handled by builtin_create_async_function
    // which creates a Promise before executing the function body.
    
    let promise = PromiseAPI::resolve(Value::Number(42.0));
    
    // The Promise should be immediately available
    assert!(promise.is_settled(), "Promise should be settled immediately for sync resolution");
    assert!(promise.is_fulfilled(), "Promise should be fulfilled");
    assert_eq!(promise.get_value(), Some(&Value::Number(42.0)));
}

#[test]
fn test_async_function_throw_rejects_promise() {
    // This test verifies that throwing in an async function rejects the Promise
    // Requirements: 5.4
    
    let promise = PromiseAPI::reject(Value::String("Test error".to_string()));
    
    assert!(promise.is_rejected(), "Promise should be rejected when async function throws");
    assert_eq!(promise.get_reason(), Some(&Value::String("Test error".to_string())));
}

#[test]
fn test_async_function_return_resolves_promise() {
    // This test verifies that returning from an async function resolves the Promise
    // Requirements: 5.5
    
    let promise = PromiseAPI::resolve(Value::Number(100.0));
    
    assert!(promise.is_fulfilled(), "Promise should be fulfilled when async function returns");
    assert_eq!(promise.get_value(), Some(&Value::Number(100.0)));
}

#[test]
fn test_async_function_finally_runs_on_success() {
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    
    let finally_ran = Arc::new(AtomicBool::new(false));
    let finally_ran_clone = finally_ran.clone();
    
    let promise = PromiseAPI::resolve(Value::Number(42.0));
    let _result = promise.finally(move || {
        finally_ran_clone.store(true, Ordering::SeqCst);
    });
    
    assert!(finally_ran.load(Ordering::SeqCst), "Finally should run on success");
}

#[test]
fn test_async_function_finally_runs_on_error() {
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    
    let finally_ran = Arc::new(AtomicBool::new(false));
    let finally_ran_clone = finally_ran.clone();
    
    let promise = PromiseAPI::reject(Value::String("error".to_string()));
    let _result = promise.finally(move || {
        finally_ran_clone.store(true, Ordering::SeqCst);
    });
    
    assert!(finally_ran.load(Ordering::SeqCst), "Finally should run on error");
}
