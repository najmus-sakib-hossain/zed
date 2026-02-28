//! Pattern analyzer

use super::scanner::ScanResult;
use crate::Result;

/// Analyzes code patterns in a project
#[derive(Debug, Default)]
pub struct PatternAnalyzer;

impl PatternAnalyzer {
    /// Create a new pattern analyzer
    pub fn new() -> Self {
        Self
    }

    /// Analyze patterns from scan results
    pub fn analyze(&self, scan_result: &ScanResult) -> Result<Vec<String>> {
        let mut patterns = Vec::new();

        // Detect patterns based on file counts
        if let Some(&count) = scan_result.file_counts.get("rs") {
            if count > 0 {
                patterns.push("Rust codebase".to_string());
            }
        }

        if let Some(&count) = scan_result.file_counts.get("ts") {
            if count > 0 {
                patterns.push("TypeScript codebase".to_string());
            }
        }

        // Detect architecture patterns
        if scan_result.key_directories.contains(&"src".to_string()) {
            patterns.push("Standard src directory structure".to_string());
        }

        if scan_result.key_directories.contains(&"crates".to_string()) {
            patterns.push("Cargo workspace structure".to_string());
        }

        if scan_result.key_directories.contains(&"packages".to_string()) {
            patterns.push("Monorepo structure".to_string());
        }

        // Detect development practices
        if scan_result.has_tests {
            patterns.push("Has testing infrastructure".to_string());
        }

        if scan_result.has_ci {
            patterns.push("Has CI/CD pipeline".to_string());
        }

        if scan_result.has_docs {
            patterns.push("Has documentation".to_string());
        }

        Ok(patterns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_new() {
        let _analyzer = PatternAnalyzer::new();
    }

    #[test]
    fn test_analyze_empty() {
        let analyzer = PatternAnalyzer::new();
        let scan_result = ScanResult::default();
        let patterns = analyzer.analyze(&scan_result).unwrap();
        assert!(patterns.is_empty());
    }
}
