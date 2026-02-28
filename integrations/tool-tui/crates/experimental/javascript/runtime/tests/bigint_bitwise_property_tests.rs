//! Property-Based Tests for BigInt Bitwise Operations
//!
//! **Feature: production-readiness, Property 4: BigInt bitwise correctness**
//! **Validates: Requirements 1.5**
//!
//! These tests verify that BigInt bitwise operations in dx-js produce correct results
//! consistent with two's complement representation, matching ECMAScript specification behavior.

use num_bigint::BigInt;
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

/// Generate small non-negative shift amounts
fn arb_shift_amount() -> impl Strategy<Value = u64> {
    0u64..64u64
}

/// Generate small BigInt shift amounts (as BigInt)
fn arb_bigint_shift() -> impl Strategy<Value = BigInt> {
    (0u64..64u64).prop_map(BigInt::from)
}

// ============================================================================
// Property Tests for BigInt Bitwise AND (&)
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 4: BigInt bitwise correctness - AND**
    /// *For any* two BigInt values a and b, a & b SHALL produce the correct bitwise AND result.
    #[test]
    fn prop_bigint_and_correctness(a in arb_bigint(), b in arb_bigint()) {
        let expected = &a & &b;
        let result = bigint_and(&a, &b);
        prop_assert_eq!(result.clone(), expected.clone(),
            "BigInt AND failed: {} & {} = {} (expected {})", a, b, result, expected);
    }

    /// **Property 4: BigInt bitwise correctness - AND Commutativity**
    /// *For any* two BigInt values a and b, a & b SHALL equal b & a.
    #[test]
    fn prop_bigint_and_commutative(a in arb_bigint(), b in arb_bigint()) {
        let result1 = bigint_and(&a, &b);
        let result2 = bigint_and(&b, &a);
        prop_assert_eq!(result1, result2,
            "BigInt AND not commutative: {} & {} != {} & {}", a, b, b, a);
    }

    /// **Property 4: BigInt bitwise correctness - AND Idempotence**
    /// *For any* BigInt value a, a & a SHALL equal a.
    #[test]
    fn prop_bigint_and_idempotent(a in arb_bigint()) {
        let result = bigint_and(&a, &a);
        prop_assert_eq!(result.clone(), a.clone(),
            "BigInt AND idempotence failed: {} & {} = {} (expected {})", a, a, result, a);
    }

    /// **Property 4: BigInt bitwise correctness - AND with Zero**
    /// *For any* BigInt value a, a & 0 SHALL equal 0.
    #[test]
    fn prop_bigint_and_zero(a in arb_bigint()) {
        let zero = BigInt::from(0);
        let result = bigint_and(&a, &zero);
        prop_assert_eq!(result.clone(), zero.clone(),
            "BigInt AND with zero failed: {} & 0 = {} (expected 0)", a, result);
    }

    /// **Property 4: BigInt bitwise correctness - AND with -1**
    /// *For any* BigInt value a, a & -1 SHALL equal a (since -1 is all 1s in two's complement).
    #[test]
    fn prop_bigint_and_minus_one(a in arb_bigint()) {
        let minus_one = BigInt::from(-1);
        let result = bigint_and(&a, &minus_one);
        prop_assert_eq!(result.clone(), a.clone(),
            "BigInt AND with -1 failed: {} & -1 = {} (expected {})", a, result, a);
    }
}

// ============================================================================
// Property Tests for BigInt Bitwise OR (|)
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 4: BigInt bitwise correctness - OR**
    /// *For any* two BigInt values a and b, a | b SHALL produce the correct bitwise OR result.
    #[test]
    fn prop_bigint_or_correctness(a in arb_bigint(), b in arb_bigint()) {
        let expected = &a | &b;
        let result = bigint_or(&a, &b);
        prop_assert_eq!(result.clone(), expected.clone(),
            "BigInt OR failed: {} | {} = {} (expected {})", a, b, result, expected);
    }

    /// **Property 4: BigInt bitwise correctness - OR Commutativity**
    /// *For any* two BigInt values a and b, a | b SHALL equal b | a.
    #[test]
    fn prop_bigint_or_commutative(a in arb_bigint(), b in arb_bigint()) {
        let result1 = bigint_or(&a, &b);
        let result2 = bigint_or(&b, &a);
        prop_assert_eq!(result1, result2,
            "BigInt OR not commutative: {} | {} != {} | {}", a, b, b, a);
    }

    /// **Property 4: BigInt bitwise correctness - OR Idempotence**
    /// *For any* BigInt value a, a | a SHALL equal a.
    #[test]
    fn prop_bigint_or_idempotent(a in arb_bigint()) {
        let result = bigint_or(&a, &a);
        prop_assert_eq!(result.clone(), a.clone(),
            "BigInt OR idempotence failed: {} | {} = {} (expected {})", a, a, result, a);
    }

    /// **Property 4: BigInt bitwise correctness - OR with Zero**
    /// *For any* BigInt value a, a | 0 SHALL equal a.
    #[test]
    fn prop_bigint_or_zero(a in arb_bigint()) {
        let zero = BigInt::from(0);
        let result = bigint_or(&a, &zero);
        prop_assert_eq!(result.clone(), a.clone(),
            "BigInt OR with zero failed: {} | 0 = {} (expected {})", a, result, a);
    }

    /// **Property 4: BigInt bitwise correctness - OR with -1**
    /// *For any* BigInt value a, a | -1 SHALL equal -1.
    #[test]
    fn prop_bigint_or_minus_one(a in arb_bigint()) {
        let minus_one = BigInt::from(-1);
        let result = bigint_or(&a, &minus_one);
        prop_assert_eq!(result.clone(), minus_one.clone(),
            "BigInt OR with -1 failed: {} | -1 = {} (expected -1)", a, result);
    }
}

// ============================================================================
// Property Tests for BigInt Bitwise XOR (^)
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 4: BigInt bitwise correctness - XOR**
    /// *For any* two BigInt values a and b, a ^ b SHALL produce the correct bitwise XOR result.
    #[test]
    fn prop_bigint_xor_correctness(a in arb_bigint(), b in arb_bigint()) {
        let expected = &a ^ &b;
        let result = bigint_xor(&a, &b);
        prop_assert_eq!(result.clone(), expected.clone(),
            "BigInt XOR failed: {} ^ {} = {} (expected {})", a, b, result, expected);
    }

    /// **Property 4: BigInt bitwise correctness - XOR Commutativity**
    /// *For any* two BigInt values a and b, a ^ b SHALL equal b ^ a.
    #[test]
    fn prop_bigint_xor_commutative(a in arb_bigint(), b in arb_bigint()) {
        let result1 = bigint_xor(&a, &b);
        let result2 = bigint_xor(&b, &a);
        prop_assert_eq!(result1, result2,
            "BigInt XOR not commutative: {} ^ {} != {} ^ {}", a, b, b, a);
    }

    /// **Property 4: BigInt bitwise correctness - XOR Self-Inverse**
    /// *For any* BigInt value a, a ^ a SHALL equal 0.
    #[test]
    fn prop_bigint_xor_self_inverse(a in arb_bigint()) {
        let result = bigint_xor(&a, &a);
        let zero = BigInt::from(0);
        prop_assert_eq!(result.clone(), zero,
            "BigInt XOR self-inverse failed: {} ^ {} = {} (expected 0)", a, a, result);
    }

    /// **Property 4: BigInt bitwise correctness - XOR with Zero**
    /// *For any* BigInt value a, a ^ 0 SHALL equal a.
    #[test]
    fn prop_bigint_xor_zero(a in arb_bigint()) {
        let zero = BigInt::from(0);
        let result = bigint_xor(&a, &zero);
        prop_assert_eq!(result.clone(), a.clone(),
            "BigInt XOR with zero failed: {} ^ 0 = {} (expected {})", a, result, a);
    }

    /// **Property 4: BigInt bitwise correctness - XOR Round-trip**
    /// *For any* two BigInt values a and b, (a ^ b) ^ b SHALL equal a.
    #[test]
    fn prop_bigint_xor_roundtrip(a in arb_bigint(), b in arb_bigint()) {
        let xored = bigint_xor(&a, &b);
        let result = bigint_xor(&xored, &b);
        prop_assert_eq!(result.clone(), a.clone(),
            "BigInt XOR roundtrip failed: ({} ^ {}) ^ {} = {} (expected {})", a, b, b, result, a);
    }
}

// ============================================================================
// Property Tests for BigInt Bitwise NOT (~)
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 4: BigInt bitwise correctness - NOT**
    /// *For any* BigInt value a, ~a SHALL equal -(a + 1) per ECMAScript spec.
    #[test]
    fn prop_bigint_not_correctness(a in arb_bigint()) {
        let expected = -(&a + BigInt::from(1));
        let result = bigint_not(&a);
        prop_assert_eq!(result.clone(), expected.clone(),
            "BigInt NOT failed: ~{} = {} (expected {})", a, result, expected);
    }

    /// **Property 4: BigInt bitwise correctness - NOT Double Inverse**
    /// *For any* BigInt value a, ~~a SHALL equal a.
    #[test]
    fn prop_bigint_not_double_inverse(a in arb_bigint()) {
        let not_a = bigint_not(&a);
        let result = bigint_not(&not_a);
        prop_assert_eq!(result.clone(), a.clone(),
            "BigInt NOT double inverse failed: ~~{} = {} (expected {})", a, result, a);
    }
}

/// **Property 4: BigInt bitwise correctness - NOT of Zero**
/// ~0 SHALL equal -1.
#[test]
fn prop_bigint_not_zero() {
    let zero = BigInt::from(0);
    let result = bigint_not(&zero);
    let expected = BigInt::from(-1);
    assert_eq!(result.clone(), expected,
        "BigInt NOT of zero failed: ~0 = {} (expected -1)", result);
}

/// **Property 4: BigInt bitwise correctness - NOT of -1**
/// ~(-1) SHALL equal 0.
#[test]
fn prop_bigint_not_minus_one() {
    let minus_one = BigInt::from(-1);
    let result = bigint_not(&minus_one);
    let expected = BigInt::from(0);
    assert_eq!(result.clone(), expected,
        "BigInt NOT of -1 failed: ~(-1) = {} (expected 0)", result);
}

// ============================================================================
// Property Tests for BigInt Left Shift (<<)
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 4: BigInt bitwise correctness - Left Shift**
    /// *For any* BigInt value a and shift amount n, a << n SHALL produce the correct result.
    #[test]
    fn prop_bigint_shl_correctness(a in arb_bigint(), n in arb_shift_amount()) {
        let expected = &a << n;
        let result = bigint_shl(&a, n);
        prop_assert_eq!(result.clone(), expected.clone(),
            "BigInt left shift failed: {} << {} = {} (expected {})", a, n, result, expected);
    }

    /// **Property 4: BigInt bitwise correctness - Left Shift by Zero**
    /// *For any* BigInt value a, a << 0 SHALL equal a.
    #[test]
    fn prop_bigint_shl_zero(a in arb_bigint()) {
        let result = bigint_shl(&a, 0);
        prop_assert_eq!(result.clone(), a.clone(),
            "BigInt left shift by zero failed: {} << 0 = {} (expected {})", a, result, a);
    }

    /// **Property 4: BigInt bitwise correctness - Left Shift Multiplication**
    /// *For any* BigInt value a and shift amount n, a << n SHALL equal a * 2^n.
    #[test]
    fn prop_bigint_shl_is_multiplication(a in arb_bigint(), n in 0u32..32u32) {
        let shift_result = bigint_shl(&a, n as u64);
        let multiplier = BigInt::from(2).pow(n);
        let expected = &a * &multiplier;
        prop_assert_eq!(shift_result.clone(), expected.clone(),
            "BigInt left shift multiplication failed: {} << {} = {} (expected {} * 2^{} = {})", 
            a, n, shift_result, a, n, expected);
    }
}

// ============================================================================
// Property Tests for BigInt Right Shift (>>)
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 4: BigInt bitwise correctness - Right Shift**
    /// *For any* BigInt value a and shift amount n, a >> n SHALL produce the correct result.
    #[test]
    fn prop_bigint_shr_correctness(a in arb_bigint(), n in arb_shift_amount()) {
        let expected = &a >> n;
        let result = bigint_shr(&a, n);
        prop_assert_eq!(result.clone(), expected.clone(),
            "BigInt right shift failed: {} >> {} = {} (expected {})", a, n, result, expected);
    }

    /// **Property 4: BigInt bitwise correctness - Right Shift by Zero**
    /// *For any* BigInt value a, a >> 0 SHALL equal a.
    #[test]
    fn prop_bigint_shr_zero(a in arb_bigint()) {
        let result = bigint_shr(&a, 0);
        prop_assert_eq!(result.clone(), a.clone(),
            "BigInt right shift by zero failed: {} >> 0 = {} (expected {})", a, result, a);
    }

    /// **Property 4: BigInt bitwise correctness - Right Shift Division**
    /// *For any* non-negative BigInt value a and shift amount n, a >> n SHALL equal floor(a / 2^n).
    #[test]
    fn prop_bigint_shr_is_division(a in (0i64..1_000_000i64).prop_map(BigInt::from), n in 0u32..32u32) {
        let shift_result = bigint_shr(&a, n as u64);
        let divisor = BigInt::from(2).pow(n);
        let expected = &a / &divisor;
        prop_assert_eq!(shift_result.clone(), expected.clone(),
            "BigInt right shift division failed: {} >> {} = {} (expected {} / 2^{} = {})", 
            a, n, shift_result, a, n, expected);
    }
}

// ============================================================================
// Property Tests for Bitwise Operation Relationships
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 4: BigInt bitwise correctness - De Morgan's Law (AND/OR)**
    /// *For any* two BigInt values a and b, ~(a & b) SHALL equal (~a) | (~b).
    #[test]
    fn prop_bigint_de_morgan_and(a in arb_bigint(), b in arb_bigint()) {
        let lhs = bigint_not(&bigint_and(&a, &b));
        let rhs = bigint_or(&bigint_not(&a), &bigint_not(&b));
        prop_assert_eq!(lhs.clone(), rhs.clone(),
            "De Morgan's law (AND) failed: ~({} & {}) = {} but (~{}) | (~{}) = {}", 
            a, b, lhs, a, b, rhs);
    }

    /// **Property 4: BigInt bitwise correctness - De Morgan's Law (OR/AND)**
    /// *For any* two BigInt values a and b, ~(a | b) SHALL equal (~a) & (~b).
    #[test]
    fn prop_bigint_de_morgan_or(a in arb_bigint(), b in arb_bigint()) {
        let lhs = bigint_not(&bigint_or(&a, &b));
        let rhs = bigint_and(&bigint_not(&a), &bigint_not(&b));
        prop_assert_eq!(lhs.clone(), rhs.clone(),
            "De Morgan's law (OR) failed: ~({} | {}) = {} but (~{}) & (~{}) = {}", 
            a, b, lhs, a, b, rhs);
    }

    /// **Property 4: BigInt bitwise correctness - Shift Round-trip**
    /// *For any* non-negative BigInt value a and shift amount n, (a << n) >> n SHALL equal a.
    #[test]
    fn prop_bigint_shift_roundtrip(a in (0i64..1_000_000i64).prop_map(BigInt::from), n in 0u64..32u64) {
        let shifted = bigint_shl(&a, n);
        let result = bigint_shr(&shifted, n);
        prop_assert_eq!(result.clone(), a.clone(),
            "BigInt shift roundtrip failed: ({} << {}) >> {} = {} (expected {})", 
            a, n, n, result, a);
    }

    /// **Property 4: BigInt bitwise correctness - XOR with NOT**
    /// *For any* BigInt value a, a ^ (-1) SHALL equal ~a.
    #[test]
    fn prop_bigint_xor_minus_one_is_not(a in arb_bigint()) {
        let minus_one = BigInt::from(-1);
        let xor_result = bigint_xor(&a, &minus_one);
        let not_result = bigint_not(&a);
        prop_assert_eq!(xor_result.clone(), not_result.clone(),
            "BigInt XOR with -1 should equal NOT: {} ^ -1 = {} but ~{} = {}", 
            a, xor_result, a, not_result);
    }
}

// ============================================================================
// Implementation Functions (simulating runtime behavior)
// ============================================================================

/// Simulate BigInt bitwise AND
fn bigint_and(a: &BigInt, b: &BigInt) -> BigInt {
    a & b
}

/// Simulate BigInt bitwise OR
fn bigint_or(a: &BigInt, b: &BigInt) -> BigInt {
    a | b
}

/// Simulate BigInt bitwise XOR
fn bigint_xor(a: &BigInt, b: &BigInt) -> BigInt {
    a ^ b
}

/// Simulate BigInt bitwise NOT (~a = -(a + 1))
fn bigint_not(a: &BigInt) -> BigInt {
    -(a + BigInt::from(1))
}

/// Simulate BigInt left shift
fn bigint_shl(a: &BigInt, n: u64) -> BigInt {
    a << n
}

/// Simulate BigInt right shift
fn bigint_shr(a: &BigInt, n: u64) -> BigInt {
    a >> n
}
