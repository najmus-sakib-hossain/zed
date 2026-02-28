//! Property-based tests for version comparison SIMD/scalar equivalence
//!
//! **Feature: dx-py-package-manager, Property 5: SIMD/Scalar Version Resolution Equivalence**
//! **Validates: Requirements 4.5, 4.6**

use dx_py_core::version::{compare_versions, compare_versions_scalar, PackedVersion};
use proptest::prelude::*;

/// Generate arbitrary valid PackedVersion
fn arb_version() -> impl Strategy<Value = PackedVersion> {
    (0u32..1000u32, 0u32..1000u32, 0u32..1000u32)
        .prop_map(|(major, minor, patch)| PackedVersion::new(major, minor, patch))
}

/// Generate a vector of arbitrary versions
fn arb_version_vec(min: usize, max: usize) -> impl Strategy<Value = Vec<PackedVersion>> {
    proptest::collection::vec(arb_version(), min..max)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 5: SIMD and scalar comparison produce identical results
    ///
    /// *For any* version constraint and set of candidate versions,
    /// the SIMD-accelerated comparison SHALL produce the exact same
    /// matching results as the scalar fallback implementation.
    /// **Validates: Requirements 4.5, 4.6**
    #[test]
    fn prop_simd_scalar_equivalence(
        constraint in arb_version(),
        candidates in arb_version_vec(1, 64)
    ) {
        // Get results from both implementations
        let scalar_results = compare_versions_scalar(&constraint, &candidates);
        let unified_results = compare_versions(&constraint, &candidates);

        // Results should be identical
        prop_assert_eq!(
            scalar_results.len(),
            unified_results.len(),
            "Result lengths differ"
        );

        for (i, (scalar, unified)) in scalar_results.iter().zip(unified_results.iter()).enumerate() {
            prop_assert_eq!(
                scalar, unified,
                "Mismatch at index {}: constraint={:?}, candidate={:?}, scalar={}, unified={}",
                i, constraint, candidates[i], scalar, unified
            );
        }
    }

    /// Property 5: Version comparison is deterministic
    ///
    /// *For any* version constraint and candidates, running comparison
    /// multiple times SHALL produce identical results.
    /// **Validates: Requirements 4.5**
    #[test]
    fn prop_comparison_deterministic(
        constraint in arb_version(),
        candidates in arb_version_vec(1, 32)
    ) {
        let result1 = compare_versions(&constraint, &candidates);
        let result2 = compare_versions(&constraint, &candidates);
        let result3 = compare_versions(&constraint, &candidates);

        prop_assert_eq!(&result1, &result2, "Results differ between runs 1 and 2");
        prop_assert_eq!(&result2, &result3, "Results differ between runs 2 and 3");
    }

    /// Property 5: Version ordering is transitive
    ///
    /// *For any* three versions a, b, c, if a >= b and b >= c, then a >= c.
    /// **Validates: Requirements 4.6**
    #[test]
    fn prop_version_ordering_transitive(
        a in arb_version(),
        b in arb_version(),
        c in arb_version()
    ) {
        if a >= b && b >= c {
            prop_assert!(a >= c, "Transitivity violated: {:?} >= {:?} >= {:?} but {:?} < {:?}", a, b, c, a, c);
        }
    }

    /// Property 5: Version comparison is reflexive
    ///
    /// *For any* version v, v >= v should always be true.
    /// **Validates: Requirements 4.6**
    #[test]
    fn prop_version_comparison_reflexive(v in arb_version()) {
        let results = compare_versions(&v, &[v]);
        prop_assert!(results[0], "Version {:?} should satisfy >= itself", v);
    }

    /// Property 5: Version comparison respects major version
    ///
    /// *For any* two versions where major differs, the one with higher major
    /// should be greater regardless of minor/patch.
    /// **Validates: Requirements 4.6**
    #[test]
    fn prop_major_version_dominates(
        major1 in 0u32..500u32,
        major2 in 501u32..1000u32,
        minor1 in 0u32..1000u32,
        minor2 in 0u32..1000u32,
        patch1 in 0u32..1000u32,
        patch2 in 0u32..1000u32
    ) {
        let v1 = PackedVersion::new(major1, minor1, patch1);
        let v2 = PackedVersion::new(major2, minor2, patch2);

        // v2 has higher major, so v2 > v1 always
        prop_assert!(v2 > v1, "{:?} should be > {:?}", v2, v1);

        // v1 should not satisfy >= v2
        let results = compare_versions(&v2, &[v1]);
        prop_assert!(!results[0], "{:?} should not satisfy >= {:?}", v1, v2);
    }

    /// Property 5: Empty candidates returns empty results
    ///
    /// *For any* constraint, comparing against empty candidates
    /// SHALL return empty results.
    /// **Validates: Requirements 4.5**
    #[test]
    fn prop_empty_candidates_empty_results(constraint in arb_version()) {
        let results = compare_versions(&constraint, &[]);
        prop_assert!(results.is_empty(), "Empty candidates should produce empty results");
    }

    /// Property 5: All candidates >= 0.0.0
    ///
    /// *For any* set of candidates, all should satisfy >= 0.0.0.
    /// **Validates: Requirements 4.6**
    #[test]
    fn prop_all_versions_gte_zero(candidates in arb_version_vec(1, 32)) {
        let zero = PackedVersion::zero();
        let results = compare_versions(&zero, &candidates);

        for (i, result) in results.iter().enumerate() {
            prop_assert!(*result, "Version {:?} should satisfy >= 0.0.0", candidates[i]);
        }
    }
}
