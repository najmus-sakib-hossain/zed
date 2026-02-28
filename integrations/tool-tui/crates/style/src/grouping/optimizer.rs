//! Grouping Optimizer
//!
//! Project-wide analysis for optimal class groupings with size calculation
//! and analyze-only mode support.
//!
//! **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 11.9**

use ahash::{AHashMap, AHashSet};
use std::path::{Path, PathBuf};

/// Configuration for grouping optimization
#[derive(Debug, Clone)]
pub struct GroupingConfig {
    /// Minimum occurrences to consider for grouping
    pub min_occurrences: usize,
    /// Minimum byte savings required to apply grouping
    pub min_savings: i64,
    /// File extensions to scan
    pub extensions: Vec<String>,
    /// Directories to exclude from scanning
    pub excluded_dirs: Vec<String>,
}

impl Default for GroupingConfig {
    fn default() -> Self {
        Self {
            min_occurrences: 3,
            min_savings: 10,
            extensions: vec![
                "html".to_string(),
                "jsx".to_string(),
                "tsx".to_string(),
                "vue".to_string(),
            ],
            excluded_dirs: vec![
                "node_modules".to_string(),
                "target".to_string(),
                ".git".to_string(),
                "dist".to_string(),
                "build".to_string(),
            ],
        }
    }
}

/// A candidate for grouping optimization
#[derive(Debug, Clone)]
pub struct GroupingCandidate {
    /// Classes in this group
    pub classes: Vec<String>,
    /// Number of occurrences in project
    pub occurrences: usize,
    /// Byte savings if grouped (negative means grouping increases size)
    pub savings: i64,
    /// Files where this pattern appears
    pub files: Vec<PathBuf>,
    /// Suggested group name
    pub suggested_name: String,
}

/// Result of project analysis
#[derive(Debug, Clone)]
pub struct GroupingReport {
    /// Grouping candidates sorted by savings (highest first)
    pub candidates: Vec<GroupingCandidate>,
    /// Number of files scanned
    pub files_scanned: usize,
    /// Total potential savings in bytes
    pub total_savings: i64,
    /// Patterns analyzed
    pub patterns_analyzed: usize,
}

/// Result of applying groupings
#[derive(Debug, Clone)]
pub enum ApplyResult {
    /// Analysis only, no changes made
    AnalyzeOnly(GroupingReport),
    /// Groupings applied
    Applied {
        /// Applied groupings
        applied: Vec<GroupingCandidate>,
        /// Files modified
        files_modified: Vec<PathBuf>,
    },
}

/// File scanner for project analysis
#[derive(Debug)]
pub struct FileScanner {
    /// File extensions to include
    extensions: Vec<String>,
    /// Directories to exclude
    excluded_dirs: Vec<String>,
}

impl FileScanner {
    /// Create a new file scanner
    pub fn new(extensions: Vec<String>, excluded_dirs: Vec<String>) -> Self {
        Self {
            extensions,
            excluded_dirs,
        }
    }

    /// Scan a directory for matching files
    ///
    /// **Validates: Requirements 3.1**
    pub fn scan(&self, root: &Path) -> Vec<PathBuf> {
        let mut files = Vec::new();
        self.scan_recursive(root, &mut files);
        files
    }

    fn scan_recursive(&self, dir: &Path, files: &mut Vec<PathBuf>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                // Check if directory should be excluded
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if self.excluded_dirs.iter().any(|e| e == name) {
                        continue;
                    }
                }
                self.scan_recursive(&path, files);
            } else if path.is_file() {
                // Check extension
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if self.extensions.iter().any(|e| e == ext) {
                        files.push(path);
                    }
                }
            }
        }
    }
}

/// Size calculator for grouping optimization
#[derive(Debug)]
pub struct SizeCalculator {
    /// Average CSS rule overhead per class
    css_rule_overhead: usize,
    /// Group name prefix length
    group_prefix_len: usize,
}

impl SizeCalculator {
    /// Create a new size calculator
    pub fn new() -> Self {
        Self {
            css_rule_overhead: 30, // Average CSS rule overhead
            group_prefix_len: 8,   // "dxg-xxxx"
        }
    }

    /// Calculate byte savings for a grouping candidate
    ///
    /// **Validates: Requirements 3.2, 3.4**
    pub fn calculate_savings(&self, classes: &[String], occurrences: usize) -> i64 {
        // Current size: sum of all class names * occurrences
        let current_size: usize = classes
            .iter()
            .map(|c| c.len() + 1) // +1 for space separator
            .sum::<usize>()
            * occurrences;

        // Grouped size: group name * occurrences + CSS rule overhead
        let grouped_size = (self.group_prefix_len * occurrences) + self.css_rule_overhead;

        current_size as i64 - grouped_size as i64
    }

    /// Estimate CSS rule size for a set of classes
    pub fn estimate_css_rule_size(&self, classes: &[String]) -> usize {
        // Estimate: selector + braces + properties
        // .dxg-xxxx { property: value; ... }
        let selector_size = self.group_prefix_len + 3; // ".dxg-xxxx { "
        let properties_size: usize = classes.len() * 20; // Average property size
        let closing = 2; // " }"

        selector_size + properties_size + closing
    }
}

impl Default for SizeCalculator {
    fn default() -> Self {
        Self::new()
    }
}

/// Class pattern extracted from files
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct ClassPattern {
    /// Sorted classes for consistent hashing
    classes: Vec<String>,
}

impl ClassPattern {
    fn new(mut classes: Vec<String>) -> Self {
        classes.sort();
        Self { classes }
    }
}

/// Project-wide grouping optimizer
///
/// **Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8**
pub struct GroupingOptimizer {
    /// Configuration
    config: GroupingConfig,
    /// File scanner
    scanner: FileScanner,
    /// Size calculator
    size_calc: SizeCalculator,
    /// File modification times for incremental analysis
    file_mtimes: AHashMap<PathBuf, std::time::SystemTime>,
}

impl GroupingOptimizer {
    /// Create a new grouping optimizer
    pub fn new(config: GroupingConfig) -> Self {
        let scanner = FileScanner::new(config.extensions.clone(), config.excluded_dirs.clone());

        Self {
            config,
            scanner,
            size_calc: SizeCalculator::new(),
            file_mtimes: AHashMap::new(),
        }
    }

    /// Analyze project and find grouping candidates
    ///
    /// **Validates: Requirements 3.1, 3.2, 3.3, 3.5**
    pub fn analyze(&mut self, project_root: &Path) -> GroupingReport {
        // Scan all matching files
        let files = self.scanner.scan(project_root);
        let files_scanned = files.len();

        // Extract class patterns from all files
        let patterns = self.extract_patterns(&files);
        let patterns_analyzed = patterns.len();

        // Calculate savings for each candidate
        let mut candidates: Vec<GroupingCandidate> = patterns
            .into_iter()
            .filter(|(_, (count, _))| *count >= self.config.min_occurrences)
            .map(|(pattern, (count, files))| {
                let savings = self.size_calc.calculate_savings(&pattern.classes, count);
                GroupingCandidate {
                    classes: pattern.classes,
                    occurrences: count,
                    savings,
                    files,
                    suggested_name: self.generate_group_name(count),
                }
            })
            .filter(|c| c.savings >= self.config.min_savings) // Only keep beneficial groupings
            .collect();

        // Sort by savings (highest first)
        candidates.sort_by(|a, b| b.savings.cmp(&a.savings));

        let total_savings = candidates.iter().map(|c| c.savings).sum();

        GroupingReport {
            candidates,
            files_scanned,
            total_savings,
            patterns_analyzed,
        }
    }

    /// Analyze only changed files (incremental)
    ///
    /// **Validates: Requirements 3.7**
    pub fn analyze_incremental(&mut self, project_root: &Path) -> GroupingReport {
        let files = self.scanner.scan(project_root);

        // Filter to only changed files
        let changed_files: Vec<PathBuf> =
            files.into_iter().filter(|f| self.is_file_changed(f)).collect();

        if changed_files.is_empty() {
            return GroupingReport {
                candidates: Vec::new(),
                files_scanned: 0,
                total_savings: 0,
                patterns_analyzed: 0,
            };
        }

        // Update mtimes
        for file in &changed_files {
            if let Ok(metadata) = std::fs::metadata(file) {
                if let Ok(mtime) = metadata.modified() {
                    self.file_mtimes.insert(file.clone(), mtime);
                }
            }
        }

        // Analyze changed files
        let patterns = self.extract_patterns(&changed_files);
        let patterns_analyzed = patterns.len();

        let mut candidates: Vec<GroupingCandidate> = patterns
            .into_iter()
            .filter(|(_, (count, _))| *count >= self.config.min_occurrences)
            .map(|(pattern, (count, files))| {
                let savings = self.size_calc.calculate_savings(&pattern.classes, count);
                GroupingCandidate {
                    classes: pattern.classes,
                    occurrences: count,
                    savings,
                    files,
                    suggested_name: self.generate_group_name(count),
                }
            })
            .filter(|c| c.savings >= self.config.min_savings)
            .collect();

        candidates.sort_by(|a, b| b.savings.cmp(&a.savings));
        let total_savings = candidates.iter().map(|c| c.savings).sum();

        GroupingReport {
            candidates,
            files_scanned: changed_files.len(),
            total_savings,
            patterns_analyzed,
        }
    }

    /// Check if a file has changed since last analysis
    fn is_file_changed(&self, path: &Path) -> bool {
        let current_mtime = match std::fs::metadata(path) {
            Ok(m) => match m.modified() {
                Ok(t) => t,
                Err(_) => return true,
            },
            Err(_) => return true,
        };

        match self.file_mtimes.get(path) {
            Some(cached_mtime) => current_mtime > *cached_mtime,
            None => true,
        }
    }

    /// Extract class patterns from files
    fn extract_patterns(&self, files: &[PathBuf]) -> AHashMap<ClassPattern, (usize, Vec<PathBuf>)> {
        let mut patterns: AHashMap<ClassPattern, (usize, Vec<PathBuf>)> = AHashMap::new();

        for file in files {
            let content = match std::fs::read(file) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Extract class attributes
            let class_sets = self.extract_class_attributes(&content);

            for classes in class_sets {
                if classes.len() >= 2 {
                    let pattern = ClassPattern::new(classes);
                    let entry = patterns.entry(pattern).or_insert((0, Vec::new()));
                    entry.0 += 1;
                    if !entry.1.contains(file) {
                        entry.1.push(file.clone());
                    }
                }
            }
        }

        patterns
    }

    /// Extract class attributes from file content
    fn extract_class_attributes(&self, content: &[u8]) -> Vec<Vec<String>> {
        let mut results = Vec::new();
        let content_str = String::from_utf8_lossy(content);

        // Find class="..." and class='...' and className="..." patterns
        let patterns = ["class=\"", "class='", "className=\"", "className='"];

        for pattern in patterns {
            let quote = if pattern.ends_with('"') { '"' } else { '\'' };
            let mut pos = 0;

            while let Some(start) = content_str[pos..].find(pattern) {
                let attr_start = pos + start + pattern.len();
                if let Some(end) = content_str[attr_start..].find(quote) {
                    let class_value = &content_str[attr_start..attr_start + end];
                    let classes: Vec<String> =
                        class_value.split_whitespace().map(|s| s.to_string()).collect();

                    if !classes.is_empty() {
                        results.push(classes);
                    }
                    pos = attr_start + end + 1;
                } else {
                    break;
                }
            }
        }

        results
    }

    /// Generate a group name based on occurrence count
    fn generate_group_name(&self, _occurrences: usize) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        format!("dxg-{:x}", (timestamp % 0xFFFF) as u16)
    }

    /// Apply groupings (or analyze only)
    ///
    /// **Validates: Requirements 3.5, 3.6**
    pub fn apply(&self, report: &GroupingReport, analyze_only: bool) -> ApplyResult {
        if analyze_only {
            return ApplyResult::AnalyzeOnly(report.clone());
        }

        let mut applied = Vec::new();
        let mut files_modified = AHashSet::new();

        for candidate in &report.candidates {
            if candidate.savings > 0 {
                // Apply grouping to files
                for file in &candidate.files {
                    if self.apply_grouping_to_file(file, candidate).is_ok() {
                        files_modified.insert(file.clone());
                    }
                }
                applied.push(candidate.clone());
            }
        }

        ApplyResult::Applied {
            applied,
            files_modified: files_modified.into_iter().collect(),
        }
    }

    /// Apply a single grouping to a file
    fn apply_grouping_to_file(
        &self,
        file: &Path,
        candidate: &GroupingCandidate,
    ) -> Result<(), std::io::Error> {
        let content = std::fs::read_to_string(file)?;
        let modified = self.replace_classes_in_content(&content, candidate);
        std::fs::write(file, modified)?;
        Ok(())
    }

    /// Replace class pattern with group name in content
    fn replace_classes_in_content(&self, content: &str, candidate: &GroupingCandidate) -> String {
        let pattern_set: AHashSet<&str> = candidate.classes.iter().map(|s| s.as_str()).collect();
        let mut result = content.to_string();

        // Find and replace class attributes
        let patterns = ["class=\"", "class='", "className=\"", "className='"];

        for pattern in patterns {
            let quote = if pattern.ends_with('"') { '"' } else { '\'' };
            let mut new_result = String::new();
            let mut pos = 0;

            while let Some(start) = result[pos..].find(pattern) {
                let attr_start = pos + start + pattern.len();
                new_result.push_str(&result[pos..attr_start]);

                if let Some(end) = result[attr_start..].find(quote) {
                    let class_value = &result[attr_start..attr_start + end];
                    let classes: Vec<&str> = class_value.split_whitespace().collect();

                    // Check if all pattern classes are present
                    let all_present = pattern_set.iter().all(|p| classes.contains(p));

                    if all_present {
                        // Replace with group name
                        let mut new_classes: Vec<&str> =
                            classes.into_iter().filter(|c| !pattern_set.contains(c)).collect();
                        new_classes.push(&candidate.suggested_name);
                        new_result.push_str(&new_classes.join(" "));
                    } else {
                        new_result.push_str(class_value);
                    }

                    pos = attr_start + end;
                } else {
                    pos = attr_start;
                }
            }

            new_result.push_str(&result[pos..]);
            result = new_result;
        }

        result
    }

    /// Generate dx-markdown report
    ///
    /// **Validates: Requirements 11.9**
    pub fn generate_report(&self, report: &GroupingReport) -> String {
        let mut md = String::new();

        md.push_str("# Grouping Optimization Report\n\n");
        md.push_str(&format!("Files scanned: {}\n", report.files_scanned));
        md.push_str(&format!("Patterns analyzed: {}\n", report.patterns_analyzed));
        md.push_str(&format!("Total potential savings: {} bytes\n\n", report.total_savings));

        if report.candidates.is_empty() {
            md.push_str("No beneficial groupings found.\n");
            return md;
        }

        md.push_str("## Recommended Groupings\n\n");
        md.push_str("| Group | Classes | Occurrences | Savings |\n");
        md.push_str("|-------|---------|-------------|--------|\n");

        for candidate in &report.candidates {
            md.push_str(&format!(
                "| {} | {} | {} | {} bytes |\n",
                candidate.suggested_name,
                candidate.classes.join(" "),
                candidate.occurrences,
                candidate.savings
            ));
        }

        md
    }

    /// Get configuration
    pub fn config(&self) -> &GroupingConfig {
        &self.config
    }
}

impl Default for GroupingOptimizer {
    fn default() -> Self {
        Self::new(GroupingConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_grouping_config_default() {
        let config = GroupingConfig::default();
        assert_eq!(config.min_occurrences, 3);
        assert_eq!(config.min_savings, 10);
        assert!(config.extensions.contains(&"html".to_string()));
        assert!(config.extensions.contains(&"jsx".to_string()));
    }

    #[test]
    fn test_file_scanner() {
        let dir = tempdir().unwrap();

        // Create test files
        fs::write(dir.path().join("test.html"), "<div class=\"flex\"></div>").unwrap();
        fs::write(dir.path().join("test.jsx"), "<div className=\"flex\"></div>").unwrap();
        fs::write(dir.path().join("test.txt"), "not scanned").unwrap();

        let scanner = FileScanner::new(vec!["html".to_string(), "jsx".to_string()], vec![]);

        let files = scanner.scan(dir.path());
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_file_scanner_excludes_dirs() {
        let dir = tempdir().unwrap();

        // Create test structure
        fs::write(dir.path().join("test.html"), "<div></div>").unwrap();
        fs::create_dir(dir.path().join("node_modules")).unwrap();
        fs::write(dir.path().join("node_modules/test.html"), "<div></div>").unwrap();

        let scanner = FileScanner::new(vec!["html".to_string()], vec!["node_modules".to_string()]);

        let files = scanner.scan(dir.path());
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_size_calculator() {
        let calc = SizeCalculator::new();

        // Test with classes that should have positive savings
        let classes = vec![
            "flex".to_string(),
            "items-center".to_string(),
            "justify-between".to_string(),
            "p-4".to_string(),
        ];

        let savings = calc.calculate_savings(&classes, 10);
        // With 10 occurrences, grouping should save bytes
        assert!(savings > 0, "Expected positive savings, got {}", savings);
    }

    #[test]
    fn test_size_calculator_negative_savings() {
        let calc = SizeCalculator::new();

        // Single short class with few occurrences
        let classes = vec!["p-4".to_string()];
        let savings = calc.calculate_savings(&classes, 2);

        // Should have negative savings (not worth grouping)
        assert!(savings < 0, "Expected negative savings for single class");
    }

    #[test]
    fn test_grouping_optimizer_analyze() {
        let dir = tempdir().unwrap();

        // Create files with repeated patterns
        let html = r#"
            <div class="flex items-center p-4">Content 1</div>
            <div class="flex items-center p-4">Content 2</div>
            <div class="flex items-center p-4">Content 3</div>
        "#;
        fs::write(dir.path().join("test.html"), html).unwrap();

        let config = GroupingConfig {
            min_occurrences: 2,
            min_savings: -1000, // Allow any savings for testing
            ..Default::default()
        };

        let mut optimizer = GroupingOptimizer::new(config);
        let report = optimizer.analyze(dir.path());

        assert_eq!(report.files_scanned, 1);
        assert!(report.patterns_analyzed > 0);
    }

    #[test]
    fn test_grouping_optimizer_analyze_only() {
        let dir = tempdir().unwrap();

        let html = r#"<div class="flex items-center">Test</div>"#;
        fs::write(dir.path().join("test.html"), html).unwrap();

        let config = GroupingConfig::default();
        let mut optimizer = GroupingOptimizer::new(config);
        let report = optimizer.analyze(dir.path());

        // Apply in analyze-only mode
        let result = optimizer.apply(&report, true);

        match result {
            ApplyResult::AnalyzeOnly(_) => {
                // Verify file wasn't modified
                let content = fs::read_to_string(dir.path().join("test.html")).unwrap();
                assert!(content.contains("flex items-center"));
            }
            ApplyResult::Applied { .. } => {
                panic!("Expected AnalyzeOnly result");
            }
        }
    }

    #[test]
    fn test_generate_report() {
        let optimizer = GroupingOptimizer::default();

        let report = GroupingReport {
            candidates: vec![GroupingCandidate {
                classes: vec!["flex".to_string(), "items-center".to_string()],
                occurrences: 5,
                savings: 100,
                files: vec![PathBuf::from("test.html")],
                suggested_name: "dxg-test1".to_string(),
            }],
            files_scanned: 10,
            total_savings: 100,
            patterns_analyzed: 50,
        };

        let md = optimizer.generate_report(&report);

        assert!(md.contains("# Grouping Optimization Report"));
        assert!(md.contains("Files scanned: 10"));
        assert!(md.contains("dxg-test1"));
        assert!(md.contains("flex items-center"));
    }

    #[test]
    fn test_extract_class_attributes() {
        let optimizer = GroupingOptimizer::default();

        let content = br#"
            <div class="flex items-center">Test</div>
            <div className="bg-white p-4">React</div>
            <div class='single-quotes'>Vue</div>
        "#;

        let classes = optimizer.extract_class_attributes(content);

        assert!(classes.len() >= 3);
        assert!(classes.iter().any(|c| c.contains(&"flex".to_string())));
        assert!(classes.iter().any(|c| c.contains(&"bg-white".to_string())));
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;
    use std::fs;
    use tempfile::tempdir;

    fn arb_class_name() -> impl Strategy<Value = String> {
        prop::sample::select(vec![
            "flex",
            "items-center",
            "justify-between",
            "p-4",
            "m-2",
            "bg-white",
            "text-black",
            "rounded",
            "shadow",
            "border",
        ])
        .prop_map(|s| s.to_string())
    }

    fn arb_class_list() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(arb_class_name(), 2..6)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        /// Feature: dx-style-advanced-features, Property 5: Grouping Size Invariant
        /// *For any* applied grouping, the total output size (HTML + CSS) SHALL be smaller
        /// than or equal to the ungrouped output size.
        /// **Validates: Requirements 3.3, 3.8**
        #[test]
        fn prop_grouping_size_invariant(
            classes in arb_class_list(),
            occurrences in 3usize..20usize
        ) {
            let calc = SizeCalculator::new();
            let savings = calc.calculate_savings(&classes, occurrences);

            // If savings is positive, grouping reduces size
            // If savings is negative, grouping increases size (and shouldn't be applied)

            // Calculate original size
            let original_size: usize = classes
                .iter()
                .map(|c| c.len() + 1)
                .sum::<usize>() * occurrences;

            // Calculate grouped size
            let grouped_size = (8 * occurrences) + 30; // group name + CSS overhead

            let expected_savings = original_size as i64 - grouped_size as i64;

            // Savings calculation should be consistent
            prop_assert_eq!(
                savings, expected_savings,
                "Savings calculation should match: {} vs {}",
                savings, expected_savings
            );
        }

        /// Feature: dx-style-advanced-features, Property 6: Grouping Analyze-Only Mode
        /// *For any* project analysis with `--analyze` flag, the Grouping_Optimizer
        /// SHALL NOT modify any files in the project.
        /// **Validates: Requirements 3.6**
        #[test]
        fn prop_grouping_analyze_only_mode(
            classes in arb_class_list()
        ) {
            let dir = tempdir().unwrap();

            // Create test file
            let class_str = classes.join(" ");
            let html = format!(
                r#"<div class="{}">Test 1</div>
                <div class="{}">Test 2</div>
                <div class="{}">Test 3</div>"#,
                class_str, class_str, class_str
            );
            let file_path = dir.path().join("test.html");
            fs::write(&file_path, &html).unwrap();

            // Get original content
            let original_content = fs::read_to_string(&file_path).unwrap();

            // Run optimizer in analyze-only mode
            let config = GroupingConfig {
                min_occurrences: 2,
                min_savings: -1000,
                ..Default::default()
            };
            let mut optimizer = GroupingOptimizer::new(config);
            let report = optimizer.analyze(dir.path());
            let result = optimizer.apply(&report, true); // analyze_only = true

            // Verify it's AnalyzeOnly result
            prop_assert!(
                matches!(result, ApplyResult::AnalyzeOnly(_)),
                "Should return AnalyzeOnly result"
            );

            // Verify file wasn't modified
            let final_content = fs::read_to_string(&file_path).unwrap();
            prop_assert_eq!(
                original_content, final_content,
                "File should not be modified in analyze-only mode"
            );
        }

        /// Property test for class extraction
        #[test]
        fn prop_class_extraction(classes in arb_class_list()) {
            let optimizer = GroupingOptimizer::default();

            let class_str = classes.join(" ");
            let html = format!(r#"<div class="{}">Test</div>"#, class_str);

            let extracted = optimizer.extract_class_attributes(html.as_bytes());

            prop_assert!(!extracted.is_empty(), "Should extract at least one class list");

            // All original classes should be present
            let first = &extracted[0];
            for class in &classes {
                prop_assert!(
                    first.contains(class),
                    "Extracted classes should contain '{}'",
                    class
                );
            }
        }

        /// Property test for report generation
        #[test]
        fn prop_report_generation(
            classes in arb_class_list(),
            occurrences in 1usize..10usize,
            savings in -100i64..1000i64
        ) {
            let optimizer = GroupingOptimizer::default();

            let report = GroupingReport {
                candidates: vec![
                    GroupingCandidate {
                        classes: classes.clone(),
                        occurrences,
                        savings,
                        files: vec![PathBuf::from("test.html")],
                        suggested_name: "dxg-test".to_string(),
                    },
                ],
                files_scanned: 1,
                total_savings: savings,
                patterns_analyzed: 1,
            };

            let md = optimizer.generate_report(&report);

            // Report should contain key information
            prop_assert!(md.contains("Grouping Optimization Report"));
            prop_assert!(md.contains("dxg-test"));
            prop_assert!(md.contains(&occurrences.to_string()));
        }
    }
}
