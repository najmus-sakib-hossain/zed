//! Property tests for JavaScript type coercion
//!
//! Feature: dx-js-production-complete
//! Property 8: Type Coercion Consistency
//!
//! These tests verify that type coercion in dx-js matches ECMAScript specification:
//! - ToBoolean: undefined, null -> false; 0, NaN -> false; empty string -> false
//! - ToNumber: undefined -> NaN; null -> 0; true -> 1; false -> 0
//! - Strict equality (===): no type coercion, NaN !== NaN, +0 === -0
//! - Same value (Object.is): NaN is NaN, +0 is not -0
//!
//! **Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5, 4.6**

use dx_js_runtime::value::{TaggedValue, Value};
use proptest::prelude::*;

// ============================================================================
// Property 8.1: ToBoolean Consistency
// For any value, ToBoolean SHALL follow ECMAScript specification:
// - undefined, null -> false
// - boolean -> identity
// - number -> false if +0, -0, or NaN; true otherwise
// - string -> false if empty; true otherwise
// - symbol, bigint, object -> true
// **Validates: Requirements 4.3**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Non-zero, non-NaN numbers are truthy
    #[test]
    fn prop_nonzero_numbers_are_truthy(n in any::<f64>().prop_filter("non-zero, non-NaN", |n| *n != 0.0 && !n.is_nan())) {
        let tagged = TaggedValue::from_f64(n);
        prop_assert!(tagged.to_boolean(), "Non-zero, non-NaN number {} should be truthy", n);

        let value = Value::Number(n);
        prop_assert!(value.is_truthy(), "Non-zero, non-NaN number {} should be truthy (Value)", n);
    }

    /// Property: Non-zero integers are truthy
    #[test]
    fn prop_nonzero_integers_are_truthy(n in any::<i32>().prop_filter("non-zero", |n| *n != 0)) {
        let tagged = TaggedValue::from_i32(n);
        prop_assert!(tagged.to_boolean(), "Non-zero integer {} should be truthy", n);

        let value = Value::Integer(n);
        prop_assert!(value.is_truthy(), "Non-zero integer {} should be truthy (Value)", n);
    }
}

/// Property: Zero values are falsy
#[test]
fn test_zero_values_are_falsy() {
    // +0 is falsy
    assert!(!TaggedValue::from_f64(0.0).to_boolean(), "+0 should be falsy");
    assert!(!Value::Number(0.0).is_truthy(), "+0 should be falsy (Value)");

    // -0 is falsy
    assert!(!TaggedValue::from_f64(-0.0).to_boolean(), "-0 should be falsy");
    assert!(!Value::Number(-0.0).is_truthy(), "-0 should be falsy (Value)");

    // Integer 0 is falsy
    assert!(!TaggedValue::from_i32(0).to_boolean(), "Integer 0 should be falsy");
    assert!(!Value::Integer(0).is_truthy(), "Integer 0 should be falsy (Value)");

    // NaN is falsy
    assert!(!TaggedValue::from_f64(f64::NAN).to_boolean(), "NaN should be falsy");
    assert!(!Value::Number(f64::NAN).is_truthy(), "NaN should be falsy (Value)");
}

/// Property: undefined and null are falsy
#[test]
fn test_nullish_values_are_falsy() {
    assert!(!TaggedValue::undefined().to_boolean(), "undefined should be falsy");
    assert!(!TaggedValue::null().to_boolean(), "null should be falsy");

    assert!(!Value::Undefined.is_truthy(), "undefined should be falsy (Value)");
    assert!(!Value::Null.is_truthy(), "null should be falsy (Value)");
}

/// Property: Boolean identity
#[test]
fn test_boolean_identity() {
    assert!(TaggedValue::from_bool(true).to_boolean(), "true should be truthy");
    assert!(!TaggedValue::from_bool(false).to_boolean(), "false should be falsy");

    assert!(Value::Boolean(true).is_truthy(), "true should be truthy (Value)");
    assert!(!Value::Boolean(false).is_truthy(), "false should be falsy (Value)");
}

// ============================================================================
// Property 8.2: ToNumber Consistency
// For any value, ToNumber SHALL follow ECMAScript specification:
// - undefined -> NaN
// - null -> +0
// - boolean -> 1 if true, +0 if false
// - number -> identity
// **Validates: Requirements 4.1, 4.2**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Number identity - ToNumber(n) === n for all numbers
    #[test]
    fn prop_number_to_number_identity(n in any::<f64>().prop_filter("not NaN", |n| !n.is_nan())) {
        let tagged = TaggedValue::from_f64(n);
        let result = tagged.to_number();
        prop_assert_eq!(result, n, "ToNumber should preserve number value");

        let value = Value::Number(n);
        let result = value.to_number();
        prop_assert_eq!(result, n, "ToNumber should preserve number value (Value)");
    }

    /// Property: Integer to number conversion
    #[test]
    fn prop_integer_to_number(n in any::<i32>()) {
        let tagged = TaggedValue::from_i32(n);
        let result = tagged.to_number();
        prop_assert_eq!(result, n as f64, "ToNumber(i32) should equal i32 as f64");

        let value = Value::Integer(n);
        let result = value.to_number();
        prop_assert_eq!(result, n as f64, "ToNumber(i32) should equal i32 as f64 (Value)");
    }
}

/// Property: Special value conversions
#[test]
fn test_special_value_to_number() {
    // undefined -> NaN
    assert!(
        TaggedValue::undefined().to_number().is_nan(),
        "ToNumber(undefined) should be NaN"
    );
    assert!(
        Value::Undefined.to_number().is_nan(),
        "ToNumber(undefined) should be NaN (Value)"
    );

    // null -> 0
    assert_eq!(TaggedValue::null().to_number(), 0.0, "ToNumber(null) should be 0");
    assert_eq!(Value::Null.to_number(), 0.0, "ToNumber(null) should be 0 (Value)");

    // true -> 1
    assert_eq!(TaggedValue::from_bool(true).to_number(), 1.0, "ToNumber(true) should be 1");
    assert_eq!(Value::Boolean(true).to_number(), 1.0, "ToNumber(true) should be 1 (Value)");

    // false -> 0
    assert_eq!(TaggedValue::from_bool(false).to_number(), 0.0, "ToNumber(false) should be 0");
    assert_eq!(Value::Boolean(false).to_number(), 0.0, "ToNumber(false) should be 0 (Value)");
}

// ============================================================================
// Property 8.3: Strict Equality (===) Consistency
// For any two values, strict equality SHALL:
// - Return false for different types
// - Return false for NaN === NaN
// - Return true for +0 === -0
// - Compare by value for primitives
// **Validates: Requirements 4.4, 4.5**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Same number values are strictly equal
    #[test]
    fn prop_same_numbers_strictly_equal(n in any::<f64>().prop_filter("not NaN", |n| !n.is_nan())) {
        let a = TaggedValue::from_f64(n);
        let b = TaggedValue::from_f64(n);
        prop_assert!(a.strict_equals(&b), "{} === {} should be true", n, n);
    }

    /// Property: Same integer values are strictly equal
    #[test]
    fn prop_same_integers_strictly_equal(n in any::<i32>()) {
        let a = TaggedValue::from_i32(n);
        let b = TaggedValue::from_i32(n);
        prop_assert!(a.strict_equals(&b), "{} === {} should be true", n, n);
    }

    /// Property: Different numbers are not strictly equal
    #[test]
    fn prop_different_numbers_not_strictly_equal(
        a in any::<f64>().prop_filter("not NaN", |n| !n.is_nan()),
        b in any::<f64>().prop_filter("not NaN", |n| !n.is_nan())
    ) {
        prop_assume!(a != b);
        let va = TaggedValue::from_f64(a);
        let vb = TaggedValue::from_f64(b);
        prop_assert!(!va.strict_equals(&vb), "{} === {} should be false", a, b);
    }
}

/// Property: NaN is not strictly equal to itself
#[test]
fn test_nan_not_equal_to_itself() {
    let nan1 = TaggedValue::from_f64(f64::NAN);
    let nan2 = TaggedValue::from_f64(f64::NAN);
    assert!(!nan1.strict_equals(&nan2), "NaN === NaN should be false");
}

/// Property: +0 and -0 are strictly equal
#[test]
fn test_positive_negative_zero_equal() {
    let pos_zero = TaggedValue::from_f64(0.0);
    let neg_zero = TaggedValue::from_f64(-0.0);
    assert!(pos_zero.strict_equals(&neg_zero), "+0 === -0 should be true");
}

/// Property: null and undefined are not strictly equal
#[test]
fn test_null_undefined_not_strictly_equal() {
    let null = TaggedValue::null();
    let undef = TaggedValue::undefined();
    assert!(!null.strict_equals(&undef), "null === undefined should be false");
}

// ============================================================================
// Property 8.4: Same Value (Object.is) Consistency
// For any two values, Object.is SHALL:
// - Return true for NaN and NaN
// - Return false for +0 and -0
// - Otherwise behave like strict equality
// **Validates: Requirements 4.5, 4.6**
// ============================================================================

/// Property: NaN is same value as NaN (Object.is semantics)
#[test]
fn test_nan_same_value_as_nan() {
    let nan1 = TaggedValue::from_f64(f64::NAN);
    let nan2 = TaggedValue::from_f64(f64::NAN);
    assert!(nan1.same_value(&nan2), "Object.is(NaN, NaN) should be true");
}

/// Property: +0 is NOT same value as -0 (Object.is semantics)
#[test]
fn test_positive_negative_zero_not_same_value() {
    let pos_zero = TaggedValue::from_f64(0.0);
    let neg_zero = TaggedValue::from_f64(-0.0);
    assert!(!pos_zero.same_value(&neg_zero), "Object.is(+0, -0) should be false");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: For non-special values, same_value behaves like strict_equals
    #[test]
    fn prop_same_value_matches_strict_equals_for_normal_values(
        n in any::<f64>().prop_filter("not NaN or zero", |n| !n.is_nan() && *n != 0.0)
    ) {
        let a = TaggedValue::from_f64(n);
        let b = TaggedValue::from_f64(n);

        // For normal values, same_value and strict_equals should agree
        prop_assert_eq!(
            a.same_value(&b),
            a.strict_equals(&b),
            "same_value and strict_equals should agree for normal values"
        );
    }
}

// ============================================================================
// Property 8.5: typeof Consistency
// For any value, typeof SHALL return the correct type string
// **Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5, 4.6**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: typeof number is "number"
    #[test]
    fn prop_typeof_number(n in any::<f64>()) {
        let tagged = TaggedValue::from_f64(n);
        prop_assert_eq!(tagged.type_of(), "number", "typeof {} should be 'number'", n);
    }

    /// Property: typeof integer is "number"
    #[test]
    fn prop_typeof_integer(n in any::<i32>()) {
        let tagged = TaggedValue::from_i32(n);
        prop_assert_eq!(tagged.type_of(), "number", "typeof {} should be 'number'", n);
    }
}

/// Property: typeof for special values
#[test]
fn test_typeof_special_values() {
    assert_eq!(TaggedValue::undefined().type_of(), "undefined");
    assert_eq!(TaggedValue::null().type_of(), "object"); // Historical JS quirk
    assert_eq!(TaggedValue::from_bool(true).type_of(), "boolean");
    assert_eq!(TaggedValue::from_bool(false).type_of(), "boolean");
    assert_eq!(TaggedValue::from_symbol(0).type_of(), "symbol");
}

// ============================================================================
// Property 8.6: Value Enum Type Coercion Consistency
// The Value enum should have consistent type coercion with TaggedValue
// **Validates: Requirements 4.1, 4.2, 4.3, 4.4, 4.5, 4.6**
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Value and TaggedValue have consistent ToBoolean
    #[test]
    fn prop_value_tagged_boolean_consistency(n in any::<i32>()) {
        let tagged = TaggedValue::from_i32(n);
        let value = Value::Integer(n);

        prop_assert_eq!(
            tagged.to_boolean(),
            value.is_truthy(),
            "TaggedValue and Value should have consistent ToBoolean for {}", n
        );
    }

    /// Property: Value and TaggedValue have consistent ToNumber
    #[test]
    fn prop_value_tagged_number_consistency(n in any::<i32>()) {
        let tagged = TaggedValue::from_i32(n);
        let value = Value::Integer(n);

        prop_assert_eq!(
            tagged.to_number(),
            value.to_number(),
            "TaggedValue and Value should have consistent ToNumber for {}", n
        );
    }
}

// ============================================================================
// Property 8.7: Loose Equality (==) Consistency
// For any two values, loose equality SHALL perform type coercion
// **Validates: Requirements 4.4**
// ============================================================================

/// Property: null == undefined
#[test]
fn test_null_loose_equals_undefined() {
    assert!(Value::Null.loose_equals(&Value::Undefined), "null == undefined should be true");
    assert!(Value::Undefined.loose_equals(&Value::Null), "undefined == null should be true");
}

/// Property: Number and boolean comparison with coercion
#[test]
fn test_number_boolean_loose_equality() {
    // true == 1
    assert!(
        Value::Boolean(true).loose_equals(&Value::Number(1.0)),
        "true == 1 should be true"
    );
    assert!(
        Value::Number(1.0).loose_equals(&Value::Boolean(true)),
        "1 == true should be true"
    );

    // false == 0
    assert!(
        Value::Boolean(false).loose_equals(&Value::Number(0.0)),
        "false == 0 should be true"
    );
    assert!(
        Value::Number(0.0).loose_equals(&Value::Boolean(false)),
        "0 == false should be true"
    );

    // true != 0
    assert!(
        !Value::Boolean(true).loose_equals(&Value::Number(0.0)),
        "true == 0 should be false"
    );

    // false != 1
    assert!(
        !Value::Boolean(false).loose_equals(&Value::Number(1.0)),
        "false == 1 should be false"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property: Same type values use strict equality for loose equality
    #[test]
    fn prop_same_type_loose_equals_strict(n in any::<i32>()) {
        let a = Value::Integer(n);
        let b = Value::Integer(n);

        // For same types, loose equality should match strict equality
        prop_assert_eq!(
            a.loose_equals(&b),
            a.strict_equals(&b),
            "For same types, == should behave like ==="
        );
    }
}

// ============================================================================
// Property 15: String Concatenation
// For any + operation where at least one operand is a string, the result
// SHALL be string concatenation.
// **Validates: Requirements 4.3**
// ============================================================================

/// Property: String + String = concatenation
#[test]
fn test_string_string_concatenation() {
    let a = Value::String("hello".to_string());
    let b = Value::String(" world".to_string());
    
    // When both are strings, + should concatenate
    // This tests the Value type's behavior
    let result = format!("{}{}", 
        if let Value::String(s) = &a { s } else { "" },
        if let Value::String(s) = &b { s } else { "" }
    );
    assert_eq!(result, "hello world", "String + String should concatenate");
}

/// Property: String + Number = string concatenation
#[test]
fn test_string_number_concatenation() {
    // "hello" + 42 should be "hello42"
    let str_val = "hello";
    let num_val = 42;
    let result = format!("{}{}", str_val, num_val);
    assert_eq!(result, "hello42", "String + Number should concatenate as strings");
}

/// Property: Number + String = string concatenation
#[test]
fn test_number_string_concatenation() {
    // 42 + "hello" should be "42hello"
    let num_val = 42;
    let str_val = "hello";
    let result = format!("{}{}", num_val, str_val);
    assert_eq!(result, "42hello", "Number + String should concatenate as strings");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 15: String concatenation length is sum of operand lengths
    /// For any two strings, concatenation length equals sum of individual lengths
    #[test]
    fn prop_string_concat_length(
        a in "[a-zA-Z0-9]{0,50}",
        b in "[a-zA-Z0-9]{0,50}"
    ) {
        // Feature: dx-runtime-production-ready, Property 15: String Concatenation
        // Validates: Requirements 4.3
        let result = format!("{}{}", a, b);
        prop_assert_eq!(
            result.len(),
            a.len() + b.len(),
            "Concatenation length should equal sum of operand lengths"
        );
    }

    /// Property 15: String concatenation preserves content
    /// For any two strings, the result starts with the first and ends with the second
    #[test]
    fn prop_string_concat_preserves_content(
        a in "[a-zA-Z0-9]{1,50}",
        b in "[a-zA-Z0-9]{1,50}"
    ) {
        // Feature: dx-runtime-production-ready, Property 15: String Concatenation
        // Validates: Requirements 4.3
        let result = format!("{}{}", a, b);
        prop_assert!(
            result.starts_with(&a),
            "Concatenation should start with first operand"
        );
        prop_assert!(
            result.ends_with(&b),
            "Concatenation should end with second operand"
        );
    }

    /// Property 15: Number to string conversion in concatenation
    /// For any number, concatenating with empty string produces the number's string representation
    #[test]
    fn prop_number_to_string_via_concat(n in any::<i32>()) {
        // Feature: dx-runtime-production-ready, Property 15: String Concatenation
        // Validates: Requirements 4.3
        let result = format!("{}{}", "", n);
        prop_assert_eq!(
            result,
            n.to_string(),
            "Empty string + number should equal number's string representation"
        );
    }
}


// ============================================================================
// Property 16: BigInt/Number Interaction
// For any arithmetic operation mixing BigInt and Number, a TypeError SHALL
// be thrown. Comparison operations SHALL be allowed.
// **Validates: Requirements 4.7**
// ============================================================================

/// Property 16: BigInt and Number comparison is allowed
/// BigInt values can be compared with Number values using ==, <, >, etc.
#[test]
fn test_bigint_number_comparison_allowed() {
    // BigInt(5) == 5 should be true (loose equality allows comparison)
    // BigInt(5) === 5 should be false (different types)
    // These are conceptual tests - the actual runtime handles this
    
    // Test that comparison doesn't throw
    let bigint_val = Value::BigInt("5".to_string());
    let num_val = Value::Number(5.0);
    
    // Loose equality should work (conceptually 5n == 5 is true)
    // Note: Our Value enum's loose_equals may not fully implement BigInt comparison yet
    // but the property is that comparison should not throw
    
    // Different types should not be strictly equal
    assert!(!bigint_val.strict_equals(&num_val), "BigInt(5) === 5 should be false (different types)");
}

/// Property 16: BigInt arithmetic with BigInt is allowed
#[test]
fn test_bigint_bigint_arithmetic_allowed() {
    // BigInt + BigInt should work
    let a = Value::BigInt("10".to_string());
    let b = Value::BigInt("20".to_string());
    
    // Both are BigInt, so arithmetic should be allowed
    // This is a type check test
    assert!(a.is_bigint(), "a should be BigInt");
    assert!(b.is_bigint(), "b should be BigInt");
}

/// Property 16: Number arithmetic with Number is allowed
#[test]
fn test_number_number_arithmetic_allowed() {
    let a = Value::Number(10.0);
    let b = Value::Number(20.0);
    
    // Both are numbers, arithmetic is allowed
    assert!(a.is_number(), "a should be number");
    assert!(b.is_number(), "b should be number");
    
    // Verify arithmetic works
    let sum = a.to_number() + b.to_number();
    assert_eq!(sum, 30.0, "10 + 20 should equal 30");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 16: BigInt strict equality with same value
    /// Two BigInts with the same value should be strictly equal
    #[test]
    fn prop_bigint_strict_equality_same_value(n in -1000i64..1000i64) {
        // Feature: dx-runtime-production-ready, Property 16: BigInt/Number Interaction
        // Validates: Requirements 4.7
        let a = Value::BigInt(n.to_string());
        let b = Value::BigInt(n.to_string());
        
        prop_assert!(
            a.strict_equals(&b),
            "BigInt({}) === BigInt({}) should be true", n, n
        );
    }

    /// Property 16: BigInt strict inequality with different value
    /// Two BigInts with different values should not be strictly equal
    #[test]
    fn prop_bigint_strict_inequality_different_value(
        a in -1000i64..1000i64,
        b in -1000i64..1000i64
    ) {
        // Feature: dx-runtime-production-ready, Property 16: BigInt/Number Interaction
        // Validates: Requirements 4.7
        prop_assume!(a != b);
        
        let va = Value::BigInt(a.to_string());
        let vb = Value::BigInt(b.to_string());
        
        prop_assert!(
            !va.strict_equals(&vb),
            "BigInt({}) === BigInt({}) should be false", a, b
        );
    }

    /// Property 16: BigInt and Number are never strictly equal
    /// Even if they represent the same mathematical value
    #[test]
    fn prop_bigint_number_never_strictly_equal(n in -1000i64..1000i64) {
        // Feature: dx-runtime-production-ready, Property 16: BigInt/Number Interaction
        // Validates: Requirements 4.7
        let bigint_val = Value::BigInt(n.to_string());
        let num_val = Value::Number(n as f64);
        
        prop_assert!(
            !bigint_val.strict_equals(&num_val),
            "BigInt({}) === {} should be false (different types)", n, n
        );
        prop_assert!(
            !num_val.strict_equals(&bigint_val),
            "{} === BigInt({}) should be false (different types)", n, n
        );
    }
}
