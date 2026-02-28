//! Affected Detector
//!
//! Queries the Binary Affected Graph for impact detection.

use crate::bag::AffectedGraphData;
use crate::change::ChangeDetector;
use crate::types::ImportStatement;
use std::path::{Path, PathBuf};

/// Affected Detector for determining which packages are affected by changes
pub struct AffectedDetector {
    /// Affected graph data
    graph: AffectedGraphData,
    /// Change detector for import analysis
    change_detector: ChangeDetector,
}

impl AffectedDetector {
    /// Create a new affected detector
    pub fn new(graph: AffectedGraphData) -> Self {
        Self {
            graph,
            change_detector: ChangeDetector::new(),
        }
    }

    /// Get packages affected by file changes (< 5ms target)
    pub fn affected(&self, changed_files: &[PathBuf]) -> Vec<u32> {
        let mut affected = std::collections::HashSet::new();

        for file in changed_files {
            // Find owning package
            if let Some(pkg_idx) = self.file_to_package(file) {
                // Add the package itself
                affected.insert(pkg_idx);

                // Add all transitive dependents
                for &dep in self.graph.transitive_dependents(pkg_idx) {
                    affected.insert(dep);
                }
            }
        }

        affected.into_iter().collect()
    }

    /// Get packages that depend on given package (O(1))
    pub fn dependents(&self, package_idx: u32) -> &[u32] {
        self.graph.dependents(package_idx)
    }

    /// Get full transitive dependents
    pub fn transitive_dependents(&self, package_idx: u32) -> &[u32] {
        self.graph.transitive_dependents(package_idx)
    }

    /// Map file path to owning package
    pub fn file_to_package(&self, path: &Path) -> Option<u32> {
        let path_str = path.to_string_lossy();
        self.graph.file_to_package(&path_str)
    }

    /// Analyze imports using SIMD
    pub fn analyze_imports(&self, path: &Path) -> std::io::Result<Vec<ImportStatement>> {
        self.change_detector.detect_imports_file(path)
    }

    /// Get the underlying graph
    pub fn graph(&self) -> &AffectedGraphData {
        &self.graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_graph() -> AffectedGraphData {
        // a -> b -> c (a depends on b, b depends on c)
        let mut graph = AffectedGraphData::from_edges(3, &[(0, 1), (1, 2)]);

        // Add file mappings
        graph.add_file_mapping("packages/a/src/index.ts", 0);
        graph.add_file_mapping("packages/b/src/index.ts", 1);
        graph.add_file_mapping("packages/c/src/index.ts", 2);

        graph
    }

    #[test]
    fn test_affected_detection() {
        let graph = create_test_graph();
        let detector = AffectedDetector::new(graph);

        // Changing c affects a and b (they depend on c)
        let affected = detector.affected(&[PathBuf::from("packages/c/src/index.ts")]);

        // c itself plus a and b
        assert!(affected.contains(&2)); // c
        assert!(affected.contains(&1)); // b depends on c
        assert!(affected.contains(&0)); // a depends on b which depends on c
    }

    #[test]
    fn test_file_to_package() {
        let graph = create_test_graph();
        let detector = AffectedDetector::new(graph);

        assert_eq!(detector.file_to_package(Path::new("packages/a/src/index.ts")), Some(0));
        assert_eq!(detector.file_to_package(Path::new("packages/b/src/index.ts")), Some(1));
        assert_eq!(detector.file_to_package(Path::new("packages/unknown/src/index.ts")), None);
    }

    #[test]
    fn test_dependents() {
        let graph = create_test_graph();
        let detector = AffectedDetector::new(graph);

        // c has b as dependent (b depends on c)
        assert!(detector.dependents(2).contains(&1));

        // b has a as dependent
        assert!(detector.dependents(1).contains(&0));

        // a has no dependents
        assert!(detector.dependents(0).is_empty());
    }

    #[test]
    fn test_transitive_dependents() {
        let graph = create_test_graph();
        let detector = AffectedDetector::new(graph);

        // c's transitive dependents include both a and b
        let trans = detector.transitive_dependents(2);
        assert!(trans.contains(&0));
        assert!(trans.contains(&1));
    }
}
