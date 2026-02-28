//! Task 6.4: Property Tests for Prefix Elimination Parsing
//!
//! Property-based tests for prefix elimination functionality:
//! - Property 24: Prefix Marker Recognition
//! - Property 25: Multiple Prefix Support
//! - Property 26: Suffix Marker Recognition
//! - Property 27: Prefix Application
//! - Property 28: Suffix Application
//!
//! Validates: Requirements 6.1, 6.2, 6.3, 6.4, 6.5

use proptest::prelude::*;
use serializer::llm::parser::LlmParser;
use serializer::llm::types::DxLlmValue;

// ============================================================================
// Property 24: Prefix Marker Recognition
// ============================================================================

/// **Property 24: Prefix Marker Recognition**
///
/// *For any* table with `@prefix` markers before the data, parsing should
/// recognize and extract all prefix strings.
///
/// **Validates: Requirements 6.1**
#[cfg(test)]
mod property_24_prefix_marker_recognition {
    use super::*;

    // Strategy to generate valid prefix strings
    fn arb_prefix() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-zA-Z0-9/_.-]{1,20}").expect("valid regex")
    }

    // Strategy to generate table with single prefix marker
    fn arb_table_with_prefix() -> impl Strategy<Value = String> {
        (
            arb_prefix(),
            prop::collection::vec(prop::string::string_regex("[a-zA-Z]{3,10}").unwrap(), 1..=3),
        )
            .prop_map(|(prefix, _names)| format!("table:2(id name)@{}[1 Alice, 2 Bob]", prefix))
    }

    proptest! {
        #[test]
        fn prefix_marker_is_recognized(input in arb_table_with_prefix()) {
            // Parse the input
            let result = LlmParser::parse(&input);

            // Should parse successfully
            prop_assert!(result.is_ok(), "Failed to parse table with prefix marker: {:?}", result.err());

            let doc = result.unwrap();

            // Should have one section
            prop_assert_eq!(doc.sections.len(), 1, "Should have exactly one section");
        }
    }
}

// ============================================================================
// Property 25: Multiple Prefix Support
// ============================================================================

/// **Property 25: Multiple Prefix Support**
///
/// *For any* table with multiple `@prefix` markers, parsing should recognize
/// and extract all prefixes in order.
///
/// **Validates: Requirements 6.2**
#[cfg(test)]
mod property_25_multiple_prefix_support {
    use super::*;

    // Strategy to generate valid prefix strings
    fn arb_prefix() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-zA-Z0-9/_.-]{1,15}").expect("valid regex")
    }

    // Strategy to generate table with multiple prefix markers
    fn arb_table_with_multiple_prefixes() -> impl Strategy<Value = (String, Vec<String>)> {
        prop::collection::vec(arb_prefix(), 2..=4).prop_map(|prefixes| {
            let prefix_markers =
                prefixes.iter().map(|p| format!("@{}", p)).collect::<Vec<_>>().join(" ");

            let input = format!("table:2(id name){}[1 Alice, 2 Bob]", prefix_markers);

            (input, prefixes)
        })
    }

    proptest! {
        #[test]
        fn multiple_prefixes_are_recognized((input, _prefixes) in arb_table_with_multiple_prefixes()) {
            // Parse the input
            let result = LlmParser::parse(&input);

            // Should parse successfully
            prop_assert!(result.is_ok(), "Failed to parse table with multiple prefixes: {:?}", result.err());

            let doc = result.unwrap();

            // Should have one section
            prop_assert_eq!(doc.sections.len(), 1, "Should have exactly one section");
        }
    }
}

// ============================================================================
// Property 26: Suffix Marker Recognition
// ============================================================================

/// **Property 26: Suffix Marker Recognition**
///
/// *For any* table with `@@suffix` markers, parsing should recognize and
/// extract all suffix strings.
///
/// **Validates: Requirements 6.3**
#[cfg(test)]
mod property_26_suffix_marker_recognition {
    use super::*;

    // Strategy to generate valid suffix strings
    fn arb_suffix() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-zA-Z0-9._-]{1,20}").expect("valid regex")
    }

    // Strategy to generate table with suffix marker
    fn arb_table_with_suffix() -> impl Strategy<Value = String> {
        arb_suffix().prop_map(|suffix| format!("table:2(id email)@@{}[1 alice, 2 bob]", suffix))
    }

    proptest! {
        #[test]
        fn suffix_marker_is_recognized(input in arb_table_with_suffix()) {
            // Parse the input
            let result = LlmParser::parse(&input);

            // Should parse successfully
            prop_assert!(result.is_ok(), "Failed to parse table with suffix marker: {:?}", result.err());

            let doc = result.unwrap();

            // Should have one section
            prop_assert_eq!(doc.sections.len(), 1, "Should have exactly one section");
        }
    }
}

// ============================================================================
// Property 27: Prefix Application
// ============================================================================

/// **Property 27: Prefix Application**
///
/// *For any* table with prefix markers, parsing should prepend the prefixes
/// to appropriate column values in the resulting DxSection.
///
/// **Validates: Requirements 6.4**
#[cfg(test)]
mod property_27_prefix_application {
    use super::*;

    // Strategy to generate valid prefix strings
    fn arb_prefix() -> impl Strategy<Value = String> {
        prop::string::string_regex("/api/[a-z]{2,8}/").expect("valid regex")
    }

    // Strategy to generate path-like values
    fn arb_path() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-z]{3,10}").expect("valid regex")
    }

    // Strategy to generate table with prefix and path column
    fn arb_table_with_prefix_and_path() -> impl Strategy<Value = (String, String, Vec<String>)> {
        (arb_prefix(), prop::collection::vec(arb_path(), 2..=4)).prop_map(|(prefix, paths)| {
            let rows = paths
                .iter()
                .enumerate()
                .map(|(i, path)| format!("{} {}", i + 1, path))
                .collect::<Vec<_>>()
                .join(", ");

            let input = format!("table:2(id endpoint)@{}[{}]", prefix, rows);

            (input, prefix, paths)
        })
    }

    proptest! {
        #[test]
        fn prefix_is_applied_to_appropriate_columns((input, prefix, paths) in arb_table_with_prefix_and_path()) {
            // Parse the input
            let result = LlmParser::parse(&input);

            // Should parse successfully
            prop_assert!(result.is_ok(), "Failed to parse table with prefix: {:?}", result.err());

            let doc = result.unwrap();

            // Should have one section
            prop_assert_eq!(doc.sections.len(), 1, "Should have exactly one section");

            // Get the section
            let section = doc.sections.values().next().unwrap();

            // Should have correct number of rows
            prop_assert_eq!(section.rows.len(), paths.len(), "Should have correct number of rows");

            // Check that prefix was applied to endpoint column (column index 1)
            for (i, row) in section.rows.iter().enumerate() {
                if let Some(DxLlmValue::Str(endpoint)) = row.get(1) {
                    // The endpoint should start with the prefix
                    prop_assert!(
                        endpoint.starts_with(&prefix),
                        "Row {} endpoint '{}' should start with prefix '{}'",
                        i, endpoint, prefix
                    );

                    // The endpoint should end with the original path
                    prop_assert!(
                        endpoint.ends_with(&paths[i]),
                        "Row {} endpoint '{}' should end with path '{}'",
                        i, endpoint, paths[i]
                    );
                }
            }
        }
    }
}

// ============================================================================
// Property 28: Suffix Application
// ============================================================================

/// **Property 28: Suffix Application**
///
/// *For any* table with suffix markers, parsing should append the suffixes
/// to appropriate column values in the resulting DxSection.
///
/// **Validates: Requirements 6.5**
#[cfg(test)]
mod property_28_suffix_application {
    use super::*;

    // Strategy to generate valid suffix strings (domain-like)
    fn arb_suffix() -> impl Strategy<Value = String> {
        prop::string::string_regex("@[a-z]{3,10}\\.(com|org|net)").expect("valid regex")
    }

    // Strategy to generate email username-like values
    fn arb_username() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-z]{3,10}").expect("valid regex")
    }

    // Strategy to generate table with suffix and email column
    fn arb_table_with_suffix_and_email() -> impl Strategy<Value = (String, String, Vec<String>)> {
        (arb_suffix(), prop::collection::vec(arb_username(), 2..=4)).prop_map(
            |(suffix, usernames)| {
                let rows = usernames
                    .iter()
                    .enumerate()
                    .map(|(i, username)| format!("{} {}", i + 1, username))
                    .collect::<Vec<_>>()
                    .join(", ");

                let input = format!("table:2(id email)@@{}[{}]", suffix, rows);

                (input, suffix, usernames)
            },
        )
    }

    proptest! {
        #[test]
        fn suffix_is_applied_to_appropriate_columns((input, suffix, usernames) in arb_table_with_suffix_and_email()) {
            // Parse the input
            let result = LlmParser::parse(&input);

            // Should parse successfully
            prop_assert!(result.is_ok(), "Failed to parse table with suffix: {:?}", result.err());

            let doc = result.unwrap();

            // Should have one section
            prop_assert_eq!(doc.sections.len(), 1, "Should have exactly one section");

            // Get the section
            let section = doc.sections.values().next().unwrap();

            // Should have correct number of rows
            prop_assert_eq!(section.rows.len(), usernames.len(), "Should have correct number of rows");

            // Check that suffix was applied to email column (column index 1)
            for (i, row) in section.rows.iter().enumerate() {
                if let Some(DxLlmValue::Str(email)) = row.get(1) {
                    // The email should end with the suffix
                    prop_assert!(
                        email.ends_with(&suffix),
                        "Row {} email '{}' should end with suffix '{}'",
                        i, email, suffix
                    );

                    // The email should start with the original username
                    prop_assert!(
                        email.starts_with(&usernames[i]),
                        "Row {} email '{}' should start with username '{}'",
                        i, email, usernames[i]
                    );
                }
            }
        }
    }
}
