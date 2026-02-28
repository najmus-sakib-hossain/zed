//! Property tests for Promise.all/race Semantics
//!
//! **Property 19: Promise.all/race Semantics**
//! *For any* Promise.all call, the result should be an array of all resolved values.
//! *For any* Promise.race call, the result should be the first settled value.
//!
//! **Validates: Requirements 5.6, 5.7**
//!
//! Requirements:
//! - 5.6: WHEN Promise.all is called, THE DX_Runtime SHALL wait for all promises and return an array of results
//! - 5.7: WHEN Promise.race is called, THE DX_Runtime SHALL resolve/reject with the first settled promise
//!
//! **Feature: dx-runtime-production-ready, Property 19: Promise.all/race Semantics**

use dx_js_runtime::runtime::async_runtime::{Promise, PromiseAPI};
use dx_js_runtime::value::Value;
use proptest::prelude::*;

// ============================================================================
// Property 19: Promise.all/race Semantics
// For any Promise.all call, the result should be an array of all resolved values.
// For Promise.race, the result should be the first settled value.
// **Validates: Requirements 5.6, 5.7**
// ============================================================================

// ============================================================================
// Promise.all Tests - Requirements 5.6
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that Promise.all returns an array of all resolved values in order
    /// Requirements: 5.6 - WHEN Promise.all is called, THE DX_Runtime SHALL wait for all promises and return an array of results
    /// **Feature: dx-runtime-production-ready, Property 19: Promise.all/race Semantics**
    #[test]
    fn prop_promise_all_returns_array_of_results(
        values in prop::collection::vec(any::<i32>(), 1..20usize)
    ) {
        // Create promises that resolve with values
        let promises: Vec<Promise> = values
            .iter()
            .map(|&v| PromiseAPI::resolve(Value::Number(v as f64)))
            .collect();

        let result = PromiseAPI::all(promises);

        // Property: Result should be fulfilled
        prop_assert!(result.is_fulfilled(), 
            "Promise.all should be fulfilled when all promises resolve");

        // Property: Result should be an array with all values in order
        if let Some(Value::Array(arr)) = result.get_value() {
            prop_assert_eq!(arr.len(), values.len(), 
                "Result array length should match input length");

            for (i, (&expected, actual)) in values.iter().zip(arr.iter()).enumerate() {
                if let Value::Number(n) = actual {
                    prop_assert_eq!(*n as i32, expected, 
                        "Value at index {} should match: expected {}, got {}", 
                        i, expected, *n as i32);
                } else {
                    prop_assert!(false, "Expected number at index {}", i);
                }
            }
        } else {
            prop_assert!(false, "Expected array result from Promise.all");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that Promise.all with string values returns all strings
    /// **Feature: dx-runtime-production-ready, Property 19: Promise.all/race Semantics**
    #[test]
    fn prop_promise_all_with_strings(
        values in prop::collection::vec("[a-zA-Z0-9 ]{0,20}", 1..10usize)
    ) {
        // Create promises that resolve with strings
        let promises: Vec<Promise> = values
            .iter()
            .map(|v| PromiseAPI::resolve(Value::String(v.clone())))
            .collect();

        let result = PromiseAPI::all(promises);

        // Property: Result should be fulfilled
        prop_assert!(result.is_fulfilled(), 
            "Promise.all should be fulfilled when all promises resolve");

        // Property: Result should contain all strings in order
        if let Some(Value::Array(arr)) = result.get_value() {
            prop_assert_eq!(arr.len(), values.len(), 
                "Result array length should match input length");

            for (i, (expected, actual)) in values.iter().zip(arr.iter()).enumerate() {
                if let Value::String(s) = actual {
                    prop_assert_eq!(s, expected, 
                        "String at index {} should match", i);
                } else {
                    prop_assert!(false, "Expected string at index {}", i);
                }
            }
        } else {
            prop_assert!(false, "Expected array result from Promise.all");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that Promise.all rejects on first rejection
    /// Requirements: 5.6 - Promise.all should reject when any promise rejects
    /// **Feature: dx-runtime-production-ready, Property 19: Promise.all/race Semantics**
    #[test]
    fn prop_promise_all_rejects_on_first_rejection(
        values_before in prop::collection::vec(any::<i32>(), 0..10usize),
        error_msg in "[a-zA-Z0-9 ]{1,30}",
        values_after in prop::collection::vec(any::<i32>(), 0..10usize)
    ) {
        // Create promises: some fulfilled, one rejected, some more fulfilled
        let mut promises: Vec<Promise> = values_before
            .iter()
            .map(|&v| PromiseAPI::resolve(Value::Number(v as f64)))
            .collect();

        promises.push(PromiseAPI::reject(Value::String(error_msg.clone())));

        promises.extend(
            values_after
                .iter()
                .map(|&v| PromiseAPI::resolve(Value::Number(v as f64)))
        );

        let result = PromiseAPI::all(promises);

        // Property: Result should be rejected
        prop_assert!(result.is_rejected(), 
            "Promise.all should be rejected when any promise rejects");

        // Property: Rejection reason should be the first rejection
        if let Some(Value::String(reason)) = result.get_reason() {
            prop_assert_eq!(reason, &error_msg, 
                "Rejection reason should match the first rejection");
        } else {
            prop_assert!(false, "Expected string rejection reason");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that Promise.all preserves order regardless of value types
    /// **Feature: dx-runtime-production-ready, Property 19: Promise.all/race Semantics**
    #[test]
    fn prop_promise_all_preserves_order_mixed_types(
        num_values in prop::collection::vec(any::<i32>(), 1..5usize),
        str_values in prop::collection::vec("[a-z]{1,10}", 1..5usize),
        bool_values in prop::collection::vec(any::<bool>(), 1..5usize)
    ) {
        // Create mixed promises
        let mut promises: Vec<Promise> = Vec::new();
        let mut expected_types: Vec<&str> = Vec::new();

        for &v in &num_values {
            promises.push(PromiseAPI::resolve(Value::Number(v as f64)));
            expected_types.push("number");
        }
        for v in &str_values {
            promises.push(PromiseAPI::resolve(Value::String(v.clone())));
            expected_types.push("string");
        }
        for &v in &bool_values {
            promises.push(PromiseAPI::resolve(Value::Boolean(v)));
            expected_types.push("boolean");
        }

        let result = PromiseAPI::all(promises);

        // Property: Result should be fulfilled
        prop_assert!(result.is_fulfilled(), 
            "Promise.all should be fulfilled");

        // Property: Result should have correct length and types in order
        if let Some(Value::Array(arr)) = result.get_value() {
            prop_assert_eq!(arr.len(), expected_types.len(), 
                "Result array length should match");

            for (i, (expected_type, actual)) in expected_types.iter().zip(arr.iter()).enumerate() {
                let actual_type = match actual {
                    Value::Number(_) => "number",
                    Value::String(_) => "string",
                    Value::Boolean(_) => "boolean",
                    _ => "other",
                };
                prop_assert_eq!(actual_type, *expected_type, 
                    "Type at index {} should match", i);
            }
        } else {
            prop_assert!(false, "Expected array result");
        }
    }
}

// ============================================================================
// Promise.race Tests - Requirements 5.7
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that Promise.race resolves with the first fulfilled promise
    /// Requirements: 5.7 - WHEN Promise.race is called, THE DX_Runtime SHALL resolve/reject with the first settled promise
    /// **Feature: dx-runtime-production-ready, Property 19: Promise.all/race Semantics**
    #[test]
    fn prop_promise_race_resolves_with_first_fulfilled(
        first_value in any::<i32>(),
        other_values in prop::collection::vec(any::<i32>(), 0..10usize)
    ) {
        // Create promises with first one fulfilled
        let mut promises = vec![PromiseAPI::resolve(Value::Number(first_value as f64))];
        promises.extend(
            other_values
                .iter()
                .map(|&v| PromiseAPI::resolve(Value::Number(v as f64)))
        );

        let result = PromiseAPI::race(promises);

        // Property: Result should be fulfilled
        prop_assert!(result.is_fulfilled(), 
            "Promise.race should be fulfilled when first promise is fulfilled");

        // Property: Result should be the first settled value
        if let Some(Value::Number(n)) = result.get_value() {
            prop_assert_eq!(*n as i32, first_value, 
                "Promise.race should resolve with first settled value");
        } else {
            prop_assert!(false, "Expected number result");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that Promise.race rejects with the first rejected promise
    /// Requirements: 5.7 - Promise.race should reject with first rejection
    /// **Feature: dx-runtime-production-ready, Property 19: Promise.all/race Semantics**
    #[test]
    fn prop_promise_race_rejects_with_first_rejected(
        error_msg in "[a-zA-Z0-9 ]{1,30}",
        other_values in prop::collection::vec(any::<i32>(), 0..10usize)
    ) {
        // Create promises with first one rejected
        let mut promises = vec![PromiseAPI::reject(Value::String(error_msg.clone()))];
        promises.extend(
            other_values
                .iter()
                .map(|&v| PromiseAPI::resolve(Value::Number(v as f64)))
        );

        let result = PromiseAPI::race(promises);

        // Property: Result should be rejected
        prop_assert!(result.is_rejected(), 
            "Promise.race should be rejected when first promise is rejected");

        // Property: Rejection reason should be the first rejection
        if let Some(Value::String(reason)) = result.get_reason() {
            prop_assert_eq!(reason, &error_msg, 
                "Promise.race should reject with first rejection reason");
        } else {
            prop_assert!(false, "Expected string rejection reason");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that Promise.race with string values works correctly
    /// **Feature: dx-runtime-production-ready, Property 19: Promise.all/race Semantics**
    #[test]
    fn prop_promise_race_with_strings(
        first_value in "[a-zA-Z0-9 ]{1,30}",
        other_values in prop::collection::vec("[a-zA-Z0-9 ]{1,20}", 0..10usize)
    ) {
        // Create promises with first one fulfilled with a string
        let mut promises = vec![PromiseAPI::resolve(Value::String(first_value.clone()))];
        promises.extend(
            other_values
                .iter()
                .map(|v| PromiseAPI::resolve(Value::String(v.clone())))
        );

        let result = PromiseAPI::race(promises);

        // Property: Result should be fulfilled
        prop_assert!(result.is_fulfilled(), 
            "Promise.race should be fulfilled");

        // Property: Result should be the first string
        if let Some(Value::String(s)) = result.get_value() {
            prop_assert_eq!(s, &first_value, 
                "Promise.race should resolve with first string value");
        } else {
            prop_assert!(false, "Expected string result");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that Promise.race with boolean values works correctly
    /// **Feature: dx-runtime-production-ready, Property 19: Promise.all/race Semantics**
    #[test]
    fn prop_promise_race_with_booleans(
        first_value in any::<bool>(),
        other_values in prop::collection::vec(any::<bool>(), 0..10usize)
    ) {
        // Create promises with first one fulfilled with a boolean
        let mut promises = vec![PromiseAPI::resolve(Value::Boolean(first_value))];
        promises.extend(
            other_values
                .iter()
                .map(|&v| PromiseAPI::resolve(Value::Boolean(v)))
        );

        let result = PromiseAPI::race(promises);

        // Property: Result should be fulfilled
        prop_assert!(result.is_fulfilled(), 
            "Promise.race should be fulfilled");

        // Property: Result should be the first boolean
        if let Some(Value::Boolean(b)) = result.get_value() {
            prop_assert_eq!(*b, first_value, 
                "Promise.race should resolve with first boolean value");
        } else {
            prop_assert!(false, "Expected boolean result");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Test that Promise.race with mixed fulfilled and rejected returns first settled
    /// **Feature: dx-runtime-production-ready, Property 19: Promise.all/race Semantics**
    #[test]
    fn prop_promise_race_first_settled_wins(
        first_is_fulfilled in any::<bool>(),
        first_value in any::<i32>(),
        error_msg in "[a-zA-Z0-9 ]{1,20}",
        other_count in 0usize..5usize
    ) {
        // Create first promise based on first_is_fulfilled
        let first_promise = if first_is_fulfilled {
            PromiseAPI::resolve(Value::Number(first_value as f64))
        } else {
            PromiseAPI::reject(Value::String(error_msg.clone()))
        };

        let mut promises = vec![first_promise];
        
        // Add some other promises
        for i in 0..other_count {
            promises.push(PromiseAPI::resolve(Value::Number((i + 100) as f64)));
        }

        let result = PromiseAPI::race(promises);

        // Property: Result should match first promise's state
        if first_is_fulfilled {
            prop_assert!(result.is_fulfilled(), 
                "Promise.race should be fulfilled when first is fulfilled");
            if let Some(Value::Number(n)) = result.get_value() {
                prop_assert_eq!(*n as i32, first_value, 
                    "Should resolve with first value");
            } else {
                prop_assert!(false, "Expected number result");
            }
        } else {
            prop_assert!(result.is_rejected(), 
                "Promise.race should be rejected when first is rejected");
            if let Some(Value::String(reason)) = result.get_reason() {
                prop_assert_eq!(reason, &error_msg, 
                    "Should reject with first rejection reason");
            } else {
                prop_assert!(false, "Expected string rejection reason");
            }
        }
    }
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_promise_all_empty_array_returns_empty_array() {
    // Requirements: 5.6 - Empty Promise.all should resolve with empty array
    let result = PromiseAPI::all(vec![]);
    
    assert!(result.is_fulfilled(), "Promise.all([]) should be fulfilled");
    if let Some(Value::Array(arr)) = result.get_value() {
        assert!(arr.is_empty(), "Promise.all([]) should resolve with empty array");
    } else {
        panic!("Expected empty array result");
    }
}

#[test]
fn test_promise_race_empty_array_returns_pending() {
    // Requirements: 5.7 - Empty Promise.race should return pending promise
    let result = PromiseAPI::race(vec![]);
    
    // Empty race returns pending promise (never settles)
    assert!(!result.is_settled(), "Promise.race([]) should be pending");
}

#[test]
fn test_promise_all_single_fulfilled() {
    // Single fulfilled promise
    let promises = vec![PromiseAPI::resolve(Value::Number(42.0))];
    let result = PromiseAPI::all(promises);
    
    assert!(result.is_fulfilled(), "Promise.all with single fulfilled should be fulfilled");
    if let Some(Value::Array(arr)) = result.get_value() {
        assert_eq!(arr.len(), 1, "Should have one element");
        assert_eq!(arr[0], Value::Number(42.0), "Should contain the value");
    } else {
        panic!("Expected array result");
    }
}

#[test]
fn test_promise_all_single_rejected() {
    // Single rejected promise
    let promises = vec![PromiseAPI::reject(Value::String("error".to_string()))];
    let result = PromiseAPI::all(promises);
    
    assert!(result.is_rejected(), "Promise.all with single rejected should be rejected");
    assert_eq!(result.get_reason(), Some(&Value::String("error".to_string())));
}

#[test]
fn test_promise_race_single_fulfilled() {
    // Single fulfilled promise
    let promises = vec![PromiseAPI::resolve(Value::Number(42.0))];
    let result = PromiseAPI::race(promises);
    
    assert!(result.is_fulfilled(), "Promise.race with single fulfilled should be fulfilled");
    assert_eq!(result.get_value(), Some(&Value::Number(42.0)));
}

#[test]
fn test_promise_race_single_rejected() {
    // Single rejected promise
    let promises = vec![PromiseAPI::reject(Value::String("error".to_string()))];
    let result = PromiseAPI::race(promises);
    
    assert!(result.is_rejected(), "Promise.race with single rejected should be rejected");
    assert_eq!(result.get_reason(), Some(&Value::String("error".to_string())));
}

#[test]
fn test_promise_all_with_null_values() {
    // Promise.all with null values
    let promises = vec![
        PromiseAPI::resolve(Value::Null),
        PromiseAPI::resolve(Value::Number(1.0)),
        PromiseAPI::resolve(Value::Null),
    ];
    let result = PromiseAPI::all(promises);
    
    assert!(result.is_fulfilled(), "Promise.all with null values should be fulfilled");
    if let Some(Value::Array(arr)) = result.get_value() {
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0], Value::Null);
        assert_eq!(arr[1], Value::Number(1.0));
        assert_eq!(arr[2], Value::Null);
    } else {
        panic!("Expected array result");
    }
}

#[test]
fn test_promise_all_with_undefined_values() {
    // Promise.all with undefined values
    let promises = vec![
        PromiseAPI::resolve(Value::Undefined),
        PromiseAPI::resolve(Value::Number(1.0)),
        PromiseAPI::resolve(Value::Undefined),
    ];
    let result = PromiseAPI::all(promises);
    
    assert!(result.is_fulfilled(), "Promise.all with undefined values should be fulfilled");
    if let Some(Value::Array(arr)) = result.get_value() {
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0], Value::Undefined);
        assert_eq!(arr[1], Value::Number(1.0));
        assert_eq!(arr[2], Value::Undefined);
    } else {
        panic!("Expected array result");
    }
}

#[test]
fn test_promise_race_with_null_first() {
    // Promise.race with null as first value
    let promises = vec![
        PromiseAPI::resolve(Value::Null),
        PromiseAPI::resolve(Value::Number(1.0)),
    ];
    let result = PromiseAPI::race(promises);
    
    assert!(result.is_fulfilled(), "Promise.race should be fulfilled");
    assert_eq!(result.get_value(), Some(&Value::Null), "Should resolve with null");
}

#[test]
fn test_promise_race_with_undefined_first() {
    // Promise.race with undefined as first value
    let promises = vec![
        PromiseAPI::resolve(Value::Undefined),
        PromiseAPI::resolve(Value::Number(1.0)),
    ];
    let result = PromiseAPI::race(promises);
    
    assert!(result.is_fulfilled(), "Promise.race should be fulfilled");
    assert_eq!(result.get_value(), Some(&Value::Undefined), "Should resolve with undefined");
}

#[test]
fn test_promise_all_rejection_with_number() {
    // Promise.all rejection with number reason
    let promises = vec![
        PromiseAPI::resolve(Value::Number(1.0)),
        PromiseAPI::reject(Value::Number(42.0)),
        PromiseAPI::resolve(Value::Number(3.0)),
    ];
    let result = PromiseAPI::all(promises);
    
    assert!(result.is_rejected(), "Promise.all should be rejected");
    assert_eq!(result.get_reason(), Some(&Value::Number(42.0)));
}

#[test]
fn test_promise_race_rejection_with_number() {
    // Promise.race rejection with number reason
    let promises = vec![
        PromiseAPI::reject(Value::Number(42.0)),
        PromiseAPI::resolve(Value::Number(1.0)),
    ];
    let result = PromiseAPI::race(promises);
    
    assert!(result.is_rejected(), "Promise.race should be rejected");
    assert_eq!(result.get_reason(), Some(&Value::Number(42.0)));
}
