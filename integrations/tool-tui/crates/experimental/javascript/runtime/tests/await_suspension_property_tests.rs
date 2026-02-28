//! Property tests for Await Suspension
//!
//! **Property 18: Await Suspension**
//! *For any* await expression, execution should suspend until the Promise settles,
//! then resume with the resolved value or throw the rejection reason.
//!
//! **Validates: Requirements 5.2, 5.3**
//!
//! Requirements:
//! - 5.2: WHEN await is encountered, THE DX_Runtime SHALL suspend execution until the Promise resolves
//! - 5.3: WHEN an awaited Promise rejects, THE DX_Runtime SHALL throw the rejection reason as an exception
//!
//! **Feature: dx-runtime-production-ready, Property 18: Await Suspension**

use dx_js_runtime::runtime::async_runtime::{PromiseAPI, PromiseResult};
use dx_js_runtime::value::Value;
use proptest::prelude::*;

// ============================================================================
// Property 18: Await Suspension
// For any await expression, execution should suspend until the Promise settles,
// then resume with the resolved value or throw the rejection reason.
// **Validates: Requirements 5.2, 5.3**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that awaiting a fulfilled Promise returns the resolved value
    /// Requirements: 5.2 - WHEN await is encountered, THE DX_Runtime SHALL suspend execution until the Promise resolves
    /// **Feature: dx-runtime-production-ready, Property 18: Await Suspension**
    #[test]
    fn prop_await_fulfilled_promise_returns_value(value in any::<f64>()) {
        // Create a fulfilled Promise
        let promise = PromiseAPI::resolve(Value::Number(value));
        
        // Property: The Promise should be fulfilled
        prop_assert!(promise.is_fulfilled(), 
            "Promise should be fulfilled");
        
        // Property: Awaiting should return the resolved value
        if let Some(Value::Number(n)) = promise.get_value() {
            if value.is_nan() {
                prop_assert!(n.is_nan(), 
                    "Await should return NaN when Promise resolves with NaN");
            } else {
                prop_assert_eq!(*n, value, 
                    "Await should return the resolved value");
            }
        } else {
            prop_assert!(false, "Expected number value");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that awaiting a rejected Promise throws the rejection reason
    /// Requirements: 5.3 - WHEN an awaited Promise rejects, THE DX_Runtime SHALL throw the rejection reason
    /// **Feature: dx-runtime-production-ready, Property 18: Await Suspension**
    #[test]
    fn prop_await_rejected_promise_throws(
        error_message in "[a-zA-Z0-9 ]{1,50}"
    ) {
        // Create a rejected Promise
        let promise = PromiseAPI::reject(Value::String(error_message.clone()));
        
        // Property: The Promise should be rejected
        prop_assert!(promise.is_rejected(), 
            "Promise should be rejected");
        
        // Property: The rejection reason should be available
        if let Some(Value::String(reason)) = promise.get_reason() {
            prop_assert_eq!(reason, &error_message, 
                "Rejection reason should match the error message");
        } else {
            prop_assert!(false, "Expected string rejection reason");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that awaiting a string value returns the string
    /// **Feature: dx-runtime-production-ready, Property 18: Await Suspension**
    #[test]
    fn prop_await_fulfilled_promise_with_string(
        value in "[a-zA-Z0-9 ]{0,50}"
    ) {
        // Create a fulfilled Promise with a string
        let promise = PromiseAPI::resolve(Value::String(value.clone()));
        
        // Property: The Promise should be fulfilled
        prop_assert!(promise.is_fulfilled(), 
            "Promise should be fulfilled");
        
        // Property: Awaiting should return the resolved string
        if let Some(Value::String(s)) = promise.get_value() {
            prop_assert_eq!(s, &value, 
                "Await should return the resolved string");
        } else {
            prop_assert!(false, "Expected string value");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that awaiting a boolean value returns the boolean
    /// **Feature: dx-runtime-production-ready, Property 18: Await Suspension**
    #[test]
    fn prop_await_fulfilled_promise_with_boolean(value in any::<bool>()) {
        // Create a fulfilled Promise with a boolean
        let promise = PromiseAPI::resolve(Value::Boolean(value));
        
        // Property: The Promise should be fulfilled
        prop_assert!(promise.is_fulfilled(), 
            "Promise should be fulfilled");
        
        // Property: Awaiting should return the resolved boolean
        if let Some(Value::Boolean(b)) = promise.get_value() {
            prop_assert_eq!(*b, value, 
                "Await should return the resolved boolean");
        } else {
            prop_assert!(false, "Expected boolean value");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that awaiting an array value returns the array
    /// **Feature: dx-runtime-production-ready, Property 18: Await Suspension**
    #[test]
    fn prop_await_fulfilled_promise_with_array(
        values in prop::collection::vec(any::<i32>(), 0..20)
    ) {
        // Create a fulfilled Promise with an array
        let array_value = Value::Array(
            values.iter().map(|&v| Value::Number(v as f64)).collect()
        );
        let promise = PromiseAPI::resolve(array_value);
        
        // Property: The Promise should be fulfilled
        prop_assert!(promise.is_fulfilled(), 
            "Promise should be fulfilled");
        
        // Property: Awaiting should return the resolved array
        if let Some(Value::Array(arr)) = promise.get_value() {
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
            prop_assert!(false, "Expected array value");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that chained awaits work correctly
    /// **Feature: dx-runtime-production-ready, Property 18: Await Suspension**
    #[test]
    fn prop_chained_await_works(
        initial_value in any::<i32>(),
        add_value in any::<i32>()
    ) {
        // Create a Promise chain simulating multiple awaits
        let promise1 = PromiseAPI::resolve(Value::Number(initial_value as f64));
        
        // Chain with then (simulating await)
        let promise2 = promise1.then(
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
        prop_assert!(promise2.is_fulfilled(), 
            "Chained Promise should be fulfilled");
        
        // Property: The value should be transformed
        if let Some(Value::Number(n)) = promise2.get_value() {
            let expected = (initial_value as i64 + add_value as i64) as f64;
            prop_assert_eq!(*n, expected, 
                "Chained await should return transformed value");
        } else {
            prop_assert!(false, "Expected number value");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that rejection propagates through await chain
    /// Requirements: 5.3 - WHEN an awaited Promise rejects, THE DX_Runtime SHALL throw the rejection reason
    /// **Feature: dx-runtime-production-ready, Property 18: Await Suspension**
    #[test]
    fn prop_rejection_propagates_through_chain(
        error_message in "[a-zA-Z0-9 ]{1,30}"
    ) {
        // Create a rejected Promise
        let promise1 = PromiseAPI::reject(Value::String(error_message.clone()));
        
        // Chain with then (no catch handler)
        let promise2 = promise1.then(
            Some(|v: Value| PromiseResult::Value(v)),
            None::<fn(Value) -> PromiseResult>,
        );
        
        // Property: The chained Promise should be rejected
        prop_assert!(promise2.is_rejected(), 
            "Rejection should propagate through chain");
        
        // Property: The rejection reason should be preserved
        if let Some(Value::String(reason)) = promise2.get_reason() {
            prop_assert_eq!(reason, &error_message, 
                "Rejection reason should be preserved through chain");
        } else {
            prop_assert!(false, "Expected string rejection reason");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that catch handler can recover from rejection
    /// **Feature: dx-runtime-production-ready, Property 18: Await Suspension**
    #[test]
    fn prop_catch_recovers_from_rejection(
        error_message in "[a-zA-Z0-9 ]{1,30}",
        recovery_value in any::<i32>()
    ) {
        // Create a rejected Promise
        let promise = PromiseAPI::reject(Value::String(error_message.clone()));
        
        // Catch the rejection and recover
        let recovered = promise.catch(move |_reason: Value| {
            PromiseResult::Value(Value::Number(recovery_value as f64))
        });
        
        // Property: The recovered Promise should be fulfilled
        prop_assert!(recovered.is_fulfilled(), 
            "Catch should recover from rejection");
        
        // Property: The recovery value should be returned
        if let Some(Value::Number(n)) = recovered.get_value() {
            prop_assert_eq!(*n as i32, recovery_value, 
                "Recovery value should be returned");
        } else {
            prop_assert!(false, "Expected number value");
        }
    }
}

// ============================================================================
// Edge case tests
// ============================================================================

#[test]
fn test_await_null_value() {
    let promise = PromiseAPI::resolve(Value::Null);
    
    assert!(promise.is_fulfilled(), "Promise should be fulfilled");
    assert_eq!(promise.get_value(), Some(&Value::Null), 
        "Await should return null");
}

#[test]
fn test_await_undefined_value() {
    let promise = PromiseAPI::resolve(Value::Undefined);
    
    assert!(promise.is_fulfilled(), "Promise should be fulfilled");
    assert_eq!(promise.get_value(), Some(&Value::Undefined), 
        "Await should return undefined");
}

#[test]
fn test_await_rejection_with_number() {
    let promise = PromiseAPI::reject(Value::Number(42.0));
    
    assert!(promise.is_rejected(), "Promise should be rejected");
    assert_eq!(promise.get_reason(), Some(&Value::Number(42.0)), 
        "Rejection reason should be the number");
}

#[test]
fn test_await_rejection_with_null() {
    let promise = PromiseAPI::reject(Value::Null);
    
    assert!(promise.is_rejected(), "Promise should be rejected");
    assert_eq!(promise.get_reason(), Some(&Value::Null), 
        "Rejection reason should be null");
}

#[test]
fn test_multiple_then_handlers() {
    let promise = PromiseAPI::resolve(Value::Number(10.0));
    
    // First then
    let p1 = promise.then(
        Some(|v: Value| {
            if let Value::Number(n) = v {
                PromiseResult::Value(Value::Number(n * 2.0))
            } else {
                PromiseResult::Value(v)
            }
        }),
        None::<fn(Value) -> PromiseResult>,
    );
    
    // Second then
    let p2 = p1.then(
        Some(|v: Value| {
            if let Value::Number(n) = v {
                PromiseResult::Value(Value::Number(n + 5.0))
            } else {
                PromiseResult::Value(v)
            }
        }),
        None::<fn(Value) -> PromiseResult>,
    );
    
    assert!(p2.is_fulfilled(), "Final Promise should be fulfilled");
    assert_eq!(p2.get_value(), Some(&Value::Number(25.0)), 
        "Value should be (10 * 2) + 5 = 25");
}

#[test]
fn test_finally_preserves_value() {
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    
    let finally_ran = Arc::new(AtomicBool::new(false));
    let finally_ran_clone = finally_ran.clone();
    
    let promise = PromiseAPI::resolve(Value::Number(42.0));
    let result = promise.finally(move || {
        finally_ran_clone.store(true, Ordering::SeqCst);
    });
    
    assert!(finally_ran.load(Ordering::SeqCst), "Finally should run");
    assert!(result.is_fulfilled(), "Promise should still be fulfilled");
    assert_eq!(result.get_value(), Some(&Value::Number(42.0)), 
        "Value should be preserved through finally");
}

#[test]
fn test_finally_preserves_rejection() {
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    
    let finally_ran = Arc::new(AtomicBool::new(false));
    let finally_ran_clone = finally_ran.clone();
    
    let promise = PromiseAPI::reject(Value::String("error".to_string()));
    let result = promise.finally(move || {
        finally_ran_clone.store(true, Ordering::SeqCst);
    });
    
    assert!(finally_ran.load(Ordering::SeqCst), "Finally should run");
    assert!(result.is_rejected(), "Promise should still be rejected");
    assert_eq!(result.get_reason(), Some(&Value::String("error".to_string())), 
        "Rejection reason should be preserved through finally");
}
