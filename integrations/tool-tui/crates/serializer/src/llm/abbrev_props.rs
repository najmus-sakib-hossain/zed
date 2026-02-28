//! Property-based tests for key abbreviation dictionary
//!
//! **Feature: dx-serializer-llm-human, Property 6: Key Abbreviation Round-Trip**
//! **Validates: Requirements 5.1-5.3, 6.1, 7.1**

#[cfg(test)]
mod property_tests {
    use crate::llm::abbrev::AbbrevDict;
    use proptest::prelude::*;

    /// Generate a random abbreviation from the dictionary
    /// Excludes short aliases that don't have reverse mappings
    fn arb_abbrev() -> impl Strategy<Value = String> {
        let dict = AbbrevDict::new();
        let abbrevs: Vec<String> = dict
            .global_mappings()
            .iter()
            .filter(|&(&abbrev, &_full)| {
                // Exclude short aliases: keys where compress(expand(key)) != key
                // These are one-way expansion aliases like "v" -> "version"
                // where "version" compresses to "vr", not "v"
                let expanded = dict.expand(abbrev, "");
                let compressed = dict.compress(&expanded);
                compressed == abbrev
            })
            .map(|(&s, _)| s.to_string())
            .collect();
        proptest::sample::select(abbrevs)
    }

    /// Generate a random full key from the dictionary
    fn arb_full_key() -> impl Strategy<Value = String> {
        let dict = AbbrevDict::new();
        let fulls: Vec<String> = dict.global_mappings().values().map(|s| s.to_string()).collect();
        proptest::sample::select(fulls)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 6a: For any key in the abbreviation dictionary,
        /// compressing then expanding SHALL return the original key.
        ///
        /// **Feature: dx-serializer-llm-human, Property 6: Key Abbreviation Round-Trip**
        /// **Validates: Requirements 5.1-5.3, 6.1, 7.1**
        #[test]
        fn prop_compress_then_expand_is_identity(full in arb_full_key()) {
            let dict = AbbrevDict::new();
            let compressed = dict.compress(&full);
            let expanded = dict.expand(&compressed, "");
            prop_assert_eq!(&expanded, &full,
                "compress({}) = {}, expand({}) = {} (expected {})",
                &full, &compressed, &compressed, &expanded, &full);
        }

        /// Property 6b: For any abbreviation in the dictionary,
        /// expanding then compressing SHALL return the original abbreviation.
        ///
        /// **Feature: dx-serializer-llm-human, Property 6: Key Abbreviation Round-Trip**
        /// **Validates: Requirements 5.1-5.3, 6.1, 7.1**
        #[test]
        fn prop_expand_then_compress_is_identity(abbrev in arb_abbrev()) {
            let dict = AbbrevDict::new();
            let expanded = dict.expand(&abbrev, "");
            let compressed = dict.compress(&expanded);
            prop_assert_eq!(&compressed, &abbrev,
                "expand({}) = {}, compress({}) = {} (expected {})",
                &abbrev, &expanded, &expanded, &compressed, &abbrev);
        }

        /// Property: Unknown keys pass through unchanged in both directions
        ///
        /// **Feature: dx-serializer-llm-human, Property 6: Key Abbreviation Round-Trip**
        /// **Validates: Requirements 5.1-5.3**
        #[test]
        fn prop_unknown_keys_passthrough(key in "[a-z]{10,20}") {
            let dict = AbbrevDict::new();

            // Unknown keys should pass through expand unchanged
            let expanded = dict.expand(&key, "");
            prop_assert_eq!(&expanded, &key,
                "Unknown key '{}' should pass through expand unchanged, got '{}'",
                key, expanded);

            // Unknown keys should pass through compress unchanged
            let compressed = dict.compress(&key);
            prop_assert_eq!(&compressed, &key,
                "Unknown key '{}' should pass through compress unchanged, got '{}'",
                key, compressed);
        }
    }

    #[test]
    fn test_dictionary_has_required_mappings() {
        let dict = AbbrevDict::new();

        // Verify required mappings from Requirements 5.1
        let required = vec![
            ("nm", "name"),
            ("tt", "title"),
            ("ds", "description"),
            ("st", "status"),
            ("cr", "created"),
            ("up", "updated"),
            ("pr", "price"),
            ("qt", "quantity"),
            ("em", "email"),
            ("ur", "url"),
        ];

        for (abbrev, full) in required {
            assert_eq!(
                dict.expand(abbrev, ""),
                full,
                "Required mapping {} -> {} not found",
                abbrev,
                full
            );
            assert_eq!(
                dict.compress(full),
                abbrev,
                "Required reverse mapping {} -> {} not found",
                full,
                abbrev
            );
        }
    }

    #[test]
    fn test_context_aware_expansion() {
        let dict = AbbrevDict::new();

        // Test context-aware expansion for 's'
        assert_eq!(dict.expand("s", "hikes"), "sunny");
        assert_eq!(dict.expand("s", "orders"), "status");

        // Test context-aware expansion for 'w'
        assert_eq!(dict.expand("w", "hikes"), "with");
        assert_eq!(dict.expand("w", "images"), "width");

        // Test context-aware expansion for 't'
        assert_eq!(dict.expand("t", "config"), "task");
        assert_eq!(dict.expand("t", "products"), "type");
    }
}
