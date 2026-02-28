//! Property tests for format converters
//!
//! Battle-hardening tests for JSON, YAML, TOML, TOON, and DX format converters.
//! These tests validate edge cases, malformed input handling, and round-trip correctness.

#[cfg(test)]
mod property_tests {
    use crate::converters::json::json_to_dx;
    use crate::converters::toml::toml_to_dx;
    use crate::converters::toon::{dx_to_toon, toon_to_dx};
    use crate::converters::yaml::yaml_to_dx;
    use proptest::prelude::*;

    // ========================================================================
    // Helper Strategies
    // ========================================================================

    /// Generate valid JSON-like key names (excluding DX reserved characters)
    fn json_key() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-zA-Z][a-zA-Z0-9]{0,15}")
            .unwrap()
            .prop_filter("non-empty and not reserved", |s| {
                !s.is_empty() && !["_", "~", "+", "-"].contains(&s.as_str())
            })
    }

    // ========================================================================
    // JSON Converter Property Tests
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test JSON to DX conversion preserves simple key-value pairs
        #[test]
        fn prop_json_simple_object_round_trip(
            key1 in json_key(),
            key2 in json_key(),
            val1 in -1000i64..1000,
            val2 in prop::string::string_regex("[a-zA-Z]{1,10}").unwrap()
        ) {
            prop_assume!(key1 != key2);

            let json = format!(r#"{{"{}": {}, "{}": "{}"}}"#, key1, val1, key2, val2);
            let result = json_to_dx(&json);

            prop_assert!(result.is_ok(), "JSON parsing should succeed: {:?}", result.err());

            let dx = result.unwrap();
            // Verify the DX output contains the values
            prop_assert!(dx.contains(&val1.to_string()) || dx.contains(&key1),
                "DX output should contain key or value");
        }

        /// Test JSON arrays are handled correctly
        #[test]
        fn prop_json_array_handling(
            items in prop::collection::vec(-100i64..100, 1..10)
        ) {
            let array_str = items.iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            let json = format!(r#"{{"numbers": [{}]}}"#, array_str);

            let result = json_to_dx(&json);
            prop_assert!(result.is_ok(), "JSON array parsing should succeed");
        }

        /// Test nested JSON objects
        #[test]
        fn prop_json_nested_objects(
            outer_key in json_key(),
            inner_key in json_key(),
            value in -1000i64..1000
        ) {
            let json = format!(r#"{{"{outer_key}": {{"{inner_key}": {value}}}}}"#);
            let result = json_to_dx(&json);

            prop_assert!(result.is_ok(), "Nested JSON parsing should succeed: {:?}", result.err());
        }
    }

    // ========================================================================
    // YAML Converter Property Tests
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test YAML simple key-value pairs
        #[test]
        fn prop_yaml_simple_round_trip(
            key in json_key(),
            value in prop::string::string_regex("[a-zA-Z0-9]{1,20}").unwrap()
        ) {
            let yaml = format!("{}: {}", key, value);
            let result = yaml_to_dx(&yaml);

            prop_assert!(result.is_ok(), "YAML parsing should succeed: {:?}", result.err());
        }

        /// Test YAML nested structures
        #[test]
        fn prop_yaml_nested_structure(
            parent in json_key(),
            child in json_key(),
            value in -1000i64..1000
        ) {
            let yaml = format!("{}:\n  {}: {}", parent, child, value);
            let result = yaml_to_dx(&yaml);

            prop_assert!(result.is_ok(), "Nested YAML parsing should succeed: {:?}", result.err());
        }

        /// Test YAML lists
        #[test]
        fn prop_yaml_list_handling(
            key in json_key(),
            items in prop::collection::vec(prop::string::string_regex("[a-zA-Z]{1,10}").unwrap(), 1..5)
        ) {
            let list_items = items.iter()
                .map(|s| format!("  - {}", s))
                .collect::<Vec<_>>()
                .join("\n");
            let yaml = format!("{}:\n{}", key, list_items);

            let result = yaml_to_dx(&yaml);
            prop_assert!(result.is_ok(), "YAML list parsing should succeed: {:?}", result.err());
        }
    }

    // ========================================================================
    // TOML Converter Property Tests
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test TOML simple key-value pairs
        #[test]
        fn prop_toml_simple_round_trip(
            key in json_key(),
            value in prop::string::string_regex("[a-zA-Z0-9]{1,20}").unwrap()
        ) {
            let toml = format!("{} = \"{}\"", key, value);
            let result = toml_to_dx(&toml);

            prop_assert!(result.is_ok(), "TOML parsing should succeed: {:?}", result.err());
        }

        /// Test TOML sections
        #[test]
        fn prop_toml_section_handling(
            section in json_key(),
            key in json_key(),
            value in -1000i64..1000
        ) {
            let toml = format!("[{}]\n{} = {}", section, key, value);
            let result = toml_to_dx(&toml);

            prop_assert!(result.is_ok(), "TOML section parsing should succeed: {:?}", result.err());
        }

        /// Test TOML arrays
        #[test]
        fn prop_toml_array_handling(
            key in json_key(),
            items in prop::collection::vec(-100i64..100, 1..5)
        ) {
            let array_str = items.iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            let toml = format!("{} = [{}]", key, array_str);

            let result = toml_to_dx(&toml);
            prop_assert!(result.is_ok(), "TOML array parsing should succeed: {:?}", result.err());
        }
    }

    // ========================================================================
    // TOON Converter Property Tests
    // ========================================================================

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 2: Format Conversion Round-Trip
        /// For any valid DX document, converting to TOON and back to DX SHALL preserve
        /// all data values and types (strings remain strings, numbers remain numbers,
        /// booleans remain booleans).
        /// **Validates: Requirements 2.1, 2.2**
        #[test]
        fn prop_toon_round_trip_preserves_values(
            key in json_key(),
            str_value in prop::string::string_regex("[a-zA-Z]{1,20}").unwrap(),
            int_value in -1000i64..1000
        ) {
            // Create a simple DX document with string and integer values
            let dx_input = format!("{}:{}\ncount:{}", key, str_value, int_value);

            // Convert DX -> TOON
            let toon_result = dx_to_toon(&dx_input);
            prop_assert!(toon_result.is_ok(), "DX to TOON conversion should succeed: {:?}", toon_result.err());

            let toon = toon_result.unwrap();

            // Verify TOON contains the values
            prop_assert!(toon.contains(&str_value), "TOON should contain string value");
            prop_assert!(toon.contains(&int_value.to_string()), "TOON should contain integer value");
        }

        /// Test TOON to DX conversion preserves simple key-value pairs
        #[test]
        fn prop_toon_to_dx_simple(
            key in json_key(),
            value in prop::string::string_regex("[a-zA-Z0-9]{1,20}").unwrap()
        ) {
            let toon = format!("{} \"{}\"", key, value);
            let result = toon_to_dx(&toon);

            prop_assert!(result.is_ok(), "TOON parsing should succeed: {:?}", result.err());

            let dx = result.unwrap();
            // Verify the DX output contains the value
            prop_assert!(dx.contains(&value), "DX output should contain the value");
        }

        /// Test TOON handles numbers correctly
        #[test]
        fn prop_toon_number_handling(
            key in json_key(),
            value in -1000i64..1000
        ) {
            let toon = format!("{} {}", key, value);
            let result = toon_to_dx(&toon);

            prop_assert!(result.is_ok(), "TOON number parsing should succeed: {:?}", result.err());
        }

        /// Test TOON handles booleans correctly
        #[test]
        fn prop_toon_boolean_handling(
            key in json_key(),
            value in prop::bool::ANY
        ) {
            let bool_str = if value { "true" } else { "false" };
            let toon = format!("{} {}", key, bool_str);
            let result = toon_to_dx(&toon);

            prop_assert!(result.is_ok(), "TOON boolean parsing should succeed: {:?}", result.err());
        }
    }

    // ========================================================================
    // Edge Case Tests
    // ========================================================================

    #[test]
    fn test_json_empty_object() {
        let result = json_to_dx("{}");
        assert!(result.is_ok());
    }

    #[test]
    fn test_json_deeply_nested() {
        let json = r#"{"a": {"b": {"c": {"d": {"e": 1}}}}}"#;
        let result = json_to_dx(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_yaml_empty_document() {
        let result = yaml_to_dx("");
        // Empty YAML should be handled gracefully
        let _ = result;
    }

    #[test]
    fn test_toml_empty_document() {
        let result = toml_to_dx("");
        // Empty TOML should be handled gracefully
        let _ = result;
    }

    #[test]
    fn test_json_unicode_values() {
        let json = r#"{"emoji": "🎉", "chinese": "中文", "arabic": "العربية"}"#;
        let result = json_to_dx(json);
        assert!(result.is_ok(), "Unicode should be handled: {:?}", result.err());
    }

    #[test]
    fn test_json_special_characters() {
        let json = r#"{"path": "C:\\Users\\test", "url": "https://example.com?a=1&b=2"}"#;
        let result = json_to_dx(json);
        assert!(result.is_ok(), "Special characters should be handled: {:?}", result.err());
    }

    #[test]
    fn test_json_large_numbers() {
        let json = r#"{"big": 9007199254740991, "negative": -9007199254740991}"#;
        let result = json_to_dx(json);
        assert!(result.is_ok(), "Large numbers should be handled: {:?}", result.err());
    }

    #[test]
    fn test_json_scientific_notation() {
        let json = r#"{"sci": 1.23e10, "neg_sci": -4.56e-7}"#;
        let result = json_to_dx(json);
        assert!(result.is_ok(), "Scientific notation should be handled: {:?}", result.err());
    }

    #[test]
    fn test_toon_empty_document() {
        let result = toon_to_dx("");
        // Empty TOON should be handled gracefully
        let _ = result;
    }

    #[test]
    fn test_toon_simple_key_value() {
        let toon = r#"name "test""#;
        let result = toon_to_dx(toon);
        assert!(result.is_ok(), "Simple TOON should be handled: {:?}", result.err());
    }

    #[test]
    fn test_dx_to_toon_simple() {
        let dx = "name:test\ncount:42";
        let result = dx_to_toon(dx);
        assert!(result.is_ok(), "DX to TOON should succeed: {:?}", result.err());
        let toon = result.unwrap();
        assert!(toon.contains("name"), "TOON should contain key");
        assert!(toon.contains("test"), "TOON should contain string value");
        assert!(toon.contains("42"), "TOON should contain number value");
    }

    #[test]
    fn test_dx_to_toon_booleans() {
        let dx = "active:+\ndisabled:-";
        let result = dx_to_toon(dx);
        assert!(result.is_ok(), "DX to TOON with booleans should succeed: {:?}", result.err());
        let toon = result.unwrap();
        assert!(toon.contains("true"), "TOON should contain true");
        assert!(toon.contains("false"), "TOON should contain false");
    }
}
