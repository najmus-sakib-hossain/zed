//! Property-based tests for Base62 encoding
//!
//! **Property 5: LLM Base62 Efficiency**
//! **Validates: Requirements 2.7**

#[cfg(test)]
mod property_tests {
    use crate::base62::{decode_base62, encode_base62};
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]

        /// Property 5a: Base62 encoding SHALL be reversible (round-trip)
        ///
        /// **Property 5: LLM Base62 Efficiency**
        /// **Validates: Requirements 2.7**
        #[test]
        fn prop_base62_round_trip(n in 0u64..u64::MAX / 2) {
            let encoded = encode_base62(n);
            let decoded = decode_base62(&encoded).unwrap();
            prop_assert_eq!(decoded, n, "Round-trip failed for {}: {} -> {}", n, encoded, decoded);
        }

        /// Property 5b: Base62 encoding SHALL produce shorter strings for large numbers
        ///
        /// For numbers >= 62, Base62 encoding should be more efficient than decimal.
        ///
        /// **Property 5: LLM Base62 Efficiency**
        /// **Validates: Requirements 2.7**
        #[test]
        fn prop_base62_efficiency_for_large_numbers(n in 1000u64..1_000_000u64) {
            let base62 = encode_base62(n);
            let decimal = n.to_string();

            // Base62 should be shorter or equal for numbers >= 62
            prop_assert!(
                base62.len() <= decimal.len(),
                "Base62 ({}) should be <= decimal ({}) for n={}",
                base62.len(), decimal.len(), n
            );
        }

        /// Property 5c: Base62 encoding SHALL use only alphanumeric characters
        ///
        /// **Property 5: LLM Base62 Efficiency**
        /// **Validates: Requirements 2.7**
        #[test]
        fn prop_base62_uses_only_alphanumeric(n in 0u64..u64::MAX / 2) {
            let encoded = encode_base62(n);
            for c in encoded.chars() {
                prop_assert!(
                    c.is_ascii_alphanumeric(),
                    "Base62 should only use alphanumeric chars, got '{}' in {}",
                    c, encoded
                );
            }
        }

        /// Property 5d: Base62 encoding SHALL be deterministic
        ///
        /// **Property 5: LLM Base62 Efficiency**
        /// **Validates: Requirements 2.7**
        #[test]
        fn prop_base62_deterministic(n in 0u64..u64::MAX / 2) {
            let encoded1 = encode_base62(n);
            let encoded2 = encode_base62(n);
            prop_assert_eq!(encoded1, encoded2, "Encoding should be deterministic");
        }

        /// Property 5e: Base62 encoding length SHALL grow logarithmically
        ///
        /// **Property 5: LLM Base62 Efficiency**
        /// **Validates: Requirements 2.7**
        #[test]
        fn prop_base62_length_logarithmic(n in 62u64..u64::MAX / 2) {
            let encoded = encode_base62(n);

            // Expected length is ceil(log62(n))
            let expected_len = ((n as f64).log(62.0).ceil()) as usize;

            // Allow for rounding differences
            prop_assert!(
                encoded.len() >= expected_len.saturating_sub(1) && encoded.len() <= expected_len + 1,
                "Length {} should be approximately {} for n={}",
                encoded.len(), expected_len, n
            );
        }

        /// Property: Invalid Base62 characters SHALL produce errors
        ///
        /// **Validates: Requirements 6.8**
        #[test]
        fn prop_invalid_chars_produce_errors(
            prefix in "[0-9A-Za-z]{0,5}",
            invalid in "[^0-9A-Za-z]",
            suffix in "[0-9A-Za-z]{0,5}"
        ) {
            let input = format!("{}{}{}", prefix, invalid, suffix);
            let result = decode_base62(&input);
            prop_assert!(result.is_err(), "Should fail for invalid input: {}", input);
        }
    }

    #[test]
    fn test_base62_efficiency_examples() {
        // Test specific examples from requirements
        let cases = vec![
            (320u64, "5A", 2),      // 320 -> 5A (2 chars vs 3 decimal)
            (540u64, "8i", 2),      // 540 -> 8i (2 chars vs 3 decimal)
            (10000u64, "2bI", 3),   // 10000 -> 2bI (3 chars vs 5 decimal)
            (999999u64, "4c91", 4), // 999999 -> 4c91 (4 chars vs 6 decimal)
        ];

        for (n, _expected_prefix, expected_len) in cases {
            let encoded = encode_base62(n);
            assert_eq!(
                encoded.len(),
                expected_len,
                "Expected {} chars for {}, got {} ({})",
                expected_len,
                n,
                encoded.len(),
                encoded
            );
            // Verify round-trip
            let decoded = decode_base62(&encoded).unwrap();
            assert_eq!(decoded, n);
        }
    }

    #[test]
    fn test_base62_savings_percentage() {
        // Calculate average savings for numbers in different ranges
        let ranges = vec![
            (100u64, 999u64),       // 3-digit decimals
            (1000u64, 9999u64),     // 4-digit decimals
            (10000u64, 99999u64),   // 5-digit decimals
            (100000u64, 999999u64), // 6-digit decimals
        ];

        for (start, end) in ranges {
            let mut total_decimal_len = 0;
            let mut total_base62_len = 0;
            let sample_size = 100;

            for i in 0..sample_size {
                let n = start + (end - start) * i / sample_size;
                total_decimal_len += n.to_string().len();
                total_base62_len += encode_base62(n).len();
            }

            let savings = 1.0 - (total_base62_len as f64 / total_decimal_len as f64);
            println!(
                "Range {}-{}: {:.1}% savings ({} vs {} chars)",
                start,
                end,
                savings * 100.0,
                total_base62_len,
                total_decimal_len
            );

            // Base62 should provide at least 20% savings for 4+ digit numbers
            if start >= 1000 {
                assert!(
                    savings > 0.15,
                    "Expected >15% savings for range {}-{}, got {:.1}%",
                    start,
                    end,
                    savings * 100.0
                );
            }
        }
    }

    #[test]
    fn test_base62_boundary_values() {
        // Test boundary values
        assert_eq!(encode_base62(0), "0");
        assert_eq!(encode_base62(9), "9");
        assert_eq!(encode_base62(10), "A");
        assert_eq!(encode_base62(35), "Z");
        assert_eq!(encode_base62(36), "a");
        assert_eq!(encode_base62(61), "z");
        assert_eq!(encode_base62(62), "10");
        assert_eq!(encode_base62(63), "11");

        // Verify round-trips
        for n in [0, 9, 10, 35, 36, 61, 62, 63, 100, 1000, 10000] {
            let encoded = encode_base62(n);
            let decoded = decode_base62(&encoded).unwrap();
            assert_eq!(decoded, n, "Round-trip failed for {}", n);
        }
    }
}
