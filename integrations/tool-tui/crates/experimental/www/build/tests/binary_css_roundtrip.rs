//! Property tests for Binary CSS round trip
//!
//! Feature: dx-www-production-ready, Property 4: Binary CSS Round Trip
//!
//! This property test verifies that CSS compilation to Binary Dawn format and
//! decoding produces equivalent styles. The property tests that:
//! 1. For any valid CSS input, compiling to Binary Dawn then decoding produces equivalent styles
//! 2. The round trip preserves all CSS rules
//! 3. The round trip preserves CSS text exactly
//! 4. Multiple CSS entries round trip correctly

use proptest::prelude::*;
use std::collections::HashMap;

/// Strategy for generating arbitrary CSS class names
fn arbitrary_css_class() -> impl Strategy<Value = String> {
    prop::string::string_regex("[a-z][a-z0-9-]{2,15}").unwrap()
}

/// Strategy for generating arbitrary CSS property names
fn arbitrary_css_property() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "display",
        "color",
        "background",
        "padding",
        "margin",
        "font-size",
        "width",
        "height",
        "border",
        "flex",
        "grid",
        "position",
        "text-align",
        "opacity",
        "z-index",
    ])
    .prop_map(|s| s.to_string())
}

/// Strategy for generating arbitrary CSS property values
fn arbitrary_css_value() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "red", "blue", "white", "black", "flex", "block", "inline", "1rem", "10px", "100%", "auto",
        "center", "left", "right", "0", "1", "absolute", "relative",
    ])
    .prop_map(|s| s.to_string())
}

/// Strategy for generating a single CSS rule
fn arbitrary_css_rule() -> impl Strategy<Value = String> {
    (arbitrary_css_class(), arbitrary_css_property(), arbitrary_css_value())
        .prop_map(|(class, prop, value)| format!(".{} {{ {}: {}; }}", class, prop, value))
}

/// Strategy for generating multiple CSS rules (1-20 rules)
fn arbitrary_css_rules() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(arbitrary_css_rule(), 1..=20)
}

/// Strategy for generating CSS entries with unique IDs
fn arbitrary_css_entries() -> impl Strategy<Value = Vec<(u16, String)>> {
    arbitrary_css_rules().prop_flat_map(|css_list| {
        let len = css_list.len();
        // Generate unique IDs by sampling without replacement
        prop::sample::subsequence((0u16..10000).collect::<Vec<_>>(), len)
            .prop_map(move |ids| ids.into_iter().zip(css_list.iter().cloned()).collect::<Vec<_>>())
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 4a: Binary CSS Round Trip Preserves All Entries
    ///
    /// For any valid CSS input, compiling to Binary Dawn format then decoding
    /// should produce equivalent styles with all entries preserved.
    ///
    /// **Validates: Requirements 1.2**
    // Feature: dx-www-production-ready, Property 4: Binary CSS Round Trip
    #[test]
    fn binary_css_roundtrip_preserves_all_entries(entries in arbitrary_css_entries()) {
        use style::binary::dawn::{BinaryDawnReader, BinaryDawnWriter};

        // Build Binary Dawn CSS
        let mut writer = BinaryDawnWriter::new();
        for (id, css) in &entries {
            writer.add_style(*id, css);
        }
        let binary_data = writer.build();

        // Decode Binary Dawn CSS
        let reader = BinaryDawnReader::new(&binary_data)
            .expect("Should successfully decode Binary Dawn CSS");

        // Verify all entries are preserved
        prop_assert_eq!(
            reader.entry_count(),
            entries.len(),
            "Round trip should preserve entry count"
        );

        // Verify each entry can be retrieved
        for (id, expected_css) in &entries {
            let retrieved_css = reader.get_css(*id);
            prop_assert_eq!(
                retrieved_css,
                Some(expected_css.as_str()),
                "Round trip should preserve CSS for ID {}",
                id
            );
        }
    }

    /// Property 4b: Binary CSS Round Trip Preserves CSS Text Exactly
    ///
    /// For any CSS text, the round trip through Binary Dawn format should
    /// preserve the exact CSS text byte-for-byte.
    ///
    /// **Validates: Requirements 1.2**
    // Feature: dx-www-production-ready, Property 4: Binary CSS Round Trip
    #[test]
    fn binary_css_roundtrip_preserves_css_text_exactly(css_text in arbitrary_css_rule()) {
        use style::binary::dawn::{BinaryDawnReader, BinaryDawnWriter};

        let id = 42u16;

        // Build Binary Dawn CSS
        let mut writer = BinaryDawnWriter::new();
        writer.add_style(id, &css_text);
        let binary_data = writer.build();

        // Decode Binary Dawn CSS
        let reader = BinaryDawnReader::new(&binary_data)
            .expect("Should successfully decode Binary Dawn CSS");

        // Verify exact CSS text preservation
        let retrieved_css = reader.get_css(id)
            .expect("Should retrieve CSS for ID");

        prop_assert_eq!(
            retrieved_css,
            css_text.as_str(),
            "Round trip should preserve CSS text exactly"
        );

        // Verify byte-for-byte equality
        prop_assert_eq!(
            retrieved_css.as_bytes(),
            css_text.as_bytes(),
            "Round trip should preserve CSS bytes exactly"
        );
    }

    /// Property 4c: Binary CSS Round Trip Preserves Multiple Rules
    ///
    /// For any set of CSS rules, the round trip should preserve all rules
    /// and allow retrieval by ID.
    ///
    /// **Validates: Requirements 1.2**
    // Feature: dx-www-production-ready, Property 4: Binary CSS Round Trip
    #[test]
    fn binary_css_roundtrip_preserves_multiple_rules(
        css_rules in prop::collection::vec(arbitrary_css_rule(), 1..=50)
    ) {
        use style::binary::dawn::{BinaryDawnReader, BinaryDawnWriter};

        // Build Binary Dawn CSS with sequential IDs
        let mut writer = BinaryDawnWriter::new();
        let mut id_to_css: HashMap<u16, String> = HashMap::new();

        for (idx, css) in css_rules.iter().enumerate() {
            let id = idx as u16;
            writer.add_style(id, css);
            id_to_css.insert(id, css.clone());
        }

        let binary_data = writer.build();

        // Decode Binary Dawn CSS
        let reader = BinaryDawnReader::new(&binary_data)
            .expect("Should successfully decode Binary Dawn CSS");

        // Verify all rules are preserved
        prop_assert_eq!(
            reader.entry_count(),
            css_rules.len(),
            "Round trip should preserve all rules"
        );

        // Verify each rule can be retrieved by ID
        for (id, expected_css) in &id_to_css {
            let retrieved_css = reader.get_css(*id);
            prop_assert_eq!(
                retrieved_css,
                Some(expected_css.as_str()),
                "Round trip should preserve CSS rule for ID {}",
                id
            );
        }
    }

    /// Property 4d: Binary CSS Round Trip Handles Empty Input
    ///
    /// For empty CSS input (no styles), the round trip should produce
    /// a valid Binary Dawn file with zero entries.
    ///
    /// **Validates: Requirements 1.2**
    // Feature: dx-www-production-ready, Property 4: Binary CSS Round Trip
    #[test]
    fn binary_css_roundtrip_handles_empty_input(_dummy in 0..1u8) {
        use style::binary::dawn::{BinaryDawnReader, BinaryDawnWriter};

        // Build empty Binary Dawn CSS
        let writer = BinaryDawnWriter::new();
        let binary_data = writer.build();

        // Decode Binary Dawn CSS
        let reader = BinaryDawnReader::new(&binary_data)
            .expect("Should successfully decode empty Binary Dawn CSS");

        // Verify empty file is valid
        prop_assert_eq!(
            reader.entry_count(),
            0,
            "Empty input should produce zero entries"
        );
    }

    /// Property 4e: Binary CSS Round Trip Preserves Order Independence
    ///
    /// For any set of CSS entries added in any order, the round trip should
    /// allow retrieval of all entries regardless of insertion order.
    ///
    /// **Validates: Requirements 1.2**
    // Feature: dx-www-production-ready, Property 4: Binary CSS Round Trip
    #[test]
    fn binary_css_roundtrip_order_independent(entries in arbitrary_css_entries()) {
        use style::binary::dawn::{BinaryDawnReader, BinaryDawnWriter};

        if entries.is_empty() {
            return Ok(());
        }

        // Build Binary Dawn CSS with entries in given order
        let mut writer = BinaryDawnWriter::new();
        for (id, css) in &entries {
            writer.add_style(*id, css);
        }
        let binary_data = writer.build();

        // Decode Binary Dawn CSS
        let reader = BinaryDawnReader::new(&binary_data)
            .expect("Should successfully decode Binary Dawn CSS");

        // Verify all entries are retrievable regardless of insertion order
        for (id, expected_css) in &entries {
            let retrieved_css = reader.get_css(*id);
            prop_assert_eq!(
                retrieved_css,
                Some(expected_css.as_str()),
                "Should retrieve CSS for ID {} regardless of insertion order",
                id
            );
        }
    }

    /// Property 4f: Binary CSS Round Trip Preserves Special Characters
    ///
    /// For CSS containing special characters (quotes, braces, semicolons),
    /// the round trip should preserve them exactly.
    ///
    /// **Validates: Requirements 1.2**
    // Feature: dx-www-production-ready, Property 4: Binary CSS Round Trip
    #[test]
    fn binary_css_roundtrip_preserves_special_characters(
        class_name in arbitrary_css_class(),
        property in arbitrary_css_property(),
    ) {
        use style::binary::dawn::{BinaryDawnReader, BinaryDawnWriter};

        // Create CSS with special characters
        let css_with_special = format!(
            ".{} {{ {}: \"value with 'quotes' and {{braces}}; and; semicolons\"; }}",
            class_name, property
        );

        let id = 1u16;

        // Build Binary Dawn CSS
        let mut writer = BinaryDawnWriter::new();
        writer.add_style(id, &css_with_special);
        let binary_data = writer.build();

        // Decode Binary Dawn CSS
        let reader = BinaryDawnReader::new(&binary_data)
            .expect("Should successfully decode Binary Dawn CSS");

        // Verify special characters are preserved
        let retrieved_css = reader.get_css(id)
            .expect("Should retrieve CSS with special characters");

        prop_assert_eq!(
            retrieved_css,
            css_with_special.as_str(),
            "Round trip should preserve special characters exactly"
        );
    }

    /// Property 4g: Binary CSS Round Trip Preserves Whitespace
    ///
    /// For CSS with various whitespace patterns, the round trip should
    /// preserve the whitespace exactly as written.
    ///
    /// **Validates: Requirements 1.2**
    // Feature: dx-www-production-ready, Property 4: Binary CSS Round Trip
    #[test]
    fn binary_css_roundtrip_preserves_whitespace(
        class_name in arbitrary_css_class(),
        spaces in 0..10usize,
    ) {
        use style::binary::dawn::{BinaryDawnWriter, BinaryDawnReader};

        // Create CSS with specific whitespace pattern
        let whitespace = " ".repeat(spaces);
        let css_with_whitespace = format!(
            ".{}{}{{{}color:{}red;{}}}",
            class_name, whitespace, whitespace, whitespace, whitespace
        );

        let id = 1u16;

        // Build Binary Dawn CSS
        let mut writer = BinaryDawnWriter::new();
        writer.add_style(id, &css_with_whitespace);
        let binary_data = writer.build();

        // Decode Binary Dawn CSS
        let reader = BinaryDawnReader::new(&binary_data)
            .expect("Should successfully decode Binary Dawn CSS");

        // Verify whitespace is preserved
        let retrieved_css = reader.get_css(id)
            .expect("Should retrieve CSS with whitespace");

        prop_assert_eq!(
            retrieved_css,
            css_with_whitespace.as_str(),
            "Round trip should preserve whitespace exactly"
        );
    }

    /// Property 4h: Binary CSS Round Trip Handles Large CSS
    ///
    /// For large CSS strings (up to 1000 characters), the round trip should
    /// preserve the entire content.
    ///
    /// **Validates: Requirements 1.2**
    // Feature: dx-www-production-ready, Property 4: Binary CSS Round Trip
    #[test]
    fn binary_css_roundtrip_handles_large_css(
        rules in prop::collection::vec(arbitrary_css_rule(), 10..=50)
    ) {
        use style::binary::dawn::{BinaryDawnReader, BinaryDawnWriter};

        // Create large CSS by concatenating multiple rules
        let large_css = rules.join(" ");

        let id = 1u16;

        // Build Binary Dawn CSS
        let mut writer = BinaryDawnWriter::new();
        writer.add_style(id, &large_css);
        let binary_data = writer.build();

        // Decode Binary Dawn CSS
        let reader = BinaryDawnReader::new(&binary_data)
            .expect("Should successfully decode Binary Dawn CSS with large content");

        // Verify large CSS is preserved
        let retrieved_css = reader.get_css(id)
            .expect("Should retrieve large CSS");

        prop_assert_eq!(
            retrieved_css,
            large_css.as_str(),
            "Round trip should preserve large CSS exactly"
        );

        prop_assert_eq!(
            retrieved_css.len(),
            large_css.len(),
            "Round trip should preserve CSS length"
        );
    }
}
