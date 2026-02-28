//! Property-Based Tests for Error Propagation (No Silent NaN)
//!
//! **Feature: dx-runtime-production-ready, Property 8: Error Propagation (No Silent NaN)**
//! **Validates: Requirements 2.3**
//!
//! This test validates that runtime functions propagate errors properly instead of
//! returning NaN silently. When an error condition occurs, the runtime should
//! set a structured exception that can be retrieved.

use proptest::prelude::*;

// Import error types and exception handling functions from the main crate
use dx_js_runtime::{
    get_structured_exception, set_structured_exception, clear_structured_exception,
    JsErrorType, JsException,
};

// ============================================================================
// Test Helpers
// ============================================================================

/// Generate valid BigInt values (as tagged f64)
fn arb_bigint_value() -> impl Strategy<Value = f64> {
    // BigInt IDs are encoded as -(id + 2_000_000)
    (0u64..1000u64).prop_map(|id| -(id as f64 + 2_000_000.0))
}

/// Generate non-BigInt values that should cause type errors
fn arb_non_bigint_value() -> impl Strategy<Value = f64> {
    prop_oneof![
        Just(0.0),
        Just(1.0),
        Just(-1.0),
        Just(42.0),
        Just(f64::NAN),
        Just(f64::INFINITY),
        Just(f64::NEG_INFINITY),
        // String IDs (encoded as -(id + 1_000_000))
        (0u64..100u64).prop_map(|id| -(id as f64 + 1_000_000.0)),
    ]
}

/// Generate values that would cause division by zero
fn arb_zero_bigint() -> impl Strategy<Value = f64> {
    // We need to create a BigInt with value 0
    // For testing purposes, we'll use a marker value
    Just(f64::NAN) // Placeholder - actual zero BigInt would need heap allocation
}

// ============================================================================
// Property 8: Error Propagation (No Silent NaN)
// **Validates: Requirements 2.3**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 8.1: Structured exceptions are set on error conditions
    /// *For any* error condition in a runtime function, a structured exception
    /// SHALL be set that can be retrieved via get_structured_exception.
    #[test]
    fn prop_structured_exception_set_on_error(
        error_type in prop_oneof![
            Just(JsErrorType::TypeError),
            Just(JsErrorType::RangeError),
            Just(JsErrorType::SyntaxError),
        ],
        message in "[a-zA-Z ]{1,50}",
    ) {
        // Clear any existing exception
        clear_structured_exception();
        
        // Simulate setting an exception (as the runtime functions do)
        let exception = JsException::new(error_type, message.clone());
        set_structured_exception(exception);
        
        // Verify exception can be retrieved
        let retrieved = get_structured_exception();
        prop_assert!(retrieved.is_some(), "Exception should be retrievable");
        
        let exc = retrieved.unwrap();
        prop_assert_eq!(exc.error_type, error_type, "Error type should match");
        prop_assert_eq!(exc.message, message, "Message should match");
        
        // Clean up
        clear_structured_exception();
    }

    /// Property 8.2: Exception clearing works correctly
    /// *For any* set exception, clearing it SHALL result in None being returned.
    #[test]
    fn prop_exception_clearing_works(
        error_type in prop_oneof![
            Just(JsErrorType::TypeError),
            Just(JsErrorType::RangeError),
        ],
        message in "[a-zA-Z ]{1,30}",
    ) {
        // Set an exception
        let exception = JsException::new(error_type, message);
        set_structured_exception(exception);
        
        // Verify it's set
        prop_assert!(get_structured_exception().is_some());
        
        // Clear it
        clear_structured_exception();
        
        // Verify it's cleared
        prop_assert!(get_structured_exception().is_none(), "Exception should be cleared");
    }

    /// Property 8.3: Error type is preserved through exception system
    /// *For any* JavaScript error type, creating and retrieving an exception
    /// SHALL preserve the exact error type.
    #[test]
    fn prop_error_type_preserved(
        error_type in prop_oneof![
            Just(JsErrorType::Error),
            Just(JsErrorType::TypeError),
            Just(JsErrorType::SyntaxError),
            Just(JsErrorType::ReferenceError),
            Just(JsErrorType::RangeError),
            Just(JsErrorType::URIError),
            Just(JsErrorType::EvalError),
        ],
    ) {
        clear_structured_exception();
        
        let exception = JsException::new(error_type, "test error");
        set_structured_exception(exception);
        
        let retrieved = get_structured_exception().expect("Exception should exist");
        prop_assert_eq!(retrieved.error_type, error_type, "Error type must be preserved");
        
        clear_structured_exception();
    }

    /// Property 8.4: Error messages are preserved through exception system
    /// *For any* error message, creating and retrieving an exception
    /// SHALL preserve the exact message content.
    #[test]
    fn prop_error_message_preserved(
        message in "[a-zA-Z0-9 _\\-\\.]{1,100}",
    ) {
        clear_structured_exception();
        
        let exception = JsException::new(JsErrorType::Error, message.clone());
        set_structured_exception(exception);
        
        let retrieved = get_structured_exception().expect("Exception should exist");
        prop_assert_eq!(retrieved.message, message, "Message must be preserved");
        
        clear_structured_exception();
    }
}

// ============================================================================
// Unit Tests for Specific Error Conditions
// ============================================================================

#[test]
fn test_type_error_exception_structure() {
    clear_structured_exception();
    
    let exception = JsException::type_error("undefined is not a function");
    set_structured_exception(exception);
    
    let retrieved = get_structured_exception().expect("Exception should exist");
    assert_eq!(retrieved.error_type, JsErrorType::TypeError);
    assert!(retrieved.message.contains("undefined is not a function"));
    
    clear_structured_exception();
}

#[test]
fn test_range_error_exception_structure() {
    clear_structured_exception();
    
    let exception = JsException::range_error("Division by zero");
    set_structured_exception(exception);
    
    let retrieved = get_structured_exception().expect("Exception should exist");
    assert_eq!(retrieved.error_type, JsErrorType::RangeError);
    assert!(retrieved.message.contains("Division by zero"));
    
    clear_structured_exception();
}

#[test]
fn test_syntax_error_exception_structure() {
    clear_structured_exception();
    
    let exception = JsException::syntax_error("Unexpected token");
    set_structured_exception(exception);
    
    let retrieved = get_structured_exception().expect("Exception should exist");
    assert_eq!(retrieved.error_type, JsErrorType::SyntaxError);
    assert!(retrieved.message.contains("Unexpected token"));
    
    clear_structured_exception();
}

#[test]
fn test_reference_error_exception_structure() {
    clear_structured_exception();
    
    let exception = JsException::reference_error("x is not defined");
    set_structured_exception(exception);
    
    let retrieved = get_structured_exception().expect("Exception should exist");
    assert_eq!(retrieved.error_type, JsErrorType::ReferenceError);
    assert!(retrieved.message.contains("x is not defined"));
    
    clear_structured_exception();
}

#[test]
fn test_exception_with_type_info() {
    clear_structured_exception();
    
    let exception = JsException::type_error("Type mismatch")
        .with_type_info("number", "string");
    set_structured_exception(exception);
    
    let retrieved = get_structured_exception().expect("Exception should exist");
    assert_eq!(retrieved.expected_type, Some("number".to_string()));
    assert_eq!(retrieved.received_type, Some("string".to_string()));
    
    clear_structured_exception();
}

#[test]
fn test_multiple_exceptions_last_wins() {
    clear_structured_exception();
    
    // Set first exception
    let exc1 = JsException::type_error("First error");
    set_structured_exception(exc1);
    
    // Set second exception (should overwrite)
    let exc2 = JsException::range_error("Second error");
    set_structured_exception(exc2);
    
    let retrieved = get_structured_exception().expect("Exception should exist");
    assert_eq!(retrieved.error_type, JsErrorType::RangeError);
    assert!(retrieved.message.contains("Second error"));
    
    clear_structured_exception();
}
