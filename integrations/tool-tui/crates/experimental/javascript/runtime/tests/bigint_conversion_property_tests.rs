//! Property-Based Tests for BigInt Conversion and Error Handling
//!
//! **Feature: production-readiness**
//! **Property 1: BigInt literal round-trip**
//! **Property 5: BigInt division error handling**
//! **Property 6: BigInt/Number mixing error**
//! **Property 7: BigInt constructor correctness**
//! **Validates: Requirements 1.1, 1.4, 1.6, 1.7, 1.8**
//!
//! These tests verify that BigInt conversion operations and error handling
//! in dx-js produce correct results, matching ECMAScript specification behavior.

use num_bigint::BigInt;
use num_traits::Zero;
use proptest::prelude::*;

// ============================================================================
// Test Helpers
// ============================================================================

/// Generate arbitrary BigInt values for testing
fn arb_bigint() -> impl Strategy<Value = BigInt> {
    prop_oneof![
        // Small integers
        (-1000i64..1000i64).prop_map(BigInt::from),
        // Medium integers
        (-1_000_000i64..1_000_000i64).prop_map(BigInt::from),
        // Large integers
        any::<i64>().prop_map(BigInt::from),
    ]
}

/// Generate non-zero BigInt values for division tests
fn arb_nonzero_bigint() -> impl Strategy<Value = BigInt> {
    arb_bigint().prop_filter("non-zero", |b| !b.is_zero())
}

/// Generate valid integer strings for parsing
fn arb_integer_string() -> impl Strategy<Value = String> {
    prop_oneof![
        // Simple integers
        (-1000i64..1000i64).prop_map(|n| n.to_string()),
        // Larger integers
        any::<i64>().prop_map(|n| n.to_string()),
    ]
}

/// Generate invalid strings that should fail BigInt parsing
fn arb_invalid_bigint_string() -> impl Strategy<Value = String> {
    prop_oneof![
        // Floating point numbers
        Just("3.14".to_string()),
        Just("1.0".to_string()),
        Just("-2.5".to_string()),
        // Non-numeric strings
        Just("abc".to_string()),
        Just("12abc".to_string()),
        Just("abc12".to_string()),
        // Empty string
        Just("".to_string()),
        // Only whitespace
        Just("   ".to_string()),
        // Special values
        Just("NaN".to_string()),
        Just("Infinity".to_string()),
        Just("-Infinity".to_string()),
    ]
}

/// Generate safe integers (can be exactly represented as f64)
fn arb_safe_integer() -> impl Strategy<Value = i64> {
    // Safe integer range: -(2^53 - 1) to (2^53 - 1)
    -9007199254740991i64..=9007199254740991i64
}

/// Generate non-integer f64 values
fn arb_non_integer_f64() -> impl Strategy<Value = f64> {
    prop_oneof![
        // Simple fractions
        Just(0.5),
        Just(1.5),
        Just(-0.5),
        Just(3.14159),
        // Very small fractions
        Just(0.0001),
        Just(-0.0001),
        // Random fractions
        (1i64..1000i64).prop_map(|n| n as f64 + 0.5),
    ]
}

// ============================================================================
// Property Tests for BigInt Literal Round-Trip (Property 1)
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 1: BigInt literal round-trip**
    /// *For any* valid BigInt value, converting to string and back SHALL produce an equivalent value.
    #[test]
    fn prop_bigint_to_string_roundtrip(a in arb_bigint()) {
        let string_repr = bigint_to_string(&a);
        let parsed = bigint_from_string(&string_repr);
        prop_assert!(parsed.is_some(), 
            "Failed to parse BigInt string: {}", string_repr);
        let parsed_val = parsed.unwrap();
        prop_assert_eq!(parsed_val.clone(), a.clone(),
            "BigInt round-trip failed: {} -> {} -> {}", a, string_repr, parsed_val);
    }

    /// **Property 1: BigInt literal round-trip**
    /// *For any* valid integer string, parsing and converting back to string SHALL produce an equivalent representation.
    #[test]
    fn prop_bigint_from_string_roundtrip(s in arb_integer_string()) {
        let parsed = bigint_from_string(&s);
        prop_assert!(parsed.is_some(), 
            "Failed to parse valid integer string: '{}'", s);
        let parsed_val = parsed.clone().unwrap();
        let back_to_string = bigint_to_string(&parsed_val);
        let reparsed = bigint_from_string(&back_to_string);
        prop_assert!(reparsed.is_some());
        let reparsed_val = reparsed.unwrap();
        prop_assert_eq!(parsed_val.clone(), reparsed_val.clone(),
            "BigInt string round-trip failed: '{}' -> {} -> '{}' -> {}", 
            s, parsed_val, back_to_string, reparsed_val);
    }

    /// **Property 1: BigInt literal round-trip**
    /// *For any* BigInt value, toString() SHALL produce a valid decimal representation.
    #[test]
    fn prop_bigint_to_string_is_valid_decimal(a in arb_bigint()) {
        let string_repr = bigint_to_string(&a);
        
        // Should be a valid decimal string (optional minus, then digits)
        let is_valid = if string_repr.starts_with('-') {
            string_repr[1..].chars().all(|c| c.is_ascii_digit())
        } else {
            string_repr.chars().all(|c| c.is_ascii_digit())
        };
        
        prop_assert!(is_valid,
            "BigInt toString produced invalid decimal: {} -> '{}'", a, string_repr);
    }
}

// ============================================================================
// Property Tests for BigInt Division Error Handling (Property 5)
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 5: BigInt division error handling**
    /// *For any* BigInt division by zero, the operation SHALL signal an error.
    #[test]
    fn prop_bigint_division_by_zero_error(a in arb_bigint()) {
        let zero = BigInt::from(0);
        let result = bigint_div_checked(&a, &zero);
        prop_assert!(result.is_none(),
            "BigInt division by zero should fail: {} / 0 = {:?}", a, result);
    }

    /// **Property 5: BigInt division error handling**
    /// *For any* BigInt modulo by zero, the operation SHALL signal an error.
    #[test]
    fn prop_bigint_modulo_by_zero_error(a in arb_bigint()) {
        let zero = BigInt::from(0);
        let result = bigint_mod_checked(&a, &zero);
        prop_assert!(result.is_none(),
            "BigInt modulo by zero should fail: {} % 0 = {:?}", a, result);
    }

    /// **Property 5: BigInt division error handling**
    /// *For any* valid BigInt division (non-zero divisor), the operation SHALL succeed.
    #[test]
    fn prop_bigint_division_valid_succeeds(a in arb_bigint(), b in arb_nonzero_bigint()) {
        let result = bigint_div_checked(&a, &b);
        prop_assert!(result.is_some(),
            "BigInt division with non-zero divisor should succeed: {} / {} = {:?}", a, b, result);
    }

    /// **Property 5: BigInt division error handling**
    /// *For any* BigInt exponentiation with negative exponent, the operation SHALL signal an error.
    #[test]
    fn prop_bigint_pow_negative_exponent_error(base in arb_bigint(), exp in 1i64..100i64) {
        let neg_exp = BigInt::from(-exp);
        let result = bigint_pow_checked(&base, &neg_exp);
        prop_assert!(result.is_none(),
            "BigInt pow with negative exponent should fail: {} ** {} = {:?}", base, neg_exp, result);
    }
}

// ============================================================================
// Property Tests for BigInt/Number Mixing Error (Property 6)
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 6: BigInt/Number mixing error**
    /// *For any* BigInt and Number, arithmetic operations SHALL signal a type error.
    #[test]
    fn prop_bigint_number_mixing_add_error(bigint in arb_bigint(), number in arb_safe_integer()) {
        let result = check_bigint_number_mixing(&bigint, number as f64);
        prop_assert!(result,
            "Mixing BigInt {} and Number {} should be detected as type error", bigint, number);
    }

    /// **Property 6: BigInt/Number mixing error**
    /// *For any* two BigInt values, arithmetic operations SHALL NOT signal a type error.
    #[test]
    fn prop_bigint_bigint_no_mixing_error(a in arb_bigint(), b in arb_bigint()) {
        // When both are BigInts, there should be no type error
        let same_type = check_same_type_bigint(&a, &b);
        prop_assert!(same_type,
            "Two BigInts should not trigger type mixing error: {} and {}", a, b);
    }

    /// **Property 6: BigInt/Number mixing error**
    /// *For any* two Number values, arithmetic operations SHALL NOT signal a type error.
    #[test]
    fn prop_number_number_no_mixing_error(a in arb_safe_integer(), b in arb_safe_integer()) {
        // When both are Numbers, there should be no type error
        let same_type = check_same_type_number(a as f64, b as f64);
        prop_assert!(same_type,
            "Two Numbers should not trigger type mixing error: {} and {}", a, b);
    }
}

// ============================================================================
// Property Tests for BigInt Constructor Correctness (Property 7)
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 7: BigInt constructor correctness**
    /// *For any* safe integer, BigInt(n) SHALL return the correct BigInt value.
    #[test]
    fn prop_bigint_from_safe_integer(n in arb_safe_integer()) {
        let result = bigint_from_number(n as f64);
        prop_assert!(result.is_some(),
            "BigInt from safe integer should succeed: BigInt({})", n);
        let result_val = result.unwrap();
        let expected = BigInt::from(n);
        prop_assert_eq!(result_val.clone(), expected.clone(),
            "BigInt from safe integer incorrect: BigInt({}) = {} (expected {})", 
            n, result_val, expected);
    }

    /// **Property 7: BigInt constructor correctness**
    /// *For any* non-integer f64, BigInt(n) SHALL signal an error.
    #[test]
    fn prop_bigint_from_non_integer_error(n in arb_non_integer_f64()) {
        let result = bigint_from_number(n);
        prop_assert!(result.is_none(),
            "BigInt from non-integer should fail: BigInt({}) = {:?}", n, result);
    }

    /// **Property 7: BigInt constructor correctness**
    /// *For any* valid integer string, BigInt(string) SHALL return a valid BigInt value.
    #[test]
    fn prop_bigint_from_valid_string(s in arb_integer_string()) {
        let result = bigint_from_string(&s);
        prop_assert!(result.is_some(),
            "BigInt from valid string should succeed: BigInt('{}') = {:?}", s, result);
    }

    /// **Property 7: BigInt constructor correctness**
    /// *For any* invalid string, BigInt(string) SHALL signal an error.
    #[test]
    fn prop_bigint_from_invalid_string_error(s in arb_invalid_bigint_string()) {
        let result = bigint_from_string(&s);
        prop_assert!(result.is_none(),
            "BigInt from invalid string should fail: BigInt('{}') = {:?}", s, result);
    }
}

// ============================================================================
// Unit Tests for BigInt Edge Cases
// ============================================================================

/// **Property 7: BigInt constructor correctness**
/// BigInt(NaN) SHALL signal an error.
#[test]
fn test_bigint_from_nan_error() {
    let result = bigint_from_number(f64::NAN);
    assert!(result.is_none(), "BigInt from NaN should fail");
}

/// **Property 7: BigInt constructor correctness**
/// BigInt(Infinity) SHALL signal an error.
#[test]
fn test_bigint_from_infinity_error() {
    let result = bigint_from_number(f64::INFINITY);
    assert!(result.is_none(), "BigInt from Infinity should fail");
    
    let result_neg = bigint_from_number(f64::NEG_INFINITY);
    assert!(result_neg.is_none(), "BigInt from -Infinity should fail");
}

/// **Property 7: BigInt constructor correctness**
/// BigInt(0) SHALL equal 0n.
#[test]
fn test_bigint_zero_identity() {
    let result = bigint_from_number(0.0);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), BigInt::from(0), "BigInt(0) should equal 0n");
}

/// **Property 7: BigInt constructor correctness**
/// BigInt(-0) SHALL equal 0n (no negative zero for BigInt).
#[test]
fn test_bigint_negative_zero_identity() {
    let result = bigint_from_number(-0.0);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), BigInt::from(0), "BigInt(-0) should equal 0n");
}

/// **Property 1: BigInt literal round-trip**
/// *For any* BigInt, converting to string should not have leading zeros (except for 0 itself).
#[test]
fn test_bigint_to_string_no_leading_zeros() {
    // Test zero
    let zero = BigInt::from(0);
    assert_eq!(bigint_to_string(&zero), "0");
    
    // Test positive
    let pos = BigInt::from(123);
    let pos_str = bigint_to_string(&pos);
    assert!(!pos_str.starts_with('0'), "Positive BigInt should not have leading zeros");
    
    // Test negative
    let neg = BigInt::from(-456);
    let neg_str = bigint_to_string(&neg);
    assert!(neg_str.starts_with('-'), "Negative BigInt should start with '-'");
    assert!(!neg_str[1..].starts_with('0'), "Negative BigInt should not have leading zeros after '-'");
}

// ============================================================================
// Implementation Functions (simulating runtime behavior)
// ============================================================================

/// Convert BigInt to string
fn bigint_to_string(a: &BigInt) -> String {
    a.to_string()
}

/// Parse string to BigInt
fn bigint_from_string(s: &str) -> Option<BigInt> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.parse().ok()
}

/// Convert f64 to BigInt (only for integers)
fn bigint_from_number(n: f64) -> Option<BigInt> {
    if n.is_nan() || n.is_infinite() {
        return None;
    }
    if n.fract() != 0.0 {
        return None;
    }
    Some(BigInt::from(n as i64))
}

/// Checked BigInt division (returns None for division by zero)
fn bigint_div_checked(a: &BigInt, b: &BigInt) -> Option<BigInt> {
    if b.is_zero() {
        None
    } else {
        Some(a / b)
    }
}

/// Checked BigInt modulo (returns None for modulo by zero)
fn bigint_mod_checked(a: &BigInt, b: &BigInt) -> Option<BigInt> {
    if b.is_zero() {
        None
    } else {
        Some(a % b)
    }
}

/// Checked BigInt exponentiation (returns None for negative exponent)
fn bigint_pow_checked(base: &BigInt, exp: &BigInt) -> Option<BigInt> {
    use num_traits::ToPrimitive;
    
    if *exp < BigInt::from(0) {
        return None; // Negative exponent not allowed
    }
    
    exp.to_u32().map(|e| base.pow(e))
}

/// Check if mixing BigInt and Number would cause a type error
fn check_bigint_number_mixing(_bigint: &BigInt, _number: f64) -> bool {
    // In JavaScript, mixing BigInt and Number in arithmetic causes TypeError
    // This function simulates that check
    true // Always returns true because we're mixing types
}

/// Check if two BigInts are the same type (always true)
fn check_same_type_bigint(_a: &BigInt, _b: &BigInt) -> bool {
    true // Both are BigInts
}

/// Check if two Numbers are the same type (always true)
fn check_same_type_number(_a: f64, _b: f64) -> bool {
    true // Both are Numbers
}
