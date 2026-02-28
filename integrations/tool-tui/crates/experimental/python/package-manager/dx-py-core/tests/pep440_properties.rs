//! Property-based tests for PEP 440 version parsing
//!
//! **Feature: dx-py-hardening, Property 1: PEP 440 Version Round-Trip**
//! **Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5, 2.7**
//!
//! **Feature: dx-py-hardening, Property 2: PEP 440 Version Ordering**
//! **Validates: Requirements 2.6**

use dx_py_core::pep440::{Pep440Version, PreRelease};
use proptest::prelude::*;

/// Strategy for generating valid release segments
fn release_strategy() -> impl Strategy<Value = Vec<u32>> {
    prop::collection::vec(0u32..1000, 1..5)
}

/// Strategy for generating pre-release
fn pre_release_strategy() -> impl Strategy<Value = Option<PreRelease>> {
    prop_oneof![
        Just(None),
        (0u32..100).prop_map(|n| Some(PreRelease::Alpha(n))),
        (0u32..100).prop_map(|n| Some(PreRelease::Beta(n))),
        (0u32..100).prop_map(|n| Some(PreRelease::ReleaseCandidate(n))),
    ]
}

/// Strategy for generating local version identifiers
fn local_strategy() -> impl Strategy<Value = Option<String>> {
    prop_oneof![
        Just(None),
        "[a-z0-9]{1,8}".prop_map(Some),
        "[a-z0-9]{1,4}\\.[a-z0-9]{1,4}".prop_map(Some),
    ]
}

/// Strategy for generating complete PEP 440 versions
fn pep440_version_strategy() -> impl Strategy<Value = Pep440Version> {
    (
        0u32..10,                    // epoch
        release_strategy(),          // release
        pre_release_strategy(),      // pre
        prop::option::of(0u32..100), // post
        prop::option::of(0u32..100), // dev
        local_strategy(),            // local
    )
        .prop_map(|(epoch, release, pre, post, dev, local)| Pep440Version {
            epoch,
            release,
            pre,
            post,
            dev,
            local,
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Property 1: PEP 440 Version Round-Trip**
    ///
    /// For any valid PEP 440 version, parsing then formatting SHALL produce
    /// a semantically equivalent version string.
    ///
    /// **Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5, 2.7**
    #[test]
    fn prop_version_roundtrip(version in pep440_version_strategy()) {
        // Format the version
        let formatted = version.to_string();

        // Parse it back
        let parsed = Pep440Version::parse(&formatted)
            .unwrap_or_else(|_| panic!("Failed to parse formatted version: {}", formatted));

        // Should be equal
        prop_assert_eq!(
            version.clone(), parsed.clone(),
            "Round-trip failed: {} -> {} -> {:?}",
            version, formatted, parsed
        );
    }

    /// **Property 2: PEP 440 Version Ordering - Transitivity**
    ///
    /// For any three versions a, b, c: if a < b and b < c, then a < c.
    ///
    /// **Validates: Requirements 2.6**
    #[test]
    fn prop_version_ordering_transitive(
        a in pep440_version_strategy(),
        b in pep440_version_strategy(),
        c in pep440_version_strategy()
    ) {
        use std::cmp::Ordering;

        // If a < b and b < c, then a < c
        if a.cmp(&b) == Ordering::Less && b.cmp(&c) == Ordering::Less {
            prop_assert_eq!(
                a.cmp(&c),
                Ordering::Less,
                "Transitivity violated: {} < {} < {} but {} !< {}",
                a, b, c, a, c
            );
        }

        // If a > b and b > c, then a > c
        if a.cmp(&b) == Ordering::Greater && b.cmp(&c) == Ordering::Greater {
            prop_assert_eq!(
                a.cmp(&c),
                Ordering::Greater,
                "Transitivity violated: {} > {} > {} but {} !> {}",
                a, b, c, a, c
            );
        }
    }

    /// **Property 2: PEP 440 Version Ordering - Antisymmetry**
    ///
    /// For any two versions a, b: if a <= b and b <= a, then a == b.
    ///
    /// **Validates: Requirements 2.6**
    #[test]
    fn prop_version_ordering_antisymmetric(
        a in pep440_version_strategy(),
        b in pep440_version_strategy()
    ) {
        use std::cmp::Ordering;

        let ab = a.cmp(&b);
        let ba = b.cmp(&a);

        // Antisymmetry: if a <= b and b <= a, then a == b
        if (ab == Ordering::Less || ab == Ordering::Equal)
            && (ba == Ordering::Less || ba == Ordering::Equal)
        {
            prop_assert_eq!(
                a.clone(), b.clone(),
                "Antisymmetry violated: {} <= {} and {} <= {} but {} != {}",
                a, b, b, a, a, b
            );
        }

        // Also check that cmp is consistent with eq
        if a == b {
            prop_assert_eq!(
                ab,
                Ordering::Equal,
                "Equality inconsistent with ordering: {} == {} but cmp returned {:?}",
                a, b, ab
            );
        }
    }

    /// **Property 2: PEP 440 Version Ordering - Reflexivity**
    ///
    /// For any version v: v == v.
    ///
    /// **Validates: Requirements 2.6**
    #[test]
    fn prop_version_ordering_reflexive(v in pep440_version_strategy()) {
        use std::cmp::Ordering;

        prop_assert_eq!(
            v.cmp(&v),
            Ordering::Equal,
            "Reflexivity violated: {} != {}",
            v, v
        );
        let v2 = v.clone();
        prop_assert_eq!(v, v2);
    }
}

/// Test specific PEP 440 ordering rules
#[test]
fn test_pep440_ordering_rules() {
    // dev < alpha < beta < rc < release < post
    let dev = Pep440Version::parse("1.0.0.dev1").unwrap();
    let alpha = Pep440Version::parse("1.0.0a1").unwrap();
    let beta = Pep440Version::parse("1.0.0b1").unwrap();
    let rc = Pep440Version::parse("1.0.0rc1").unwrap();
    let release = Pep440Version::parse("1.0.0").unwrap();
    let post = Pep440Version::parse("1.0.0.post1").unwrap();

    assert!(dev < alpha, "dev should be less than alpha");
    assert!(alpha < beta, "alpha should be less than beta");
    assert!(beta < rc, "beta should be less than rc");
    assert!(rc < release, "rc should be less than release");
    assert!(release < post, "release should be less than post");

    // Epoch always wins
    let v1 = Pep440Version::parse("2.0.0").unwrap();
    let v2 = Pep440Version::parse("1!1.0.0").unwrap();
    assert!(v1 < v2, "epoch 1 should be greater than epoch 0");

    // Local versions are greater than non-local
    let v1 = Pep440Version::parse("1.0.0").unwrap();
    let v2 = Pep440Version::parse("1.0.0+local").unwrap();
    assert!(v1 < v2, "local version should be greater");
}

/// Test parsing edge cases
#[test]
fn test_pep440_parsing_edge_cases() {
    // Various valid formats
    let cases = [
        "0.0.0",
        "1.0",
        "1.0.0.0.0",
        "2024.1.1",
        "1.0.0a0",
        "1.0.0b0",
        "1.0.0rc0",
        "1.0.0.dev0",
        "1.0.0.post0",
        "1.0.0+0",
        "v1.0.0",
        "V1.0.0",
        "1.0.0ALPHA1",
        "1.0.0-alpha-1",
        "1.0.0_alpha_1",
    ];

    for case in cases {
        let result = Pep440Version::parse(case);
        assert!(result.is_ok(), "Failed to parse: {}", case);
    }
}

/// Test that invalid versions are rejected
#[test]
fn test_pep440_invalid_versions() {
    let invalid = [
        "",
        "   ",
        "not-a-version",
        "1.0.0+", // empty local
        "!1.0.0", // invalid epoch
    ];

    for case in invalid {
        let result = Pep440Version::parse(case);
        assert!(result.is_err(), "Should have rejected: {}", case);
    }
}
