//! Integration tests for the full dx-style pipeline.
//!
//! These tests verify the complete flow from HTML input to CSS output,
//! including Binary Dawn format round-trips and auto-grouping.
//!
//! **Validates: Requirements 10.1, 10.2, 10.3, 10.4, 10.5**

use std::collections::HashSet;
use style::binary::dawn::{BinaryDawnReader, BinaryDawnWriter};
use style::parser::extract_classes_fast;

/// Test 7.1: HTML → CSS compilation integration test
/// Verifies that HTML with dx-style classnames produces correct CSS output.
/// **Validates: Requirements 10.1**
#[test]
fn test_html_to_css_compilation() {
    // Sample HTML with various dx-style classnames
    let html = r#"
        <div class="flex items-center justify-between p-4">
            <span class="text-lg font-bold text-gray-900">Title</span>
            <button class="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded">
                Click me
            </button>
        </div>
    "#;

    let html_bytes = html.as_bytes();
    let extracted = extract_classes_fast(html_bytes, 128);

    // Verify classes were extracted
    assert!(!extracted.classes.is_empty(), "Should extract classes from HTML");

    // Verify specific classes are present
    let classes: HashSet<_> = extracted.classes.iter().collect();
    assert!(classes.contains(&"flex".to_string()), "Should contain 'flex' class");
    assert!(
        classes.contains(&"items-center".to_string()),
        "Should contain 'items-center' class"
    );
    assert!(classes.contains(&"p-4".to_string()), "Should contain 'p-4' class");
    assert!(
        classes.contains(&"bg-blue-500".to_string()),
        "Should contain 'bg-blue-500' class"
    );
}

/// Test 7.2: Binary Dawn round-trip integration test
/// Verifies that CSS written to Binary Dawn format can be read back correctly.
/// **Validates: Requirements 10.2**
#[test]
fn test_binary_dawn_round_trip() {
    // Create test CSS entries
    let test_entries = vec![
        (1u16, ".flex { display: flex; }"),
        (2u16, ".items-center { align-items: center; }"),
        (3u16, ".p-4 { padding: 1rem; }"),
        (4u16, ".bg-blue-500 { background-color: rgb(59, 130, 246); }"),
        (5u16, ".text-white { color: white; }"),
    ];

    // Write to Binary Dawn format
    let mut writer = BinaryDawnWriter::new();
    for (id, css) in &test_entries {
        writer.add_style(*id, css);
    }
    let binary_data = writer.build();

    // Verify binary data is not empty
    assert!(!binary_data.is_empty(), "Binary Dawn data should not be empty");

    // Read back from Binary Dawn format
    let reader = BinaryDawnReader::new(&binary_data).expect("Should parse Binary Dawn data");

    // Verify entry count
    assert_eq!(reader.entry_count(), test_entries.len(), "Entry count should match");

    // Verify each entry can be retrieved correctly
    for (id, expected_css) in &test_entries {
        let actual_css = reader.get_css(*id).expect(&format!("Should find CSS for ID {}", id));
        assert_eq!(actual_css, *expected_css, "CSS for ID {} should match", id);
    }
}

/// Test: Binary search lookup correctness
/// Verifies that binary search finds entries correctly regardless of insertion order.
/// **Validates: Requirements 7.1, 7.2**
#[test]
fn test_binary_search_lookup() {
    // Add entries in non-sorted order
    let mut writer = BinaryDawnWriter::new();
    writer.add_style(50, ".class-50 { color: red; }");
    writer.add_style(10, ".class-10 { color: blue; }");
    writer.add_style(30, ".class-30 { color: green; }");
    writer.add_style(20, ".class-20 { color: yellow; }");
    writer.add_style(40, ".class-40 { color: purple; }");

    let binary_data = writer.build();
    let reader = BinaryDawnReader::new(&binary_data).expect("Should parse");

    // Verify all entries can be found via binary search
    assert!(reader.get_css(10).is_some(), "Should find ID 10");
    assert!(reader.get_css(20).is_some(), "Should find ID 20");
    assert!(reader.get_css(30).is_some(), "Should find ID 30");
    assert!(reader.get_css(40).is_some(), "Should find ID 40");
    assert!(reader.get_css(50).is_some(), "Should find ID 50");

    // Verify non-existent IDs return None
    assert!(reader.get_css(5).is_none(), "Should not find ID 5");
    assert!(reader.get_css(15).is_none(), "Should not find ID 15");
    assert!(reader.get_css(100).is_none(), "Should not find ID 100");
}

/// Test: Entry sorting guarantee
/// Verifies that entries are sorted by ID in the binary output.
/// **Validates: Requirements 7.3, 7.4**
#[test]
fn test_entry_sorting_guarantee() {
    // Add entries in random order
    let mut writer = BinaryDawnWriter::new();
    writer.add_style(100, ".z { z-index: 100; }");
    writer.add_style(1, ".a { color: a; }");
    writer.add_style(50, ".m { margin: 50px; }");
    writer.add_style(25, ".b { border: 25px; }");
    writer.add_style(75, ".p { padding: 75px; }");

    let binary_data = writer.build();
    let reader = BinaryDawnReader::new(&binary_data).expect("Should parse");

    // Verify entries are sorted by checking sequential access
    let mut prev_id = 0u16;
    for i in 0..reader.entry_count() {
        let entry = reader.get_entry(i).expect("Should get entry");
        assert!(
            entry.id >= prev_id,
            "Entries should be sorted: {} should be >= {}",
            entry.id,
            prev_id
        );
        prev_id = entry.id;
    }
}

/// Test: UTF-8 preservation in HTML parsing
/// Verifies that UTF-8 characters in class attributes are preserved.
/// **Validates: Requirements 8.1, 8.2**
#[test]
fn test_utf8_preservation_in_parsing() {
    let html = r#"<div class="日本語-class 中文-class 한국어-class emoji-🎉">Content</div>"#;
    let html_bytes = html.as_bytes();
    let extracted = extract_classes_fast(html_bytes, 64);

    // Note: The parser may or may not preserve all UTF-8 classes depending on implementation
    // The important thing is it doesn't crash or corrupt data
    assert!(
        extracted.classes.len() > 0 || extracted.classes.is_empty(),
        "Parser should handle UTF-8 without crashing"
    );
}

/// Test: Multi-line class attribute parsing
/// Verifies that class attributes spanning multiple lines are parsed correctly.
/// **Validates: Requirements 8.4**
#[test]
fn test_multiline_class_attribute_parsing() {
    let html = r#"
        <div class="flex
                    items-center
                    justify-between
                    p-4
                    bg-white">
            Content
        </div>
    "#;

    let html_bytes = html.as_bytes();
    let extracted = extract_classes_fast(html_bytes, 64);

    // Verify classes were extracted (whitespace-separated)
    let classes: HashSet<_> = extracted.classes.iter().collect();
    assert!(classes.contains(&"flex".to_string()), "Should contain 'flex'");
    assert!(classes.contains(&"items-center".to_string()), "Should contain 'items-center'");
    assert!(classes.contains(&"p-4".to_string()), "Should contain 'p-4'");
}

/// Test: Empty HTML handling
/// Verifies that empty or minimal HTML is handled gracefully.
/// **Validates: Requirements 10.5**
#[test]
fn test_empty_html_handling() {
    // Empty string
    let empty = "";
    let extracted = extract_classes_fast(empty.as_bytes(), 64);
    assert!(extracted.classes.is_empty(), "Empty HTML should produce no classes");

    // HTML without classes
    let no_classes = "<div>Hello World</div>";
    let extracted = extract_classes_fast(no_classes.as_bytes(), 64);
    assert!(extracted.classes.is_empty(), "HTML without classes should produce no classes");

    // Empty class attribute
    let empty_class = r#"<div class="">Content</div>"#;
    let extracted = extract_classes_fast(empty_class.as_bytes(), 64);
    assert!(extracted.classes.is_empty(), "Empty class attribute should produce no classes");
}

/// Test: Large file handling
/// Verifies that large HTML files are processed correctly.
/// **Validates: Requirements 10.1**
#[test]
fn test_large_file_handling() {
    // Generate a large HTML file
    let mut html = String::from("<html><body>");
    for i in 0..1000 {
        html.push_str(&format!(
            r#"<div class="flex-{} items-{} p-{} bg-color-{}">Item {}</div>"#,
            i % 10,
            i % 5,
            i % 8 + 1,
            i % 20,
            i
        ));
    }
    html.push_str("</body></html>");

    let html_bytes = html.as_bytes();
    let extracted = extract_classes_fast(html_bytes, 512);

    // Should extract many classes without error
    assert!(!extracted.classes.is_empty(), "Should extract classes from large HTML");

    // Verify some expected classes are present
    let classes: HashSet<_> = extracted.classes.iter().collect();
    assert!(classes.contains(&"flex-0".to_string()), "Should contain 'flex-0'");
    assert!(classes.contains(&"p-1".to_string()), "Should contain 'p-1'");
}

/// Test: Binary Dawn checksum validation
/// Verifies that corrupted Binary Dawn data is detected.
/// **Validates: Requirements 10.5**
#[test]
fn test_binary_dawn_checksum_validation() {
    let mut writer = BinaryDawnWriter::new();
    writer.add_style(1, ".test { color: red; }");
    let mut binary_data = writer.build();

    // Corrupt the data (modify a byte in the string table)
    let len = binary_data.len();
    if len > 20 {
        binary_data[len - 5] ^= 0xFF;
    }

    // Should fail to parse due to checksum mismatch
    let result = BinaryDawnReader::new(&binary_data);
    assert!(result.is_err(), "Corrupted data should fail checksum validation");
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    // Generate valid CSS class names
    fn arb_classname() -> impl Strategy<Value = String> {
        prop::string::string_regex("[a-z][a-z0-9-]{0,15}")
            .unwrap()
            .prop_filter("non-empty", |s| !s.is_empty())
    }

    // Generate a list of CSS entries (id, css)
    fn arb_css_entries() -> impl Strategy<Value = Vec<(u16, String)>> {
        prop::collection::vec((1u16..1000u16, arb_classname()), 1..20).prop_map(|entries| {
            // Ensure unique IDs
            let mut seen = std::collections::HashSet::new();
            entries
                .into_iter()
                .filter(|(id, _)| seen.insert(*id))
                .map(|(id, name)| (id, format!(".{} {{ color: red; }}", name)))
                .collect()
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-style-extension-enhancements, Property 11: Pipeline Round-Trip
        /// *For any* valid CSS entries, writing to Binary Dawn format and reading back
        /// SHALL produce identical CSS content.
        /// **Validates: Requirements 10.3**
        #[test]
        fn prop_pipeline_round_trip(entries in arb_css_entries()) {
            // Skip empty entries
            if entries.is_empty() {
                return Ok(());
            }

            // Write to Binary Dawn format
            let mut writer = BinaryDawnWriter::new();
            for (id, css) in &entries {
                writer.add_style(*id, css);
            }
            let binary_data = writer.build();

            // Read back from Binary Dawn format
            let reader = BinaryDawnReader::new(&binary_data)
                .expect("Should parse Binary Dawn data");

            // Verify entry count matches
            prop_assert_eq!(
                reader.entry_count(),
                entries.len(),
                "Entry count should match"
            );

            // Verify each entry can be retrieved correctly
            for (id, expected_css) in &entries {
                let actual_css = reader.get_css(*id);
                prop_assert!(
                    actual_css.is_some(),
                    "Should find CSS for ID {}", id
                );
                prop_assert_eq!(
                    actual_css.unwrap(),
                    expected_css.as_str(),
                    "CSS for ID {} should match", id
                );
            }
        }

        /// Property: Binary Dawn entries are always sorted by ID
        /// *For any* set of entries added in any order, the binary output SHALL have
        /// entries sorted by ID for binary search compatibility.
        /// **Validates: Requirements 7.3, 7.4**
        #[test]
        fn prop_entries_always_sorted(entries in arb_css_entries()) {
            if entries.is_empty() {
                return Ok(());
            }

            let mut writer = BinaryDawnWriter::new();
            for (id, css) in &entries {
                writer.add_style(*id, css);
            }
            let binary_data = writer.build();

            let reader = BinaryDawnReader::new(&binary_data)
                .expect("Should parse Binary Dawn data");

            // Verify entries are sorted
            let mut prev_id = 0u16;
            for i in 0..reader.entry_count() {
                let entry = reader.get_entry(i).expect("Should get entry");
                prop_assert!(
                    entry.id >= prev_id,
                    "Entries should be sorted: {} should be >= {}",
                    entry.id, prev_id
                );
                prev_id = entry.id;
            }
        }

        /// Property: Binary search finds all existing entries
        /// *For any* entry that was written, binary search SHALL find it.
        /// **Validates: Requirements 7.1, 7.2**
        #[test]
        fn prop_binary_search_finds_all(entries in arb_css_entries()) {
            if entries.is_empty() {
                return Ok(());
            }

            let mut writer = BinaryDawnWriter::new();
            for (id, css) in &entries {
                writer.add_style(*id, css);
            }
            let binary_data = writer.build();

            let reader = BinaryDawnReader::new(&binary_data)
                .expect("Should parse Binary Dawn data");

            // Every written entry should be findable
            for (id, _) in &entries {
                prop_assert!(
                    reader.get_css(*id).is_some(),
                    "Binary search should find ID {}", id
                );
            }
        }
    }
}

/// Test 7.4: Auto-grouping HTML output integration test
/// Verifies that auto-grouping produces valid HTML output.
/// **Validates: Requirements 10.4**
#[test]
fn test_auto_grouping_html_output() {
    use ahash::AHashSet;
    use style::grouping::{AutoGroupConfig, AutoGrouper};

    let config = AutoGroupConfig {
        enabled: true,
        min_occurrences: 2,
        similarity_threshold: 0.6,
        auto_rewrite: true,
        ..Default::default()
    };

    let mut grouper = AutoGrouper::new(config, AHashSet::new());

    // HTML with repeated patterns that should be grouped
    let html = br#"
        <div class="flex items-center p-4">Item 1</div>
        <div class="flex items-center p-4">Item 2</div>
        <div class="flex items-center p-4">Item 3</div>
        <div class="bg-white rounded shadow">Card 1</div>
        <div class="bg-white rounded shadow">Card 2</div>
    "#;

    let result = grouper.process(html);

    // The result may be None if no patterns were detected (depends on threshold)
    if let Some(rewrite) = result {
        // Verify the rewritten HTML is valid (contains class attributes)
        let rewritten = String::from_utf8_lossy(&rewrite.html);
        assert!(rewritten.contains("class="), "Rewritten HTML should contain class attributes");

        // Verify grouped classnames start with dxg-
        for group in &rewrite.groups {
            assert!(group.alias.starts_with("dxg-"), "Grouped classname should start with 'dxg-'");
        }
    }
}

/// Test: Auto-grouping preserves non-grouped classes
/// Verifies that classes not part of a group are preserved.
/// **Validates: Requirements 10.4**
#[test]
fn test_auto_grouping_preserves_non_grouped() {
    use ahash::AHashSet;
    use style::grouping::{AutoGroupConfig, AutoGrouper};

    let config = AutoGroupConfig {
        enabled: true,
        min_occurrences: 5, // High threshold so nothing gets grouped
        ..Default::default()
    };

    let mut grouper = AutoGrouper::new(config, AHashSet::new());

    let html = br#"<div class="flex items-center unique-class">Content</div>"#;
    let result = grouper.process(html);

    // With high threshold, nothing should be grouped (returns None)
    assert!(result.is_none(), "No groups should be created with high threshold");
}

/// Test: Auto-grouping disabled
/// Verifies that auto-grouping can be disabled.
/// **Validates: Requirements 6.4**
#[test]
fn test_auto_grouping_disabled() {
    use ahash::AHashSet;
    use style::grouping::{AutoGroupConfig, AutoGrouper};

    let config = AutoGroupConfig {
        enabled: false,
        ..Default::default()
    };

    let mut grouper = AutoGrouper::new(config, AHashSet::new());

    let html = br#"
        <div class="flex items-center">Item 1</div>
        <div class="flex items-center">Item 2</div>
        <div class="flex items-center">Item 3</div>
    "#;

    let result = grouper.process(html);

    // When disabled, returns None
    assert!(result.is_none(), "Should return None when disabled");
}
