//! Property-based tests for SIMD string operations
//!
//! Property 4: SIMD String Operation Correctness
//! Validates: Requirements 2.9
//!
//! For all string inputs, SIMD operations must produce the same result
//! as scalar operations.
//!
//! Property 26: AVX512-Scalar Equivalence
//! Validates: Requirements 2.1-2.5
//!
//! For all string inputs, AVX-512 operations must produce the same result
//! as scalar operations.

use dx_py_simd::avx512::Avx512StringEngine;
use dx_py_simd::scalar::ScalarStringEngine;
use dx_py_simd::*;
use proptest::prelude::*;

/// Strategy for generating ASCII strings (for SIMD-safe operations)
fn arb_ascii_string() -> impl Strategy<Value = String> {
    prop::collection::vec(0x20u8..0x7F, 0..500).prop_map(|bytes| String::from_utf8(bytes).unwrap())
}

/// Strategy for generating short ASCII strings (for needles/delimiters)
fn arb_short_ascii() -> impl Strategy<Value = String> {
    prop::collection::vec(0x20u8..0x7F, 1..20).prop_map(|bytes| String::from_utf8(bytes).unwrap())
}

/// Strategy for generating strings with mixed case
fn arb_mixed_case() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 ]{0,500}"
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 4: SIMD find produces same result as scalar find
    #[test]
    fn prop_find_correctness(
        haystack in arb_ascii_string(),
        needle in arb_short_ascii()
    ) {
        let simd_engine = get_engine();
        let scalar_engine = ScalarStringEngine::new();

        let simd_result = simd_engine.find(&haystack, &needle);
        let scalar_result = scalar_engine.find(&haystack, &needle);

        prop_assert_eq!(
            simd_result, scalar_result,
            "find mismatch for haystack={:?}, needle={:?}",
            haystack, needle
        );
    }

    /// Property 4: SIMD count produces same result as scalar count
    #[test]
    fn prop_count_correctness(
        haystack in arb_ascii_string(),
        needle in arb_short_ascii()
    ) {
        let simd_engine = get_engine();
        let scalar_engine = ScalarStringEngine::new();

        let simd_result = simd_engine.count(&haystack, &needle);
        let scalar_result = scalar_engine.count(&haystack, &needle);

        prop_assert_eq!(
            simd_result, scalar_result,
            "count mismatch for haystack={:?}, needle={:?}",
            haystack, needle
        );
    }

    /// Property 4: SIMD eq produces same result as scalar eq
    #[test]
    fn prop_eq_correctness(
        a in arb_ascii_string(),
        b in arb_ascii_string()
    ) {
        let simd_engine = get_engine();
        let scalar_engine = ScalarStringEngine::new();

        let simd_result = simd_engine.eq(&a, &b);
        let scalar_result = scalar_engine.eq(&a, &b);

        prop_assert_eq!(
            simd_result, scalar_result,
            "eq mismatch for a={:?}, b={:?}",
            a, b
        );
    }

    /// Property 4: SIMD to_lowercase produces same result as scalar
    #[test]
    fn prop_lowercase_correctness(s in arb_mixed_case()) {
        let simd_engine = get_engine();
        let scalar_engine = ScalarStringEngine::new();

        let simd_result = simd_engine.to_lowercase(&s);
        let scalar_result = scalar_engine.to_lowercase(&s);

        prop_assert_eq!(
            simd_result, scalar_result,
            "to_lowercase mismatch for s={:?}",
            s
        );
    }

    /// Property 4: SIMD to_uppercase produces same result as scalar
    #[test]
    fn prop_uppercase_correctness(s in arb_mixed_case()) {
        let simd_engine = get_engine();
        let scalar_engine = ScalarStringEngine::new();

        let simd_result = simd_engine.to_uppercase(&s);
        let scalar_result = scalar_engine.to_uppercase(&s);

        prop_assert_eq!(
            simd_result, scalar_result,
            "to_uppercase mismatch for s={:?}",
            s
        );
    }

    /// Property 4: SIMD split produces same result as scalar split
    #[test]
    fn prop_split_correctness(
        s in arb_ascii_string(),
        delimiter in arb_short_ascii()
    ) {
        let simd_engine = get_engine();
        let scalar_engine = ScalarStringEngine::new();

        let simd_result = simd_engine.split(&s, &delimiter);
        let scalar_result = scalar_engine.split(&s, &delimiter);

        prop_assert_eq!(
            simd_result, scalar_result,
            "split mismatch for s={:?}, delimiter={:?}",
            s, delimiter
        );
    }

    /// Property 4: SIMD join produces same result as scalar join
    #[test]
    fn prop_join_correctness(
        parts in prop::collection::vec(arb_short_ascii(), 0..20),
        separator in arb_short_ascii()
    ) {
        let simd_engine = get_engine();
        let scalar_engine = ScalarStringEngine::new();

        let parts_refs: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();

        let simd_result = simd_engine.join(&parts_refs, &separator);
        let scalar_result = scalar_engine.join(&parts_refs, &separator);

        prop_assert_eq!(
            simd_result, scalar_result,
            "join mismatch for parts={:?}, separator={:?}",
            parts, separator
        );
    }

    /// Property 4: SIMD replace produces same result as scalar replace
    #[test]
    fn prop_replace_correctness(
        s in arb_ascii_string(),
        from in arb_short_ascii(),
        to in arb_short_ascii()
    ) {
        let simd_engine = get_engine();
        let scalar_engine = ScalarStringEngine::new();

        let simd_result = simd_engine.replace(&s, &from, &to);
        let scalar_result = scalar_engine.replace(&s, &from, &to);

        prop_assert_eq!(
            simd_result, scalar_result,
            "replace mismatch for s={:?}, from={:?}, to={:?}",
            s, from, to
        );
    }

    /// Test that lowercase is idempotent
    #[test]
    fn prop_lowercase_idempotent(s in arb_mixed_case()) {
        let engine = get_engine();

        let once = engine.to_lowercase(&s);
        let twice = engine.to_lowercase(&once);

        prop_assert_eq!(once, twice, "lowercase should be idempotent");
    }

    /// Test that uppercase is idempotent
    #[test]
    fn prop_uppercase_idempotent(s in arb_mixed_case()) {
        let engine = get_engine();

        let once = engine.to_uppercase(&s);
        let twice = engine.to_uppercase(&once);

        prop_assert_eq!(once, twice, "uppercase should be idempotent");
    }

    /// Test that eq is reflexive
    #[test]
    fn prop_eq_reflexive(s in arb_ascii_string()) {
        let engine = get_engine();
        prop_assert!(engine.eq(&s, &s), "eq should be reflexive");
    }

    /// Test that eq is symmetric
    #[test]
    fn prop_eq_symmetric(a in arb_ascii_string(), b in arb_ascii_string()) {
        let engine = get_engine();
        prop_assert_eq!(
            engine.eq(&a, &b),
            engine.eq(&b, &a),
            "eq should be symmetric"
        );
    }

    /// Test that find returns valid index
    #[test]
    fn prop_find_valid_index(
        haystack in arb_ascii_string(),
        needle in arb_short_ascii()
    ) {
        let engine = get_engine();

        if let Some(idx) = engine.find(&haystack, &needle) {
            prop_assert!(
                idx + needle.len() <= haystack.len(),
                "find returned invalid index"
            );
            prop_assert_eq!(
                &haystack[idx..idx + needle.len()],
                needle,
                "find returned wrong position"
            );
        }
    }

    /// Test that split and join are inverse operations
    #[test]
    fn prop_split_join_inverse(
        s in "[a-z]{0,100}",
        delimiter in "[,;:|]{1}"
    ) {
        // Only test when delimiter doesn't appear in string
        prop_assume!(!s.contains(&delimiter));

        let engine = get_engine();

        let parts = engine.split(&s, &delimiter);
        let rejoined = engine.join(&parts, &delimiter);

        prop_assert_eq!(s, rejoined, "split/join should be inverse");
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_empty_string_operations() {
        let engine = get_engine();

        assert_eq!(engine.find("", ""), Some(0));
        assert_eq!(engine.find("", "a"), None);
        assert_eq!(engine.find("a", ""), Some(0));

        assert_eq!(engine.count("", "a"), 0);

        assert!(engine.eq("", ""));
        assert!(!engine.eq("", "a"));

        assert_eq!(engine.to_lowercase(""), "");
        assert_eq!(engine.to_uppercase(""), "");

        assert_eq!(engine.replace("", "a", "b"), "");
    }

    #[test]
    fn test_long_strings() {
        let engine = get_engine();

        // Test with strings longer than 32 bytes (AVX2 register size)
        let long_a = "a".repeat(1000);
        let long_b = "b".repeat(1000);
        let mixed = "aAbBcCdDeEfFgGhHiIjJkKlLmMnNoOpPqQrRsStTuUvVwWxXyYzZ".repeat(20);

        assert!(engine.eq(&long_a, &long_a));
        assert!(!engine.eq(&long_a, &long_b));

        assert_eq!(engine.find(&long_a, "a"), Some(0));
        assert_eq!(engine.find(&long_a, "b"), None);

        assert_eq!(engine.count(&long_a, "a"), 1000);

        let lower = engine.to_lowercase(&mixed);
        assert!(lower.chars().all(|c| !c.is_ascii_uppercase()));

        let upper = engine.to_uppercase(&mixed);
        assert!(upper.chars().all(|c| !c.is_ascii_lowercase()));
    }
}

/// AVX-512 specific property tests
/// Property 26: AVX512-Scalar Equivalence
/// Validates: Requirements 2.1-2.5
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
mod avx512_tests {
    use super::*;

    fn get_avx512_engine() -> Avx512StringEngine {
        // Safety: Tests will fall back to scalar if AVX-512 not available
        unsafe { Avx512StringEngine::new() }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 26: AVX-512 find produces same result as scalar find
        /// **Validates: Requirements 2.3**
        #[test]
        fn prop_avx512_find_correctness(
            haystack in arb_ascii_string(),
            needle in arb_short_ascii()
        ) {
            let avx512_engine = get_avx512_engine();
            let scalar_engine = ScalarStringEngine::new();

            let avx512_result = avx512_engine.find(&haystack, &needle);
            let scalar_result = scalar_engine.find(&haystack, &needle);

            prop_assert_eq!(
                avx512_result, scalar_result,
                "AVX-512 find mismatch for haystack={:?}, needle={:?}",
                haystack, needle
            );
        }

        /// Property 26: AVX-512 count produces same result as scalar count
        /// **Validates: Requirements 2.2**
        #[test]
        fn prop_avx512_count_correctness(
            haystack in arb_ascii_string(),
            needle in arb_short_ascii()
        ) {
            let avx512_engine = get_avx512_engine();
            let scalar_engine = ScalarStringEngine::new();

            let avx512_result = avx512_engine.count(&haystack, &needle);
            let scalar_result = scalar_engine.count(&haystack, &needle);

            prop_assert_eq!(
                avx512_result, scalar_result,
                "AVX-512 count mismatch for haystack={:?}, needle={:?}",
                haystack, needle
            );
        }

        /// Property 26: AVX-512 eq produces same result as scalar eq
        /// **Validates: Requirements 2.2**
        #[test]
        fn prop_avx512_eq_correctness(
            a in arb_ascii_string(),
            b in arb_ascii_string()
        ) {
            let avx512_engine = get_avx512_engine();
            let scalar_engine = ScalarStringEngine::new();

            let avx512_result = avx512_engine.eq(&a, &b);
            let scalar_result = scalar_engine.eq(&a, &b);

            prop_assert_eq!(
                avx512_result, scalar_result,
                "AVX-512 eq mismatch for a={:?}, b={:?}",
                a, b
            );
        }

        /// Property 26: AVX-512 to_lowercase produces same result as scalar
        /// **Validates: Requirements 2.2**
        #[test]
        fn prop_avx512_lowercase_correctness(s in arb_mixed_case()) {
            let avx512_engine = get_avx512_engine();
            let scalar_engine = ScalarStringEngine::new();

            let avx512_result = avx512_engine.to_lowercase(&s);
            let scalar_result = scalar_engine.to_lowercase(&s);

            prop_assert_eq!(
                avx512_result, scalar_result,
                "AVX-512 to_lowercase mismatch for s={:?}",
                s
            );
        }

        /// Property 26: AVX-512 to_uppercase produces same result as scalar
        /// **Validates: Requirements 2.2**
        #[test]
        fn prop_avx512_uppercase_correctness(s in arb_mixed_case()) {
            let avx512_engine = get_avx512_engine();
            let scalar_engine = ScalarStringEngine::new();

            let avx512_result = avx512_engine.to_uppercase(&s);
            let scalar_result = scalar_engine.to_uppercase(&s);

            prop_assert_eq!(
                avx512_result, scalar_result,
                "AVX-512 to_uppercase mismatch for s={:?}",
                s
            );
        }

        /// Property 26: AVX-512 split produces same result as scalar split
        /// **Validates: Requirements 2.2**
        #[test]
        fn prop_avx512_split_correctness(
            s in arb_ascii_string(),
            delimiter in arb_short_ascii()
        ) {
            let avx512_engine = get_avx512_engine();
            let scalar_engine = ScalarStringEngine::new();

            let avx512_result = avx512_engine.split(&s, &delimiter);
            let scalar_result = scalar_engine.split(&s, &delimiter);

            prop_assert_eq!(
                avx512_result, scalar_result,
                "AVX-512 split mismatch for s={:?}, delimiter={:?}",
                s, delimiter
            );
        }

        /// Property 26: AVX-512 join produces same result as scalar join
        /// **Validates: Requirements 2.2**
        #[test]
        fn prop_avx512_join_correctness(
            parts in prop::collection::vec(arb_short_ascii(), 0..20),
            separator in arb_short_ascii()
        ) {
            let avx512_engine = get_avx512_engine();
            let scalar_engine = ScalarStringEngine::new();

            let parts_refs: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();

            let avx512_result = avx512_engine.join(&parts_refs, &separator);
            let scalar_result = scalar_engine.join(&parts_refs, &separator);

            prop_assert_eq!(
                avx512_result, scalar_result,
                "AVX-512 join mismatch for parts={:?}, separator={:?}",
                parts, separator
            );
        }

        /// Property 26: AVX-512 replace produces same result as scalar replace
        /// **Validates: Requirements 2.2**
        #[test]
        fn prop_avx512_replace_correctness(
            s in arb_ascii_string(),
            from in arb_short_ascii(),
            to in arb_short_ascii()
        ) {
            let avx512_engine = get_avx512_engine();
            let scalar_engine = ScalarStringEngine::new();

            let avx512_result = avx512_engine.replace(&s, &from, &to);
            let scalar_result = scalar_engine.replace(&s, &from, &to);

            prop_assert_eq!(
                avx512_result, scalar_result,
                "AVX-512 replace mismatch for s={:?}, from={:?}, to={:?}",
                s, from, to
            );
        }
    }

    #[test]
    fn test_avx512_availability() {
        let available = Avx512StringEngine::is_available();
        println!("AVX-512 available: {}", available);

        // Verify the engine can be created and used regardless of availability
        let engine = get_avx512_engine();
        assert_eq!(engine.name(), "AVX-512");

        // Basic functionality test
        assert_eq!(engine.find("hello world", "world"), Some(6));
        assert!(engine.eq("hello", "hello"));
        assert_eq!(engine.to_lowercase("HELLO"), "hello");
        assert_eq!(engine.to_uppercase("hello"), "HELLO");
    }

    #[test]
    fn test_avx512_long_strings() {
        let engine = get_avx512_engine();

        // Test with strings longer than 64 bytes (AVX-512 register size)
        let long_a = "a".repeat(200);
        let long_b = "b".repeat(200);
        let mixed = "aAbBcCdDeEfFgGhHiIjJkKlLmMnNoOpPqQrRsStTuUvVwWxXyYzZ".repeat(10);

        assert!(engine.eq(&long_a, &long_a));
        assert!(!engine.eq(&long_a, &long_b));

        assert_eq!(engine.find(&long_a, "a"), Some(0));
        assert_eq!(engine.find(&long_a, "b"), None);

        assert_eq!(engine.count(&long_a, "a"), 200);

        let lower = engine.to_lowercase(&mixed);
        assert!(lower.chars().all(|c| !c.is_ascii_uppercase()));

        let upper = engine.to_uppercase(&mixed);
        assert!(upper.chars().all(|c| !c.is_ascii_lowercase()));
    }
}
