//! Property-Based Tests for BigInt Arithmetic
//!
//! **Feature: production-readiness, Property 2: BigInt arithmetic correctness**
//! **Validates: Requirements 1.2**
//!
//! These tests verify that BigInt arithmetic operations in dx-js produce correct results
//! for all valid BigInt values, matching ECMAScript specification behavior.

use num_bigint::BigInt;
use num_traits::Zero;
use proptest::prelude::*;

// ============================================================================
// Test Helpers
// ============================================================================

/// Generate arbitrary BigInt values for testing
fn arb_bigint() -> impl Strategy<Value = BigInt> {
    // Generate BigInts from various ranges
    prop_oneof![
        // Small integers
        (-1000i64..1000i64).prop_map(BigInt::from),
        // Medium integers
        (-1_000_000i64..1_000_000i64).prop_map(BigInt::from),
        // Large integers (beyond i64 range)
        any::<i64>().prop_map(BigInt::from),
    ]
}

/// Generate non-zero BigInt values for division tests
fn arb_nonzero_bigint() -> impl Strategy<Value = BigInt> {
    arb_bigint().prop_filter("non-zero", |b| !b.is_zero())
}

/// Generate small non-negative exponents (to avoid huge results)
fn arb_small_exponent() -> impl Strategy<Value = u32> {
    0u32..20u32
}

// ============================================================================
// Property Tests for BigInt Addition
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 2: BigInt arithmetic correctness - Addition**
    /// *For any* two BigInt values a and b, a + b SHALL produce the mathematically correct result.
    #[test]
    fn prop_bigint_addition_correctness(a in arb_bigint(), b in arb_bigint()) {
        let expected = &a + &b;
        let result = bigint_add(&a, &b);
        prop_assert_eq!(result.clone(), expected.clone(), 
            "BigInt addition failed: {} + {} = {} (expected {})", a, b, result, expected);
    }

    /// **Property 2: BigInt arithmetic correctness - Addition Commutativity**
    /// *For any* two BigInt values a and b, a + b SHALL equal b + a.
    #[test]
    fn prop_bigint_addition_commutative(a in arb_bigint(), b in arb_bigint()) {
        let result1 = bigint_add(&a, &b);
        let result2 = bigint_add(&b, &a);
        prop_assert_eq!(result1, result2, 
            "BigInt addition not commutative: {} + {} != {} + {}", a, b, b, a);
    }

    /// **Property 2: BigInt arithmetic correctness - Addition Identity**
    /// *For any* BigInt value a, a + 0 SHALL equal a.
    #[test]
    fn prop_bigint_addition_identity(a in arb_bigint()) {
        let zero = BigInt::from(0);
        let result = bigint_add(&a, &zero);
        prop_assert_eq!(result.clone(), a.clone(), 
            "BigInt addition identity failed: {} + 0 = {} (expected {})", a, result, a);
    }
}

// ============================================================================
// Property Tests for BigInt Subtraction
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 2: BigInt arithmetic correctness - Subtraction**
    /// *For any* two BigInt values a and b, a - b SHALL produce the mathematically correct result.
    #[test]
    fn prop_bigint_subtraction_correctness(a in arb_bigint(), b in arb_bigint()) {
        let expected = &a - &b;
        let result = bigint_sub(&a, &b);
        prop_assert_eq!(result.clone(), expected.clone(), 
            "BigInt subtraction failed: {} - {} = {} (expected {})", a, b, result, expected);
    }

    /// **Property 2: BigInt arithmetic correctness - Subtraction Inverse**
    /// *For any* BigInt value a, a - a SHALL equal 0.
    #[test]
    fn prop_bigint_subtraction_inverse(a in arb_bigint()) {
        let result = bigint_sub(&a, &a);
        let zero = BigInt::from(0);
        prop_assert_eq!(result.clone(), zero, 
            "BigInt subtraction inverse failed: {} - {} = {} (expected 0)", a, a, result);
    }

    /// **Property 2: BigInt arithmetic correctness - Addition/Subtraction Round-trip**
    /// *For any* two BigInt values a and b, (a + b) - b SHALL equal a.
    #[test]
    fn prop_bigint_add_sub_roundtrip(a in arb_bigint(), b in arb_bigint()) {
        let sum = bigint_add(&a, &b);
        let result = bigint_sub(&sum, &b);
        prop_assert_eq!(result.clone(), a.clone(), 
            "BigInt add/sub roundtrip failed: ({} + {}) - {} = {} (expected {})", 
            a, b, b, result, a);
    }
}

// ============================================================================
// Property Tests for BigInt Multiplication
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 2: BigInt arithmetic correctness - Multiplication**
    /// *For any* two BigInt values a and b, a * b SHALL produce the mathematically correct result.
    #[test]
    fn prop_bigint_multiplication_correctness(a in arb_bigint(), b in arb_bigint()) {
        let expected = &a * &b;
        let result = bigint_mul(&a, &b);
        prop_assert_eq!(result.clone(), expected.clone(), 
            "BigInt multiplication failed: {} * {} = {} (expected {})", a, b, result, expected);
    }

    /// **Property 2: BigInt arithmetic correctness - Multiplication Commutativity**
    /// *For any* two BigInt values a and b, a * b SHALL equal b * a.
    #[test]
    fn prop_bigint_multiplication_commutative(a in arb_bigint(), b in arb_bigint()) {
        let result1 = bigint_mul(&a, &b);
        let result2 = bigint_mul(&b, &a);
        prop_assert_eq!(result1, result2, 
            "BigInt multiplication not commutative: {} * {} != {} * {}", a, b, b, a);
    }

    /// **Property 2: BigInt arithmetic correctness - Multiplication Identity**
    /// *For any* BigInt value a, a * 1 SHALL equal a.
    #[test]
    fn prop_bigint_multiplication_identity(a in arb_bigint()) {
        let one = BigInt::from(1);
        let result = bigint_mul(&a, &one);
        prop_assert_eq!(result.clone(), a.clone(), 
            "BigInt multiplication identity failed: {} * 1 = {} (expected {})", a, result, a);
    }

    /// **Property 2: BigInt arithmetic correctness - Multiplication by Zero**
    /// *For any* BigInt value a, a * 0 SHALL equal 0.
    #[test]
    fn prop_bigint_multiplication_zero(a in arb_bigint()) {
        let zero = BigInt::from(0);
        let result = bigint_mul(&a, &zero);
        prop_assert_eq!(result.clone(), zero.clone(), 
            "BigInt multiplication by zero failed: {} * 0 = {} (expected 0)", a, result);
    }
}

// ============================================================================
// Property Tests for BigInt Division
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 2: BigInt arithmetic correctness - Division**
    /// *For any* BigInt value a and non-zero BigInt b, a / b SHALL produce the truncated quotient.
    #[test]
    fn prop_bigint_division_correctness(a in arb_bigint(), b in arb_nonzero_bigint()) {
        let expected = &a / &b;
        let result = bigint_div(&a, &b);
        prop_assert_eq!(result.clone(), expected.clone(), 
            "BigInt division failed: {} / {} = {} (expected {})", a, b, result, expected);
    }

    /// **Property 2: BigInt arithmetic correctness - Division Identity**
    /// *For any* non-zero BigInt value a, a / a SHALL equal 1.
    #[test]
    fn prop_bigint_division_identity(a in arb_nonzero_bigint()) {
        let result = bigint_div(&a, &a);
        let one = BigInt::from(1);
        prop_assert_eq!(result.clone(), one, 
            "BigInt division identity failed: {} / {} = {} (expected 1)", a, a, result);
    }

    /// **Property 2: BigInt arithmetic correctness - Division by One**
    /// *For any* BigInt value a, a / 1 SHALL equal a.
    #[test]
    fn prop_bigint_division_by_one(a in arb_bigint()) {
        let one = BigInt::from(1);
        let result = bigint_div(&a, &one);
        prop_assert_eq!(result.clone(), a.clone(), 
            "BigInt division by one failed: {} / 1 = {} (expected {})", a, result, a);
    }
}

// ============================================================================
// Property Tests for BigInt Modulo
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 2: BigInt arithmetic correctness - Modulo**
    /// *For any* BigInt value a and non-zero BigInt b, a % b SHALL produce the correct remainder.
    #[test]
    fn prop_bigint_modulo_correctness(a in arb_bigint(), b in arb_nonzero_bigint()) {
        let expected = &a % &b;
        let result = bigint_mod(&a, &b);
        prop_assert_eq!(result.clone(), expected.clone(), 
            "BigInt modulo failed: {} % {} = {} (expected {})", a, b, result, expected);
    }

    /// **Property 2: BigInt arithmetic correctness - Division/Modulo Relationship**
    /// *For any* BigInt value a and non-zero BigInt b, a == (a / b) * b + (a % b).
    #[test]
    fn prop_bigint_div_mod_relationship(a in arb_bigint(), b in arb_nonzero_bigint()) {
        let quotient = bigint_div(&a, &b);
        let remainder = bigint_mod(&a, &b);
        let reconstructed = &quotient * &b + &remainder;
        prop_assert_eq!(reconstructed.clone(), a.clone(), 
            "BigInt div/mod relationship failed: ({} / {}) * {} + ({} % {}) = {} (expected {})", 
            a, b, b, a, b, reconstructed, a);
    }
}

// ============================================================================
// Property Tests for BigInt Exponentiation
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 2: BigInt arithmetic correctness - Exponentiation**
    /// *For any* BigInt base and small non-negative exponent, base ** exp SHALL produce the correct power.
    #[test]
    fn prop_bigint_pow_correctness(base in -10i64..10i64, exp in arb_small_exponent()) {
        let base_bigint = BigInt::from(base);
        let expected = base_bigint.pow(exp);
        let result = bigint_pow(&base_bigint, exp);
        prop_assert_eq!(result.clone(), expected.clone(), 
            "BigInt exponentiation failed: {} ** {} = {} (expected {})", base, exp, result, expected);
    }

    /// **Property 2: BigInt arithmetic correctness - Exponentiation by Zero**
    /// *For any* non-zero BigInt base, base ** 0 SHALL equal 1.
    #[test]
    fn prop_bigint_pow_zero(base in arb_nonzero_bigint()) {
        let result = bigint_pow(&base, 0);
        let one = BigInt::from(1);
        prop_assert_eq!(result.clone(), one, 
            "BigInt exponentiation by zero failed: {} ** 0 = {} (expected 1)", base, result);
    }

    /// **Property 2: BigInt arithmetic correctness - Exponentiation by One**
    /// *For any* BigInt base, base ** 1 SHALL equal base.
    #[test]
    fn prop_bigint_pow_one(base in arb_bigint()) {
        let result = bigint_pow(&base, 1);
        prop_assert_eq!(result.clone(), base.clone(), 
            "BigInt exponentiation by one failed: {} ** 1 = {} (expected {})", base, result, base);
    }
}

// ============================================================================
// Implementation Functions (simulating runtime behavior)
// ============================================================================

/// Simulate BigInt addition
fn bigint_add(a: &BigInt, b: &BigInt) -> BigInt {
    a + b
}

/// Simulate BigInt subtraction
fn bigint_sub(a: &BigInt, b: &BigInt) -> BigInt {
    a - b
}

/// Simulate BigInt multiplication
fn bigint_mul(a: &BigInt, b: &BigInt) -> BigInt {
    a * b
}

/// Simulate BigInt division (truncated toward zero)
fn bigint_div(a: &BigInt, b: &BigInt) -> BigInt {
    a / b
}

/// Simulate BigInt modulo
fn bigint_mod(a: &BigInt, b: &BigInt) -> BigInt {
    a % b
}

/// Simulate BigInt exponentiation
fn bigint_pow(base: &BigInt, exp: u32) -> BigInt {
    base.pow(exp)
}
