//! Test262 ECMAScript Conformance Test Integration
//!
//! This module provides a test harness for running Test262 tests against
//! the DX-JS runtime. Test262 is the official ECMAScript conformance test suite.
//!
//! **Validates: Requirements 7.1**
//!
//! Target: 95%+ pass rate on applicable tests

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;

/// Test262 test metadata parsed from frontmatter
#[derive(Debug, Clone)]
pub struct Test262Metadata {
    /// Test description
    pub description: String,
    /// Expected error type (if test should throw)
    pub negative: Option<NegativeExpectation>,
    /// Test flags (e.g., onlyStrict, noStrict, module, async)
    pub flags: Vec<String>,
    /// Features required by this test
    pub features: Vec<String>,
    /// ES version this test targets
    pub es_version: Option<String>,
    /// Includes required (harness files)
    pub includes: Vec<String>,
}

/// Expected error for negative tests
#[derive(Debug, Clone)]
pub struct NegativeExpectation {
    /// Phase when error should occur (parse, resolution, runtime)
    pub phase: String,
    /// Expected error type
    pub error_type: String,
}

/// Result of running a single Test262 test
#[derive(Debug, Clone)]
pub enum Test262Result {
    /// Test passed
    Pass,
    /// Test failed with reason
    Fail(String),
    /// Test was skipped (unsupported feature)
    Skip(String),
    /// Test timed out
    Timeout,
}

/// Test262 test runner configuration
#[derive(Debug, Clone)]
pub struct Test262Config {
    /// Path to Test262 repository
    pub test262_path: PathBuf,
    /// Timeout for each test in milliseconds
    pub timeout_ms: u64,
    /// Features to skip (not yet implemented)
    pub skip_features: Vec<String>,
    /// Whether to run async tests
    pub run_async: bool,
    /// Whether to run module tests
    pub run_modules: bool,
}

impl Default for Test262Config {
    fn default() -> Self {
        Self {
            test262_path: PathBuf::from("test262"),
            timeout_ms: 10000,
            skip_features: vec![
                // Features not yet implemented in DX-JS
                "Atomics".to_string(),
                "SharedArrayBuffer".to_string(),
                "WeakRef".to_string(),
                "FinalizationRegistry".to_string(),
                "Temporal".to_string(),
                "decorators".to_string(),
                "import-assertions".to_string(),
                "import-attributes".to_string(),
            ],
            run_async: true,
            run_modules: true,
        }
    }
}

/// Test262 test harness
pub struct Test262Harness {
    config: Test262Config,
    /// Harness file contents (sta.js, assert.js, etc.)
    harness_files: HashMap<String, String>,
    /// Test results
    results: Vec<(String, Test262Result)>,
}

impl Test262Harness {
    /// Create a new Test262 harness
    pub fn new(config: Test262Config) -> Result<Self, Test262Error> {
        let harness_path = config.test262_path.join("harness");
        let mut harness_files = HashMap::new();
        
        // Load required harness files
        let required_harness = ["sta.js", "assert.js", "propertyHelper.js", "compareArray.js"];
        
        for file in &required_harness {
            let path = harness_path.join(file);
            if path.exists() {
                let content = fs::read_to_string(&path)
                    .map_err(|e| Test262Error::HarnessLoadError(file.to_string(), e.to_string()))?;
                harness_files.insert(file.to_string(), content);
            }
        }
        
        Ok(Self {
            config,
            harness_files,
            results: Vec::new(),
        })
    }
    
    /// Parse Test262 frontmatter from test file
    pub fn parse_metadata(content: &str) -> Option<Test262Metadata> {
        // Find YAML frontmatter between /*--- and ---*/
        let start = content.find("/*---")?;
        let end = content.find("---*/")?;
        
        if start >= end {
            return None;
        }
        
        let yaml_content = &content[start + 5..end];
        
        // Simple YAML parsing for Test262 metadata
        let mut metadata = Test262Metadata {
            description: String::new(),
            negative: None,
            flags: Vec::new(),
            features: Vec::new(),
            es_version: None,
            includes: Vec::new(),
        };
        
        for line in yaml_content.lines() {
            let line = line.trim();
            
            if line.starts_with("description:") {
                metadata.description = line[12..].trim().trim_matches(|c| c == '"' || c == '\'').to_string();
            } else if line.starts_with("flags:") {
                // Parse array on same line or following lines
                let flags_str = line[6..].trim();
                if flags_str.starts_with('[') {
                    metadata.flags = parse_yaml_array(flags_str);
                }
            } else if line.starts_with("features:") {
                let features_str = line[9..].trim();
                if features_str.starts_with('[') {
                    metadata.features = parse_yaml_array(features_str);
                }
            } else if line.starts_with("includes:") {
                let includes_str = line[9..].trim();
                if includes_str.starts_with('[') {
                    metadata.includes = parse_yaml_array(includes_str);
                }
            } else if line.starts_with("- ") && !metadata.flags.is_empty() {
                // Continuation of array
                metadata.flags.push(line[2..].trim().to_string());
            } else if line.starts_with("negative:") {
                // Parse negative expectation
                metadata.negative = Some(NegativeExpectation {
                    phase: String::new(),
                    error_type: String::new(),
                });
            } else if line.starts_with("phase:") {
                if let Some(ref mut neg) = metadata.negative {
                    neg.phase = line[6..].trim().to_string();
                }
            } else if line.starts_with("type:") {
                if let Some(ref mut neg) = metadata.negative {
                    neg.error_type = line[5..].trim().to_string();
                }
            }
        }
        
        Some(metadata)
    }
    
    /// Check if a test should be skipped based on features
    pub fn should_skip(&self, metadata: &Test262Metadata) -> Option<String> {
        // Check for unsupported features
        for feature in &metadata.features {
            if self.config.skip_features.contains(feature) {
                return Some(format!("Unsupported feature: {}", feature));
            }
        }
        
        // Check for module tests if not enabled
        if !self.config.run_modules && metadata.flags.contains(&"module".to_string()) {
            return Some("Module tests disabled".to_string());
        }
        
        // Check for async tests if not enabled
        if !self.config.run_async && metadata.flags.contains(&"async".to_string()) {
            return Some("Async tests disabled".to_string());
        }
        
        None
    }
    
    /// Build the test script with harness
    pub fn build_test_script(&self, test_content: &str, metadata: &Test262Metadata) -> String {
        let mut script = String::new();
        
        // Add required harness files
        for include in &metadata.includes {
            if let Some(harness_content) = self.harness_files.get(include) {
                script.push_str(harness_content);
                script.push('\n');
            }
        }
        
        // Always include sta.js and assert.js
        if let Some(sta) = self.harness_files.get("sta.js") {
            script.push_str(sta);
            script.push('\n');
        }
        if let Some(assert) = self.harness_files.get("assert.js") {
            script.push_str(assert);
            script.push('\n');
        }
        
        // Add the test content (strip frontmatter)
        if let Some(end) = test_content.find("---*/") {
            script.push_str(&test_content[end + 5..]);
        } else {
            script.push_str(test_content);
        }
        
        script
    }
    
    /// Run a single Test262 test
    pub fn run_test(&mut self, test_path: &Path) -> Test262Result {
        // Read test file
        let content = match fs::read_to_string(test_path) {
            Ok(c) => c,
            Err(e) => return Test262Result::Fail(format!("Failed to read test: {}", e)),
        };
        
        // Parse metadata
        let metadata = match Self::parse_metadata(&content) {
            Some(m) => m,
            None => return Test262Result::Fail("Failed to parse test metadata".to_string()),
        };
        
        // Check if should skip
        if let Some(reason) = self.should_skip(&metadata) {
            return Test262Result::Skip(reason);
        }
        
        // Build test script
        let _script = self.build_test_script(&content, &metadata);
        
        // TODO: Execute script with DX-JS runtime
        // For now, return a placeholder result
        // In a real implementation, this would:
        // 1. Create a new runtime instance
        // 2. Execute the script with timeout
        // 3. Check for expected errors (negative tests)
        // 4. Return Pass/Fail based on execution result
        
        Test262Result::Skip("Runtime execution not yet implemented".to_string())
    }
    
    /// Run all tests in a directory
    pub fn run_directory(&mut self, dir: &Path) -> Test262Summary {
        let mut summary = Test262Summary::default();
        
        if !dir.exists() {
            return summary;
        }
        
        // Walk directory recursively
        for entry in walkdir(dir) {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "js") {
                    let result = self.run_test(path);
                    let test_name = path.to_string_lossy().to_string();
                    
                    match &result {
                        Test262Result::Pass => summary.passed += 1,
                        Test262Result::Fail(_) => summary.failed += 1,
                        Test262Result::Skip(_) => summary.skipped += 1,
                        Test262Result::Timeout => summary.timeout += 1,
                    }
                    
                    self.results.push((test_name, result));
                }
            }
        }
        
        summary
    }
    
    /// Get pass rate as percentage
    pub fn pass_rate(&self) -> f64 {
        let total = self.results.iter()
            .filter(|(_, r)| !matches!(r, Test262Result::Skip(_)))
            .count();
        
        if total == 0 {
            return 0.0;
        }
        
        let passed = self.results.iter()
            .filter(|(_, r)| matches!(r, Test262Result::Pass))
            .count();
        
        (passed as f64 / total as f64) * 100.0
    }
    
    /// Get test results
    pub fn results(&self) -> &[(String, Test262Result)] {
        &self.results
    }
}

/// Summary of Test262 run
#[derive(Debug, Default)]
pub struct Test262Summary {
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub timeout: usize,
}

impl Test262Summary {
    pub fn total(&self) -> usize {
        self.passed + self.failed + self.skipped + self.timeout
    }
    
    pub fn pass_rate(&self) -> f64 {
        let applicable = self.passed + self.failed + self.timeout;
        if applicable == 0 {
            return 0.0;
        }
        (self.passed as f64 / applicable as f64) * 100.0
    }
}

/// Test262 error types
#[derive(Debug)]
pub enum Test262Error {
    /// Failed to load harness file
    HarnessLoadError(String, String),
    /// Test262 repository not found
    RepoNotFound(PathBuf),
    /// Test execution error
    ExecutionError(String),
}

impl std::fmt::Display for Test262Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Test262Error::HarnessLoadError(file, err) => {
                write!(f, "Failed to load harness file '{}': {}", file, err)
            }
            Test262Error::RepoNotFound(path) => {
                write!(f, "Test262 repository not found at: {}", path.display())
            }
            Test262Error::ExecutionError(err) => {
                write!(f, "Test execution error: {}", err)
            }
        }
    }
}

impl std::error::Error for Test262Error {}

/// Parse a simple YAML array like [a, b, c]
fn parse_yaml_array(s: &str) -> Vec<String> {
    let s = s.trim();
    if !s.starts_with('[') || !s.ends_with(']') {
        return Vec::new();
    }
    
    s[1..s.len()-1]
        .split(',')
        .map(|item| item.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Simple directory walker (avoiding external dependency)
fn walkdir(dir: &Path) -> impl Iterator<Item = Result<DirEntry, std::io::Error>> {
    WalkDir::new(dir)
}

struct WalkDir {
    stack: Vec<PathBuf>,
}

impl WalkDir {
    fn new(path: &Path) -> Self {
        Self {
            stack: vec![path.to_path_buf()],
        }
    }
}

struct DirEntry {
    path: PathBuf,
}

impl DirEntry {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Iterator for WalkDir {
    type Item = Result<DirEntry, std::io::Error>;
    
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(path) = self.stack.pop() {
            if path.is_dir() {
                match fs::read_dir(&path) {
                    Ok(entries) => {
                        for entry in entries {
                            if let Ok(entry) = entry {
                                self.stack.push(entry.path());
                            }
                        }
                    }
                    Err(e) => return Some(Err(e)),
                }
            } else {
                return Some(Ok(DirEntry { path }));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_yaml_array() {
        assert_eq!(parse_yaml_array("[a, b, c]"), vec!["a", "b", "c"]);
        assert_eq!(parse_yaml_array("['foo', 'bar']"), vec!["foo", "bar"]);
        assert_eq!(parse_yaml_array("[]"), Vec::<String>::new());
    }
    
    #[test]
    fn test_parse_metadata() {
        let content = r#"/*---
description: Test description
flags: [onlyStrict]
features: [BigInt]
includes: [assert.js]
---*/
var x = 1n;
"#;
        
        let metadata = Test262Harness::parse_metadata(content).unwrap();
        assert_eq!(metadata.description, "Test description");
        assert!(metadata.flags.contains(&"onlyStrict".to_string()));
        assert!(metadata.features.contains(&"BigInt".to_string()));
        assert!(metadata.includes.contains(&"assert.js".to_string()));
    }
    
    #[test]
    fn test_parse_negative_metadata() {
        let content = r#"/*---
description: Test that should throw
negative:
  phase: runtime
  type: TypeError
---*/
throw new TypeError();
"#;
        
        let metadata = Test262Harness::parse_metadata(content).unwrap();
        assert!(metadata.negative.is_some());
        let neg = metadata.negative.unwrap();
        assert_eq!(neg.phase, "runtime");
        assert_eq!(neg.error_type, "TypeError");
    }
    
    #[test]
    fn test_summary_pass_rate() {
        let summary = Test262Summary {
            passed: 95,
            failed: 5,
            skipped: 10,
            timeout: 0,
        };
        
        assert!((summary.pass_rate() - 95.0).abs() < 0.01);
    }
}
