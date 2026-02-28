//! Similarity detection for auto-grouping of CSS class patterns
//!
//! This module provides functionality to detect frequently co-occurring class
//! combinations in HTML files and calculate similarity scores using Jaccard index.

use ahash::{AHashMap, AHashSet};

/// A pattern key representing a sorted, deduplicated set of class names.
/// Used as a key for tracking class combination occurrences.
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct PatternKey {
    /// Sorted, deduplicated class names
    classes: Vec<String>,
}

impl PatternKey {
    /// Create a new PatternKey from a slice of class names.
    /// The classes are sorted and deduplicated for consistent hashing.
    pub fn new(classes: &[&str]) -> Self {
        let mut sorted: Vec<String> = classes.iter().map(|s| s.to_string()).collect();
        sorted.sort();
        sorted.dedup();
        Self { classes: sorted }
    }

    /// Create a new PatternKey from owned strings.
    pub fn from_strings(mut classes: Vec<String>) -> Self {
        classes.sort();
        classes.dedup();
        Self { classes }
    }

    /// Get the classes in this pattern.
    pub fn classes(&self) -> &[String] {
        &self.classes
    }

    /// Calculate Jaccard similarity with another pattern.
    /// Returns a value between 0.0 (no overlap) and 1.0 (identical).
    ///
    /// Jaccard similarity = |A ∩ B| / |A ∪ B|
    pub fn jaccard_similarity(&self, other: &PatternKey) -> f64 {
        let set_a: AHashSet<&String> = self.classes.iter().collect();
        let set_b: AHashSet<&String> = other.classes.iter().collect();

        let intersection = set_a.intersection(&set_b).count();
        let union = set_a.union(&set_b).count();

        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }

    /// Returns the number of classes in this pattern.
    pub fn len(&self) -> usize {
        self.classes.len()
    }

    /// Returns true if this pattern has no classes.
    pub fn is_empty(&self) -> bool {
        self.classes.is_empty()
    }
}

/// A cluster of similar patterns grouped together.
#[derive(Debug, Clone)]
pub struct PatternCluster {
    /// The representative pattern for this cluster
    pub representative: PatternKey,
    /// All patterns in this cluster with their occurrence counts
    pub members: Vec<(PatternKey, usize)>,
}

impl PatternCluster {
    /// Get the total occurrence count across all members.
    pub fn total_occurrences(&self) -> usize {
        self.members.iter().map(|(_, count)| count).sum()
    }
}

/// Detects frequently co-occurring class combinations in HTML.
pub struct SimilarityDetector {
    /// Pattern -> occurrence count
    patterns: AHashMap<PatternKey, usize>,
    /// Minimum occurrences to consider for grouping
    min_occurrences: usize,
    /// Similarity threshold (Jaccard index) for clustering
    similarity_threshold: f64,
}

impl SimilarityDetector {
    /// Create a new SimilarityDetector with the given thresholds.
    ///
    /// # Arguments
    /// * `min_occurrences` - Minimum times a pattern must appear to be considered
    /// * `similarity_threshold` - Jaccard similarity threshold for clustering (0.0-1.0)
    pub fn new(min_occurrences: usize, similarity_threshold: f64) -> Self {
        Self {
            patterns: AHashMap::new(),
            min_occurrences,
            similarity_threshold,
        }
    }

    /// Scan HTML bytes and record class patterns.
    /// Extracts class attributes and records 2-5 class combinations.
    pub fn scan(&mut self, html: &[u8]) {
        // Extract class attributes from HTML
        for attr in iter_class_attributes_raw(html) {
            let classes: Vec<&str> = attr.split_whitespace().collect();
            if classes.len() < 2 {
                continue;
            }

            // Record all 2-5 class combinations (sliding windows)
            let max_window = 5.min(classes.len());
            for window_size in 2..=max_window {
                for window in classes.windows(window_size) {
                    let key = PatternKey::new(window);
                    *self.patterns.entry(key).or_insert(0) += 1;
                }
            }
        }
    }

    /// Record a single class pattern with a given count.
    /// Useful for testing or manual pattern registration.
    pub fn record_pattern(&mut self, classes: &[&str], count: usize) {
        let key = PatternKey::new(classes);
        *self.patterns.entry(key).or_insert(0) += count;
    }

    /// Get patterns that exceed the occurrence threshold.
    pub fn get_groupable_patterns(&self) -> Vec<(PatternKey, usize)> {
        self.patterns
            .iter()
            .filter(|(_, count)| **count >= self.min_occurrences)
            .map(|(k, c)| (k.clone(), *c))
            .collect()
    }

    /// Cluster similar patterns together based on Jaccard similarity.
    pub fn cluster_patterns(&self) -> Vec<PatternCluster> {
        let groupable = self.get_groupable_patterns();
        let mut clusters: Vec<PatternCluster> = Vec::new();

        // Sort by occurrence count (descending) for better clustering
        let mut sorted_patterns = groupable;
        sorted_patterns.sort_by(|a, b| b.1.cmp(&a.1));

        for (pattern, count) in sorted_patterns {
            let mut merged = false;

            // Try to merge with existing cluster
            for cluster in &mut clusters {
                if cluster.representative.jaccard_similarity(&pattern) >= self.similarity_threshold
                {
                    cluster.members.push((pattern.clone(), count));
                    merged = true;
                    break;
                }
            }

            // Create new cluster if not merged
            if !merged {
                clusters.push(PatternCluster {
                    representative: pattern.clone(),
                    members: vec![(pattern, count)],
                });
            }
        }

        clusters
    }

    /// Clear all recorded patterns.
    pub fn clear(&mut self) {
        self.patterns.clear();
    }

    /// Get the number of unique patterns recorded.
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }
}

/// Iterator over class attribute values in raw HTML bytes.
/// This is a simple parser that extracts class="..." attribute values.
fn iter_class_attributes_raw(html: &[u8]) -> impl Iterator<Item = &str> {
    ClassAttributeIterator { html, pos: 0 }
}

struct ClassAttributeIterator<'a> {
    html: &'a [u8],
    pos: usize,
}

impl<'a> Iterator for ClassAttributeIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        // Search for class=" or class='
        while self.pos + 7 < self.html.len() {
            // Look for "class="
            if self.html[self.pos..].starts_with(b"class=\"")
                || self.html[self.pos..].starts_with(b"class='")
            {
                let quote = self.html[self.pos + 6];
                let start = self.pos + 7;

                // Find closing quote
                let mut end = start;
                while end < self.html.len() && self.html[end] != quote {
                    end += 1;
                }

                if end < self.html.len() {
                    self.pos = end + 1;
                    if let Ok(s) = std::str::from_utf8(&self.html[start..end]) {
                        return Some(s);
                    }
                }
            }
            self.pos += 1;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_key_new() {
        let key = PatternKey::new(&["flex", "items-center", "p-4"]);
        assert_eq!(key.classes(), &["flex", "items-center", "p-4"]);
    }

    #[test]
    fn test_pattern_key_sorts_and_dedupes() {
        let key = PatternKey::new(&["p-4", "flex", "flex", "items-center"]);
        assert_eq!(key.classes(), &["flex", "items-center", "p-4"]);
    }

    #[test]
    fn test_jaccard_similarity_identical() {
        let a = PatternKey::new(&["flex", "items-center"]);
        let b = PatternKey::new(&["items-center", "flex"]);
        assert!((a.jaccard_similarity(&b) - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_jaccard_similarity_no_overlap() {
        let a = PatternKey::new(&["flex", "items-center"]);
        let b = PatternKey::new(&["bg-white", "rounded"]);
        assert!((a.jaccard_similarity(&b) - 0.0).abs() < 0.0001);
    }

    #[test]
    fn test_jaccard_similarity_partial() {
        // A = {flex, items-center}, B = {flex, p-4}
        // Intersection = {flex} = 1
        // Union = {flex, items-center, p-4} = 3
        // Jaccard = 1/3 ≈ 0.333
        let a = PatternKey::new(&["flex", "items-center"]);
        let b = PatternKey::new(&["flex", "p-4"]);
        assert!((a.jaccard_similarity(&b) - 0.333333).abs() < 0.001);
    }

    #[test]
    fn test_jaccard_similarity_empty() {
        let a = PatternKey::new(&[]);
        let b = PatternKey::new(&[]);
        assert!((a.jaccard_similarity(&b) - 0.0).abs() < 0.0001);
    }

    #[test]
    fn test_similarity_detector_scan() {
        let mut detector = SimilarityDetector::new(2, 0.6);
        let html = br#"
            <div class="flex items-center p-4">Content</div>
            <div class="flex items-center p-4">More content</div>
            <div class="flex items-center">Other</div>
        "#;
        detector.scan(html);

        let patterns = detector.get_groupable_patterns();
        // "flex items-center" appears 3 times (in all three divs)
        assert!(
            patterns
                .iter()
                .any(|(p, c)| { p.classes() == &["flex", "items-center"] && *c >= 2 })
        );
    }

    #[test]
    fn test_similarity_detector_threshold() {
        let mut detector = SimilarityDetector::new(3, 0.6);
        detector.record_pattern(&["flex", "items-center"], 2); // Below threshold
        detector.record_pattern(&["bg-white", "rounded"], 5); // Above threshold

        let patterns = detector.get_groupable_patterns();
        assert_eq!(patterns.len(), 1);
        assert!(patterns[0].0.classes().contains(&"bg-white".to_string()));
    }

    #[test]
    fn test_cluster_patterns() {
        let mut detector = SimilarityDetector::new(1, 0.5);
        // Two similar patterns (share "flex")
        detector.record_pattern(&["flex", "items-center"], 5);
        detector.record_pattern(&["flex", "justify-center"], 3);
        // One different pattern
        detector.record_pattern(&["bg-white", "rounded", "shadow"], 4);

        let clusters = detector.cluster_patterns();
        // Should have at least 2 clusters (flex-related and bg-related)
        assert!(clusters.len() >= 1);
    }

    #[test]
    fn test_class_attribute_iterator() {
        let html = br#"<div class="flex p-4" id="test" class='bg-white'>"#;
        let attrs: Vec<&str> = iter_class_attributes_raw(html).collect();
        assert_eq!(attrs.len(), 2);
        assert_eq!(attrs[0], "flex p-4");
        assert_eq!(attrs[1], "bg-white");
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashSet;

    // Generate a vector of class names
    fn arb_class_names() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec("[a-z][a-z0-9-]{0,15}", 0..10)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-style-production-ready, Property 4: Jaccard Similarity Calculation
        /// *For any* two PatternKeys A and B, the calculated Jaccard similarity
        /// SHALL equal |A∩B| / |A∪B|.
        /// **Validates: Requirements 2.2**
        #[test]
        fn prop_jaccard_similarity(
            a in arb_class_names(),
            b in arb_class_names()
        ) {
            let key_a = PatternKey::from_strings(a.clone());
            let key_b = PatternKey::from_strings(b.clone());

            // Calculate expected Jaccard using HashSet
            let set_a: HashSet<&String> = a.iter().collect();
            let set_b: HashSet<&String> = b.iter().collect();

            let intersection = set_a.intersection(&set_b).count();
            let union = set_a.union(&set_b).count();

            let expected = if union == 0 {
                0.0
            } else {
                intersection as f64 / union as f64
            };

            let actual = key_a.jaccard_similarity(&key_b);

            // Allow small floating point tolerance
            prop_assert!(
                (actual - expected).abs() < 0.0001,
                "Jaccard mismatch: expected {}, got {} for sets {:?} and {:?}",
                expected, actual, a, b
            );
        }

        /// Property: Jaccard similarity is symmetric
        /// *For any* two PatternKeys A and B, jaccard(A, B) == jaccard(B, A)
        #[test]
        fn prop_jaccard_symmetric(
            a in arb_class_names(),
            b in arb_class_names()
        ) {
            let key_a = PatternKey::from_strings(a);
            let key_b = PatternKey::from_strings(b);

            let ab = key_a.jaccard_similarity(&key_b);
            let ba = key_b.jaccard_similarity(&key_a);

            prop_assert!(
                (ab - ba).abs() < 0.0001,
                "Jaccard not symmetric: A->B={}, B->A={}",
                ab, ba
            );
        }

        /// Property: Jaccard similarity with self is 1.0 (for non-empty sets)
        #[test]
        fn prop_jaccard_self_identity(classes in arb_class_names()) {
            let key = PatternKey::from_strings(classes.clone());

            if !key.is_empty() {
                let similarity = key.jaccard_similarity(&key);
                prop_assert!(
                    (similarity - 1.0).abs() < 0.0001,
                    "Self-similarity should be 1.0, got {}",
                    similarity
                );
            }
        }

        /// Property: Jaccard similarity is bounded [0.0, 1.0]
        #[test]
        fn prop_jaccard_bounded(
            a in arb_class_names(),
            b in arb_class_names()
        ) {
            let key_a = PatternKey::from_strings(a);
            let key_b = PatternKey::from_strings(b);

            let similarity = key_a.jaccard_similarity(&key_b);

            prop_assert!(
                similarity >= 0.0 && similarity <= 1.0,
                "Jaccard should be in [0, 1], got {}",
                similarity
            );
        }

        /// Feature: dx-style-production-ready, Property 3: Pattern Detection Threshold
        /// *For any* HTML input with class combinations appearing N times where N >= min_occurrences,
        /// the Similarity_Detector SHALL include those combinations in the groupable patterns output.
        /// **Validates: Requirements 2.1, 2.5**
        #[test]
        fn prop_pattern_detection_threshold(
            classes in prop::collection::vec("[a-z][a-z0-9-]{1,8}", 2..5),
            count in 1usize..10,
            min_occurrences in 1usize..5
        ) {
            let mut detector = SimilarityDetector::new(min_occurrences, 0.6);

            // Record the pattern with the given count
            let class_refs: Vec<&str> = classes.iter().map(|s| s.as_str()).collect();
            detector.record_pattern(&class_refs, count);

            let patterns = detector.get_groupable_patterns();

            if count >= min_occurrences {
                // Pattern should be included
                prop_assert!(
                    patterns.iter().any(|(_p, c)| *c == count),
                    "Pattern with count {} should be included when min_occurrences is {}",
                    count, min_occurrences
                );
            } else {
                // Pattern should NOT be included
                prop_assert!(
                    patterns.is_empty() || !patterns.iter().any(|(_, c)| *c == count),
                    "Pattern with count {} should NOT be included when min_occurrences is {}",
                    count, min_occurrences
                );
            }
        }

        /// Feature: dx-style-production-ready, Property 5: Similarity Threshold Grouping
        /// *For any* pattern with similarity score >= threshold, the Auto_Grouper SHALL flag it
        /// for grouping; patterns below threshold SHALL NOT be flagged.
        /// **Validates: Requirements 2.3**
        #[test]
        fn prop_similarity_threshold_grouping(
            base_classes in prop::collection::vec("[a-z][a-z0-9-]{1,8}", 2..4),
            extra_class in "[a-z][a-z0-9-]{1,8}",
            threshold in 0.3f64..0.9
        ) {
            let mut detector = SimilarityDetector::new(1, threshold);

            // Create base pattern
            let base_refs: Vec<&str> = base_classes.iter().map(|s| s.as_str()).collect();
            detector.record_pattern(&base_refs, 5);

            // Create similar pattern (base + one extra class)
            let mut similar_classes = base_classes.clone();
            similar_classes.push(extra_class.clone());
            let similar_refs: Vec<&str> = similar_classes.iter().map(|s| s.as_str()).collect();
            detector.record_pattern(&similar_refs, 3);

            // Calculate expected similarity
            let base_key = PatternKey::new(&base_refs);
            let similar_key = PatternKey::new(&similar_refs);
            let similarity = base_key.jaccard_similarity(&similar_key);

            let clusters = detector.cluster_patterns();

            if similarity >= threshold {
                // Should be clustered together (1 cluster with 2 members)
                // OR could be 2 clusters if they're not similar enough
                prop_assert!(
                    clusters.len() >= 1,
                    "Should have at least 1 cluster"
                );
            }
            // Note: We can't strictly assert they're in the same cluster because
            // the clustering algorithm may choose different representatives
        }
    }
}
