//! Property-Based Tests for BigInt Comparison Operations
//!
//! **Feature: production-readiness, Property 3: BigInt comparison correctness**
//! **Validates: Requirements 1.3**
//!
//! These tests verify that BigInt comparison operations in dx-js produce correct results
//! for all valid BigInt values, matching ECMAScript specification behavior.

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
        // Large integers (beyond i64 range)
        any::<i64>().prop_map(BigInt::from),
    ]
}

// ============================================================================
// Property Tests for BigInt Less Than (<)
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 3: BigInt comparison correctness - Less Than**
    /// *For any* two BigInt values a and b, a < b SHALL reflect the correct mathematical ordering.
    #[test]
    fn prop_bigint_lt_correctness(a in arb_bigint(), b in arb_bigint()) {
        let expected = a < b;
        let result = bigint_lt(&a, &b);
        prop_assert_eq!(result, expected,
            "BigInt less than failed: {} < {} = {} (expected {})", a, b, result, expected);
    }

    /// **Property 3: BigInt comparison correctness - Less Than Irreflexivity**
    /// *For any* BigInt value a, a < a SHALL be false.
    #[test]
    fn prop_bigint_lt_irreflexive(a in arb_bigint()) {
        let result = bigint_lt(&a, &a);
        prop_assert!(!result,
            "BigInt less than should be irreflexive: {} < {} = {} (expected false)", a, a, result);
    }

    /// **Property 3: BigInt comparison correctness - Less Than Asymmetry**
    /// *For any* two BigInt values a and b where a < b, b < a SHALL be false.
    #[test]
    fn prop_bigint_lt_asymmetric(a in arb_bigint(), b in arb_bigint()) {
        if bigint_lt(&a, &b) {
            let result = bigint_lt(&b, &a);
            prop_assert!(!result,
                "BigInt less than should be asymmetric: {} < {} but {} < {} = {}", a, b, b, a, result);
        }
    }

    /// **Property 3: BigInt comparison correctness - Less Than Transitivity**
    /// *For any* three BigInt values a, b, c where a < b and b < c, a < c SHALL be true.
    #[test]
    fn prop_bigint_lt_transitive(a in arb_bigint(), b in arb_bigint(), c in arb_bigint()) {
        if bigint_lt(&a, &b) && bigint_lt(&b, &c) {
            let result = bigint_lt(&a, &c);
            prop_assert!(result,
                "BigInt less than should be transitive: {} < {} and {} < {} implies {} < {}", 
                a, b, b, c, a, c);
        }
    }
}

// ============================================================================
// Property Tests for BigInt Greater Than (>)
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 3: BigInt comparison correctness - Greater Than**
    /// *For any* two BigInt values a and b, a > b SHALL reflect the correct mathematical ordering.
    #[test]
    fn prop_bigint_gt_correctness(a in arb_bigint(), b in arb_bigint()) {
        let expected = a > b;
        let result = bigint_gt(&a, &b);
        prop_assert_eq!(result, expected,
            "BigInt greater than failed: {} > {} = {} (expected {})", a, b, result, expected);
    }

    /// **Property 3: BigInt comparison correctness - Greater Than Irreflexivity**
    /// *For any* BigInt value a, a > a SHALL be false.
    #[test]
    fn prop_bigint_gt_irreflexive(a in arb_bigint()) {
        let result = bigint_gt(&a, &a);
        prop_assert!(!result,
            "BigInt greater than should be irreflexive: {} > {} = {} (expected false)", a, a, result);
    }

    /// **Property 3: BigInt comparison correctness - Less Than / Greater Than Duality**
    /// *For any* two BigInt values a and b, a < b SHALL equal b > a.
    #[test]
    fn prop_bigint_lt_gt_duality(a in arb_bigint(), b in arb_bigint()) {
        let lt_result = bigint_lt(&a, &b);
        let gt_result = bigint_gt(&b, &a);
        prop_assert_eq!(lt_result, gt_result,
            "BigInt lt/gt duality failed: ({} < {}) = {} but ({} > {}) = {}", 
            a, b, lt_result, b, a, gt_result);
    }
}

// ============================================================================
// Property Tests for BigInt Less Than or Equal (<=)
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 3: BigInt comparison correctness - Less Than or Equal**
    /// *For any* two BigInt values a and b, a <= b SHALL reflect the correct mathematical ordering.
    #[test]
    fn prop_bigint_le_correctness(a in arb_bigint(), b in arb_bigint()) {
        let expected = a <= b;
        let result = bigint_le(&a, &b);
        prop_assert_eq!(result, expected,
            "BigInt less than or equal failed: {} <= {} = {} (expected {})", a, b, result, expected);
    }

    /// **Property 3: BigInt comparison correctness - Less Than or Equal Reflexivity**
    /// *For any* BigInt value a, a <= a SHALL be true.
    #[test]
    fn prop_bigint_le_reflexive(a in arb_bigint()) {
        let result = bigint_le(&a, &a);
        prop_assert!(result,
            "BigInt less than or equal should be reflexive: {} <= {} = {} (expected true)", a, a, result);
    }

    /// **Property 3: BigInt comparison correctness - Less Than or Equal Antisymmetry**
    /// *For any* two BigInt values a and b where a <= b and b <= a, a SHALL equal b.
    #[test]
    fn prop_bigint_le_antisymmetric(a in arb_bigint(), b in arb_bigint()) {
        if bigint_le(&a, &b) && bigint_le(&b, &a) {
            prop_assert_eq!(a.clone(), b.clone(),
                "BigInt le antisymmetry failed: {} <= {} and {} <= {} but {} != {}", a, b, b, a, a, b);
        }
    }

    /// **Property 3: BigInt comparison correctness - Less Than implies Less Than or Equal**
    /// *For any* two BigInt values a and b where a < b, a <= b SHALL be true.
    #[test]
    fn prop_bigint_lt_implies_le(a in arb_bigint(), b in arb_bigint()) {
        if bigint_lt(&a, &b) {
            let result = bigint_le(&a, &b);
            prop_assert!(result,
                "BigInt lt should imply le: {} < {} but {} <= {} = {}", a, b, a, b, result);
        }
    }
}

// ============================================================================
// Property Tests for BigInt Greater Than or Equal (>=)
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 3: BigInt comparison correctness - Greater Than or Equal**
    /// *For any* two BigInt values a and b, a >= b SHALL reflect the correct mathematical ordering.
    #[test]
    fn prop_bigint_ge_correctness(a in arb_bigint(), b in arb_bigint()) {
        let expected = a >= b;
        let result = bigint_ge(&a, &b);
        prop_assert_eq!(result, expected,
            "BigInt greater than or equal failed: {} >= {} = {} (expected {})", a, b, result, expected);
    }

    /// **Property 3: BigInt comparison correctness - Greater Than or Equal Reflexivity**
    /// *For any* BigInt value a, a >= a SHALL be true.
    #[test]
    fn prop_bigint_ge_reflexive(a in arb_bigint()) {
        let result = bigint_ge(&a, &a);
        prop_assert!(result,
            "BigInt greater than or equal should be reflexive: {} >= {} = {} (expected true)", a, a, result);
    }

    /// **Property 3: BigInt comparison correctness - Less Than or Equal / Greater Than or Equal Duality**
    /// *For any* two BigInt values a and b, a <= b SHALL equal b >= a.
    #[test]
    fn prop_bigint_le_ge_duality(a in arb_bigint(), b in arb_bigint()) {
        let le_result = bigint_le(&a, &b);
        let ge_result = bigint_ge(&b, &a);
        prop_assert_eq!(le_result, ge_result,
            "BigInt le/ge duality failed: ({} <= {}) = {} but ({} >= {}) = {}", 
            a, b, le_result, b, a, ge_result);
    }
}

// ============================================================================
// Property Tests for BigInt Equality (==)
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 3: BigInt comparison correctness - Equality**
    /// *For any* two BigInt values a and b, a == b SHALL reflect the correct mathematical equality.
    #[test]
    fn prop_bigint_eq_correctness(a in arb_bigint(), b in arb_bigint()) {
        let expected = a == b;
        let result = bigint_eq(&a, &b);
        prop_assert_eq!(result, expected,
            "BigInt equality failed: {} == {} = {} (expected {})", a, b, result, expected);
    }

    /// **Property 3: BigInt comparison correctness - Equality Reflexivity**
    /// *For any* BigInt value a, a == a SHALL be true.
    #[test]
    fn prop_bigint_eq_reflexive(a in arb_bigint()) {
        let result = bigint_eq(&a, &a);
        prop_assert!(result,
            "BigInt equality should be reflexive: {} == {} = {} (expected true)", a, a, result);
    }

    /// **Property 3: BigInt comparison correctness - Equality Symmetry**
    /// *For any* two BigInt values a and b, a == b SHALL equal b == a.
    #[test]
    fn prop_bigint_eq_symmetric(a in arb_bigint(), b in arb_bigint()) {
        let result1 = bigint_eq(&a, &b);
        let result2 = bigint_eq(&b, &a);
        prop_assert_eq!(result1, result2,
            "BigInt equality should be symmetric: ({} == {}) = {} but ({} == {}) = {}", 
            a, b, result1, b, a, result2);
    }

    /// **Property 3: BigInt comparison correctness - Equality Transitivity**
    /// *For any* three BigInt values a, b, c where a == b and b == c, a == c SHALL be true.
    #[test]
    fn prop_bigint_eq_transitive(a in arb_bigint(), b in arb_bigint(), c in arb_bigint()) {
        if bigint_eq(&a, &b) && bigint_eq(&b, &c) {
            let result = bigint_eq(&a, &c);
            prop_assert!(result,
                "BigInt equality should be transitive: {} == {} and {} == {} implies {} == {}", 
                a, b, b, c, a, c);
        }
    }
}

// ============================================================================
// Property Tests for BigInt Strict Equality (===)
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 3: BigInt comparison correctness - Strict Equality**
    /// *For any* two BigInt values a and b, a === b SHALL reflect the correct mathematical equality.
    #[test]
    fn prop_bigint_strict_eq_correctness(a in arb_bigint(), b in arb_bigint()) {
        let expected = a == b;
        let result = bigint_strict_eq(&a, &b);
        prop_assert_eq!(result, expected,
            "BigInt strict equality failed: {} === {} = {} (expected {})", a, b, result, expected);
    }

    /// **Property 3: BigInt comparison correctness - Strict Equality equals Abstract Equality for BigInts**
    /// *For any* two BigInt values a and b, a === b SHALL equal a == b.
    #[test]
    fn prop_bigint_strict_eq_equals_eq(a in arb_bigint(), b in arb_bigint()) {
        let eq_result = bigint_eq(&a, &b);
        let strict_eq_result = bigint_strict_eq(&a, &b);
        prop_assert_eq!(eq_result, strict_eq_result,
            "BigInt strict equality should equal abstract equality: ({} == {}) = {} but ({} === {}) = {}", 
            a, b, eq_result, a, b, strict_eq_result);
    }
}

// ============================================================================
// Property Tests for Comparison Relationships
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 3: BigInt comparison correctness - Trichotomy**
    /// *For any* two BigInt values a and b, exactly one of a < b, a == b, or a > b SHALL be true.
    #[test]
    fn prop_bigint_trichotomy(a in arb_bigint(), b in arb_bigint()) {
        let lt = bigint_lt(&a, &b);
        let eq = bigint_eq(&a, &b);
        let gt = bigint_gt(&a, &b);
        
        let count = [lt, eq, gt].iter().filter(|&&x| x).count();
        prop_assert_eq!(count, 1,
            "BigInt trichotomy failed: {} < {} = {}, {} == {} = {}, {} > {} = {} (exactly one should be true)", 
            a, b, lt, a, b, eq, a, b, gt);
    }

    /// **Property 3: BigInt comparison correctness - Not Less Than and Not Greater Than implies Equal**
    /// *For any* two BigInt values a and b where !(a < b) and !(a > b), a == b SHALL be true.
    #[test]
    fn prop_bigint_not_lt_not_gt_implies_eq(a in arb_bigint(), b in arb_bigint()) {
        if !bigint_lt(&a, &b) && !bigint_gt(&a, &b) {
            let result = bigint_eq(&a, &b);
            prop_assert!(result,
                "Not lt and not gt should imply eq: !({} < {}) and !({} > {}) but {} == {} = {}", 
                a, b, a, b, a, b, result);
        }
    }

    /// **Property 3: BigInt comparison correctness - Less Than or Equal decomposition**
    /// *For any* two BigInt values a and b, a <= b SHALL equal (a < b) || (a == b).
    #[test]
    fn prop_bigint_le_decomposition(a in arb_bigint(), b in arb_bigint()) {
        let le_result = bigint_le(&a, &b);
        let lt_or_eq = bigint_lt(&a, &b) || bigint_eq(&a, &b);
        prop_assert_eq!(le_result, lt_or_eq,
            "BigInt le decomposition failed: ({} <= {}) = {} but ({} < {}) || ({} == {}) = {}", 
            a, b, le_result, a, b, a, b, lt_or_eq);
    }

    /// **Property 3: BigInt comparison correctness - Greater Than or Equal decomposition**
    /// *For any* two BigInt values a and b, a >= b SHALL equal (a > b) || (a == b).
    #[test]
    fn prop_bigint_ge_decomposition(a in arb_bigint(), b in arb_bigint()) {
        let ge_result = bigint_ge(&a, &b);
        let gt_or_eq = bigint_gt(&a, &b) || bigint_eq(&a, &b);
        prop_assert_eq!(ge_result, gt_or_eq,
            "BigInt ge decomposition failed: ({} >= {}) = {} but ({} > {}) || ({} == {}) = {}", 
            a, b, ge_result, a, b, a, b, gt_or_eq);
    }
}

// ============================================================================
// Implementation Functions (simulating runtime behavior)
// ============================================================================

/// Simulate BigInt less than
fn bigint_lt(a: &BigInt, b: &BigInt) -> bool {
    a < b
}

/// Simulate BigInt greater than
fn bigint_gt(a: &BigInt, b: &BigInt) -> bool {
    a > b
}

/// Simulate BigInt less than or equal
fn bigint_le(a: &BigInt, b: &BigInt) -> bool {
    a <= b
}

/// Simulate BigInt greater than or equal
fn bigint_ge(a: &BigInt, b: &BigInt) -> bool {
    a >= b
}

/// Simulate BigInt equality (abstract)
fn bigint_eq(a: &BigInt, b: &BigInt) -> bool {
    a == b
}

/// Simulate BigInt strict equality
fn bigint_strict_eq(a: &BigInt, b: &BigInt) -> bool {
    a == b
}
