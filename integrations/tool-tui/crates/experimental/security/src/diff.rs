//! XOR Differential Scanner
//!
//! Incremental scanning using Blake3 hashing and XOR change detection.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Cached scan result
#[derive(Debug, Clone)]
pub struct ScanResult {
    /// File hash
    pub hash: [u8; 32],
    /// Cached findings count
    pub findings_count: usize,
}

/// Differential scanner with Blake3 hashing
pub struct DifferentialScanner {
    cache: HashMap<PathBuf, ScanResult>,
}

impl DifferentialScanner {
    /// Create a new differential scanner
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Compute Blake3 hash of data
    pub fn hash(data: &[u8]) -> [u8; 32] {
        *blake3::hash(data).as_bytes()
    }

    /// Check if file has changed since last scan using XOR comparison
    pub fn has_changed(&self, path: &Path, data: &[u8]) -> bool {
        let new_hash = Self::hash(data);

        match self.cache.get(path) {
            Some(cached) => {
                // XOR comparison for O(1) change detection
                let mut diff = 0u8;
                for i in 0..32 {
                    diff |= cached.hash[i] ^ new_hash[i];
                }
                diff != 0
            }
            None => true, // Not in cache, consider changed
        }
    }

    /// Update cache with new hash
    pub fn update_cache(&mut self, path: &Path, hash: [u8; 32], findings_count: usize) {
        self.cache.insert(
            path.to_path_buf(),
            ScanResult {
                hash,
                findings_count,
            },
        );
    }

    /// Scan and cache - convenience method that hashes, checks, and updates cache
    pub fn scan_and_cache(&mut self, path: &Path, data: &[u8], findings_count: usize) -> bool {
        let hash = Self::hash(data);
        let changed = self.has_changed(path, data);
        self.update_cache(path, hash, findings_count);
        changed
    }

    /// Get cached result if unchanged
    pub fn get_cached(&self, path: &Path) -> Option<&ScanResult> {
        self.cache.get(path)
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}

impl Default for DifferentialScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_consistency() {
        let data = b"Hello, World!";
        let hash1 = DifferentialScanner::hash(data);
        let hash2 = DifferentialScanner::hash(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_different_data() {
        let hash1 = DifferentialScanner::hash(b"Hello");
        let hash2 = DifferentialScanner::hash(b"World");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_has_changed_uncached() {
        let scanner = DifferentialScanner::new();
        let path = Path::new("test.txt");
        assert!(scanner.has_changed(path, b"content"));
    }

    #[test]
    fn test_has_changed_same_content() {
        let mut scanner = DifferentialScanner::new();
        let path = Path::new("test.txt");
        let data = b"content";

        let hash = DifferentialScanner::hash(data);
        scanner.update_cache(path, hash, 0);

        assert!(!scanner.has_changed(path, data));
    }

    #[test]
    fn test_has_changed_different_content() {
        let mut scanner = DifferentialScanner::new();
        let path = Path::new("test.txt");

        let hash = DifferentialScanner::hash(b"original");
        scanner.update_cache(path, hash, 0);

        assert!(scanner.has_changed(path, b"modified"));
    }

    #[test]
    fn test_get_cached() {
        let mut scanner = DifferentialScanner::new();
        let path = Path::new("test.txt");

        assert!(scanner.get_cached(path).is_none());

        let hash = DifferentialScanner::hash(b"content");
        scanner.update_cache(path, hash, 5);

        let cached = scanner.get_cached(path).unwrap();
        assert_eq!(cached.findings_count, 5);
        assert_eq!(cached.hash, hash);
    }

    #[test]
    fn test_clear_cache() {
        let mut scanner = DifferentialScanner::new();
        let path = Path::new("test.txt");

        let hash = DifferentialScanner::hash(b"content");
        scanner.update_cache(path, hash, 0);
        assert_eq!(scanner.cache_size(), 1);

        scanner.clear_cache();
        assert_eq!(scanner.cache_size(), 0);
    }

    #[test]
    fn test_scan_and_cache() {
        let mut scanner = DifferentialScanner::new();
        let path = Path::new("test.txt");
        let data = b"content";

        // First scan should report changed (not in cache)
        let changed = scanner.scan_and_cache(path, data, 3);
        assert!(changed);

        // Second scan with same content should report not changed
        let changed = scanner.scan_and_cache(path, data, 3);
        assert!(!changed);

        // Third scan with different content should report changed
        let changed = scanner.scan_and_cache(path, b"modified", 5);
        assert!(changed);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate arbitrary file content
    fn arb_file_content() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(any::<u8>(), 0..1000)
    }

    /// Generate arbitrary file path
    fn arb_file_path() -> impl Strategy<Value = PathBuf> {
        prop::string::string_regex("[a-z0-9_/]{1,50}\\.txt")
            .unwrap()
            .prop_map(PathBuf::from)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: dx-security, Property 5: Differential Scanning Cache Correctness**
        /// **Validates: Requirements 6.2**
        ///
        /// For any file content, if the content is scanned twice without modification,
        /// the second call to has_changed() SHALL return false.
        #[test]
        fn prop_cache_correctness_unchanged_content(
            path in arb_file_path(),
            content in arb_file_content()
        ) {
            let mut scanner = DifferentialScanner::new();

            // First scan - should report changed (not in cache)
            let hash = DifferentialScanner::hash(&content);
            scanner.update_cache(&path, hash, 0);

            // Second check with same content - should report NOT changed
            let changed = scanner.has_changed(&path, &content);

            prop_assert!(
                !changed,
                "Content scanned twice without modification should report unchanged"
            );
        }

        /// Hash function should be deterministic
        #[test]
        fn prop_hash_deterministic(content in arb_file_content()) {
            let hash1 = DifferentialScanner::hash(&content);
            let hash2 = DifferentialScanner::hash(&content);

            prop_assert_eq!(
                hash1, hash2,
                "Same content should always produce same hash"
            );
        }

        /// Different content should produce different hashes (with high probability)
        #[test]
        fn prop_hash_collision_resistant(
            content1 in arb_file_content(),
            content2 in arb_file_content()
        ) {
            // Only test when contents are actually different
            if content1 != content2 {
                let hash1 = DifferentialScanner::hash(&content1);
                let hash2 = DifferentialScanner::hash(&content2);

                prop_assert_ne!(
                    hash1, hash2,
                    "Different content should produce different hashes"
                );
            }
        }

        /// XOR comparison should correctly detect changes
        #[test]
        fn prop_xor_detects_changes(
            path in arb_file_path(),
            original in arb_file_content(),
            modified in arb_file_content()
        ) {
            let mut scanner = DifferentialScanner::new();

            // Cache original content
            let hash = DifferentialScanner::hash(&original);
            scanner.update_cache(&path, hash, 0);

            // Check if modified content is detected as changed
            let changed = scanner.has_changed(&path, &modified);

            if original == modified {
                prop_assert!(
                    !changed,
                    "Identical content should not be detected as changed"
                );
            } else {
                prop_assert!(
                    changed,
                    "Different content should be detected as changed"
                );
            }
        }

        /// Cache should preserve findings count
        #[test]
        fn prop_cache_preserves_findings(
            path in arb_file_path(),
            content in arb_file_content(),
            findings_count in 0usize..1000
        ) {
            let mut scanner = DifferentialScanner::new();

            let hash = DifferentialScanner::hash(&content);
            scanner.update_cache(&path, hash, findings_count);

            let cached = scanner.get_cached(&path).unwrap();
            prop_assert_eq!(
                cached.findings_count, findings_count,
                "Cache should preserve findings count"
            );
        }

        /// scan_and_cache should correctly track changes
        #[test]
        fn prop_scan_and_cache_tracks_changes(
            path in arb_file_path(),
            content in arb_file_content()
        ) {
            let mut scanner = DifferentialScanner::new();

            // First scan should always report changed (not in cache)
            let first_changed = scanner.scan_and_cache(&path, &content, 0);
            prop_assert!(
                first_changed,
                "First scan should report changed (not in cache)"
            );

            // Second scan with same content should report not changed
            let second_changed = scanner.scan_and_cache(&path, &content, 0);
            prop_assert!(
                !second_changed,
                "Second scan with same content should report not changed"
            );
        }
    }
}
