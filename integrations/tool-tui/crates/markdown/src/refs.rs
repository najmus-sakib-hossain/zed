//! Reference graph for URL/entity deduplication.
//!
//! The reference graph allows defining URLs and repeated entities once,
//! then referencing them by short keys throughout the document.
//! This significantly reduces token usage in LLM format.

use std::collections::HashMap;

use crate::types::{DxmDocument, DxmNode, InlineNode};

/// Reference graph for URL/entity deduplication.
///
/// Stores key-value mappings and tracks usage counts for optimization.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReferenceGraph {
    /// Key to value mappings
    refs: HashMap<String, String>,
    /// Usage counts for each reference (for optimization)
    usage_counts: HashMap<String, usize>,
    /// Counter for auto-generated keys
    next_key_index: usize,
}

impl ReferenceGraph {
    /// Create a new empty reference graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a reference graph from existing mappings.
    pub fn from_refs(refs: HashMap<String, String>) -> Self {
        Self {
            refs,
            usage_counts: HashMap::new(),
            next_key_index: 0,
        }
    }

    /// Define a reference.
    ///
    /// # Arguments
    /// * `key` - The reference key (e.g., "doc", "repo")
    /// * `value` - The value to store (e.g., URL, repeated text)
    pub fn define(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        self.refs.insert(key.clone(), value.into());
        self.usage_counts.entry(key).or_insert(0);
    }

    /// Resolve a reference by key.
    ///
    /// Returns `None` if the key is not defined.
    pub fn resolve(&self, key: &str) -> Option<&str> {
        self.refs.get(key).map(|s| s.as_str())
    }

    /// Check if a reference key is defined.
    pub fn contains(&self, key: &str) -> bool {
        self.refs.contains_key(key)
    }

    /// Get all defined reference keys.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.refs.keys().map(|s| s.as_str())
    }

    /// Get all reference definitions.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.refs.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Get the number of defined references.
    pub fn len(&self) -> usize {
        self.refs.len()
    }

    /// Check if the graph is empty.
    pub fn is_empty(&self) -> bool {
        self.refs.is_empty()
    }

    /// Record a usage of a reference.
    pub fn record_usage(&mut self, key: &str) {
        if let Some(count) = self.usage_counts.get_mut(key) {
            *count += 1;
        }
    }

    /// Get the usage count for a reference.
    pub fn usage_count(&self, key: &str) -> usize {
        self.usage_counts.get(key).copied().unwrap_or(0)
    }

    /// Generate the next sequential key.
    ///
    /// Keys follow the pattern: A, B, C, ..., Z, AA, AB, ..., AZ, BA, ...
    fn next_key(&mut self) -> String {
        let key = Self::index_to_key(self.next_key_index);
        self.next_key_index += 1;
        key
    }

    /// Convert an index to a key string.
    ///
    /// 0 -> "A", 1 -> "B", ..., 25 -> "Z", 26 -> "AA", 27 -> "AB", ...
    fn index_to_key(mut index: usize) -> String {
        let mut result = String::new();
        loop {
            let remainder = index % 26;
            result.insert(0, (b'A' + remainder as u8) as char);
            if index < 26 {
                break;
            }
            index = index / 26 - 1;
        }
        result
    }

    /// Auto-generate references for repeated content in a document.
    ///
    /// Scans the document for repeated URLs and text patterns,
    /// creating references for content that appears 2+ times.
    pub fn auto_generate(&mut self, doc: &DxmDocument) {
        // Collect all URLs and their occurrence counts
        let mut url_counts: HashMap<String, usize> = HashMap::new();

        for node in &doc.nodes {
            self.collect_urls_from_node(node, &mut url_counts);
        }

        // Create references for URLs that appear 2+ times
        for (url, count) in url_counts {
            if count >= 2 && !self.has_value(&url) {
                let key = self.next_key();
                self.define(key, url);
            }
        }
    }

    /// Check if a value is already defined (reverse lookup).
    fn has_value(&self, value: &str) -> bool {
        self.refs.values().any(|v| v == value)
    }

    /// Collect URLs from a node recursively.
    fn collect_urls_from_node(&self, node: &DxmNode, counts: &mut HashMap<String, usize>) {
        match node {
            DxmNode::Header(header) => {
                for inline in &header.content {
                    self.collect_urls_from_inline(inline, counts);
                }
            }
            DxmNode::Paragraph(inlines) => {
                for inline in inlines {
                    self.collect_urls_from_inline(inline, counts);
                }
            }
            DxmNode::SemanticBlock(block) => {
                for inline in &block.content {
                    self.collect_urls_from_inline(inline, counts);
                }
            }
            DxmNode::List(list) => {
                for item in &list.items {
                    for inline in &item.content {
                        self.collect_urls_from_inline(inline, counts);
                    }
                    if let Some(nested) = &item.nested {
                        self.collect_urls_from_list_node(nested, counts);
                    }
                }
            }
            DxmNode::CodeBlock(_) | DxmNode::Table(_) | DxmNode::HorizontalRule => {}
        }
    }

    /// Collect URLs from a list node recursively.
    fn collect_urls_from_list_node(
        &self,
        list: &crate::types::ListNode,
        counts: &mut HashMap<String, usize>,
    ) {
        for item in &list.items {
            for inline in &item.content {
                self.collect_urls_from_inline(inline, counts);
            }
            if let Some(nested) = &item.nested {
                self.collect_urls_from_list_node(nested, counts);
            }
        }
    }

    /// Collect URLs from inline content.
    fn collect_urls_from_inline(&self, inline: &InlineNode, counts: &mut HashMap<String, usize>) {
        match inline {
            InlineNode::Link { url, text, .. } => {
                *counts.entry(url.clone()).or_insert(0) += 1;
                for inner in text {
                    self.collect_urls_from_inline(inner, counts);
                }
            }
            InlineNode::Image { url, .. } => {
                *counts.entry(url.clone()).or_insert(0) += 1;
            }
            InlineNode::Bold(inlines)
            | InlineNode::Italic(inlines)
            | InlineNode::Strikethrough(inlines) => {
                for inner in inlines {
                    self.collect_urls_from_inline(inner, counts);
                }
            }
            InlineNode::Text(_) | InlineNode::Code(_) | InlineNode::Reference(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_define_and_resolve() {
        let mut graph = ReferenceGraph::new();
        graph.define("doc", "https://docs.example.com");
        graph.define("repo", "https://github.com/example");

        assert_eq!(graph.resolve("doc"), Some("https://docs.example.com"));
        assert_eq!(graph.resolve("repo"), Some("https://github.com/example"));
        assert_eq!(graph.resolve("unknown"), None);
    }

    #[test]
    fn test_contains() {
        let mut graph = ReferenceGraph::new();
        graph.define("key", "value");

        assert!(graph.contains("key"));
        assert!(!graph.contains("other"));
    }

    #[test]
    fn test_index_to_key() {
        assert_eq!(ReferenceGraph::index_to_key(0), "A");
        assert_eq!(ReferenceGraph::index_to_key(1), "B");
        assert_eq!(ReferenceGraph::index_to_key(25), "Z");
        assert_eq!(ReferenceGraph::index_to_key(26), "AA");
        assert_eq!(ReferenceGraph::index_to_key(27), "AB");
        assert_eq!(ReferenceGraph::index_to_key(51), "AZ");
        assert_eq!(ReferenceGraph::index_to_key(52), "BA");
        assert_eq!(ReferenceGraph::index_to_key(701), "ZZ");
        assert_eq!(ReferenceGraph::index_to_key(702), "AAA");
    }

    #[test]
    fn test_next_key_sequence() {
        let mut graph = ReferenceGraph::new();
        assert_eq!(graph.next_key(), "A");
        assert_eq!(graph.next_key(), "B");
        assert_eq!(graph.next_key(), "C");
    }

    #[test]
    fn test_usage_tracking() {
        let mut graph = ReferenceGraph::new();
        graph.define("key", "value");

        assert_eq!(graph.usage_count("key"), 0);
        graph.record_usage("key");
        assert_eq!(graph.usage_count("key"), 1);
        graph.record_usage("key");
        assert_eq!(graph.usage_count("key"), 2);
    }

    #[test]
    fn test_from_refs() {
        let mut refs = HashMap::new();
        refs.insert("a".to_string(), "value_a".to_string());
        refs.insert("b".to_string(), "value_b".to_string());

        let graph = ReferenceGraph::from_refs(refs);
        assert_eq!(graph.len(), 2);
        assert_eq!(graph.resolve("a"), Some("value_a"));
        assert_eq!(graph.resolve("b"), Some("value_b"));
    }

    #[test]
    fn test_auto_generate_with_repeated_urls() {
        let mut graph = ReferenceGraph::new();
        let doc = DxmDocument {
            nodes: vec![
                DxmNode::Paragraph(vec![InlineNode::Link {
                    text: vec![InlineNode::Text("Link 1".to_string())],
                    url: "https://example.com".to_string(),
                    title: None,
                }]),
                DxmNode::Paragraph(vec![InlineNode::Link {
                    text: vec![InlineNode::Text("Link 2".to_string())],
                    url: "https://example.com".to_string(),
                    title: None,
                }]),
                DxmNode::Paragraph(vec![InlineNode::Link {
                    text: vec![InlineNode::Text("Other".to_string())],
                    url: "https://other.com".to_string(),
                    title: None,
                }]),
            ],
            ..Default::default()
        };

        graph.auto_generate(&doc);

        // Should create a reference for the repeated URL
        assert_eq!(graph.len(), 1);
        assert!(graph.has_value("https://example.com"));
        // Single-use URL should not get a reference
        assert!(!graph.has_value("https://other.com"));
    }

    #[test]
    fn test_iter() {
        let mut graph = ReferenceGraph::new();
        graph.define("a", "value_a");
        graph.define("b", "value_b");

        let pairs: Vec<_> = graph.iter().collect();
        assert_eq!(pairs.len(), 2);
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating valid reference keys (alphanumeric, non-empty)
    fn key_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z][a-zA-Z0-9_]{0,15}".prop_map(|s| s)
    }

    // Strategy for generating reference values (URLs or text)
    fn value_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            // URL-like values
            "https?://[a-z]{3,10}\\.[a-z]{2,4}/[a-z0-9/]{0,20}",
            // Text values
            "[a-zA-Z0-9 ]{1,50}",
        ]
    }

    proptest! {
        /// **Feature: dx-markdown, Property 7: Reference Resolution**
        /// **Validates: Requirements 7.1, 7.2, 7.3**
        ///
        /// *For any* document with reference definitions (#:key|value) and usages (^key),
        /// all usages SHALL resolve to their defined values, and undefined references
        /// SHALL return None.
        #[test]
        fn prop_reference_resolution(
            keys in prop::collection::vec(key_strategy(), 1..10),
            values in prop::collection::vec(value_strategy(), 1..10),
            undefined_key in key_strategy(),
        ) {
            let mut graph = ReferenceGraph::new();

            // Define references (properly clone the strings, not the references)
            let pairs: Vec<_> = keys.iter().zip(values.iter()).collect();
            for (key, value) in &pairs {
                graph.define((*key).clone(), (*value).clone());
            }

            // Build a map of the LAST value for each key (since HashMap overwrites duplicates)
            let mut expected: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
            for (key, value) in &pairs {
                expected.insert(key.as_str(), value.as_str());
            }

            // Property 7.1: All defined references should resolve to their last defined value
            for (key, expected_value) in expected {
                prop_assert_eq!(
                    graph.resolve(key),
                    Some(expected_value),
                    "Defined reference '{}' should resolve to '{}'",
                    key,
                    expected_value
                );
            }

            // Property 7.2: contains() should return true for defined keys
            for (key, _) in &pairs {
                prop_assert!(
                    graph.contains(key),
                    "contains() should return true for defined key '{}'",
                    key
                );
            }

            // Property 7.3: Undefined references should return None
            // (only if the undefined_key is not in our defined keys)
            if !keys.contains(&undefined_key) {
                prop_assert_eq!(
                    graph.resolve(&undefined_key),
                    None,
                    "Undefined reference '{}' should return None",
                    undefined_key
                );
                prop_assert!(
                    !graph.contains(&undefined_key),
                    "contains() should return false for undefined key '{}'",
                    undefined_key
                );
            }
        }

        /// Property: Sequential key generation produces unique keys
        #[test]
        fn prop_sequential_keys_unique(count in 1usize..100) {
            let mut graph = ReferenceGraph::new();
            let mut keys = Vec::with_capacity(count);

            for _ in 0..count {
                keys.push(graph.next_key());
            }

            // All keys should be unique
            let unique_count = keys.iter().collect::<std::collections::HashSet<_>>().len();
            prop_assert_eq!(
                unique_count,
                count,
                "All generated keys should be unique"
            );
        }

        /// Property: index_to_key produces valid alphabetic keys
        #[test]
        fn prop_index_to_key_valid(index in 0usize..10000) {
            let key = ReferenceGraph::index_to_key(index);

            // Key should be non-empty
            prop_assert!(!key.is_empty(), "Key should not be empty");

            // Key should only contain uppercase letters
            prop_assert!(
                key.chars().all(|c| c.is_ascii_uppercase()),
                "Key '{}' should only contain uppercase letters",
                key
            );
        }
    }
}
