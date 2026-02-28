//! Test Discovery
//!
//! Detects test frameworks and discovers test files and cases.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Supported test frameworks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TestFramework {
    // Rust
    CargoTest,
    // JavaScript/TypeScript
    Jest,
    Vitest,
    Mocha,
    Ava,
    Playwright,
    Cypress,
    // Python
    Pytest,
    Unittest,
    // Go
    GoTest,
    // Other
    Custom,
}

impl TestFramework {
    /// Get framework name
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            TestFramework::CargoTest => "cargo test",
            TestFramework::Jest => "jest",
            TestFramework::Vitest => "vitest",
            TestFramework::Mocha => "mocha",
            TestFramework::Ava => "ava",
            TestFramework::Playwright => "playwright",
            TestFramework::Cypress => "cypress",
            TestFramework::Pytest => "pytest",
            TestFramework::Unittest => "unittest",
            TestFramework::GoTest => "go test",
            TestFramework::Custom => "custom",
        }
    }

    /// Get command to run tests
    #[must_use]
    pub fn run_command(&self) -> &'static str {
        match self {
            TestFramework::CargoTest => "cargo test",
            TestFramework::Jest => "npx jest",
            TestFramework::Vitest => "npx vitest run",
            TestFramework::Mocha => "npx mocha",
            TestFramework::Ava => "npx ava",
            TestFramework::Playwright => "npx playwright test",
            TestFramework::Cypress => "npx cypress run",
            TestFramework::Pytest => "pytest",
            TestFramework::Unittest => "python -m unittest discover",
            TestFramework::GoTest => "go test ./...",
            TestFramework::Custom => "",
        }
    }

    /// Get coverage command
    #[must_use]
    pub fn coverage_command(&self) -> Option<&'static str> {
        match self {
            TestFramework::CargoTest => Some("cargo llvm-cov"),
            TestFramework::Jest => Some("npx jest --coverage"),
            TestFramework::Vitest => Some("npx vitest run --coverage"),
            TestFramework::Pytest => Some("pytest --cov"),
            TestFramework::GoTest => Some("go test -cover ./..."),
            _ => None,
        }
    }
}

/// A discovered test file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFile {
    pub path: PathBuf,
    pub framework: TestFramework,
    pub test_count: usize,
    pub cases: Vec<TestCase>,
}

/// A single test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub name: String,
    pub line: u32,
    pub is_async: bool,
    pub tags: Vec<String>,
}

/// Test discovery engine
pub struct TestDiscovery {
    root: PathBuf,
    detected_frameworks: Vec<TestFramework>,
    test_files: Vec<TestFile>,
}

impl TestDiscovery {
    /// Create new discovery for a project root
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            detected_frameworks: Vec::new(),
            test_files: Vec::new(),
        }
    }

    /// Detect test frameworks in the project
    pub fn detect_frameworks(&mut self) -> &[TestFramework] {
        self.detected_frameworks.clear();

        // Rust: Check for Cargo.toml with test dependencies
        if self.root.join("Cargo.toml").exists() {
            self.detected_frameworks.push(TestFramework::CargoTest);
        }

        // JavaScript/TypeScript: Check package.json
        if let Ok(content) = std::fs::read_to_string(self.root.join("package.json")) {
            if content.contains("\"jest\"") || content.contains("jest.config") {
                self.detected_frameworks.push(TestFramework::Jest);
            }
            if content.contains("\"vitest\"") || self.root.join("vitest.config.ts").exists() {
                self.detected_frameworks.push(TestFramework::Vitest);
            }
            if content.contains("\"mocha\"") {
                self.detected_frameworks.push(TestFramework::Mocha);
            }
            if content.contains("\"ava\"") {
                self.detected_frameworks.push(TestFramework::Ava);
            }
            if content.contains("\"playwright\"") || content.contains("@playwright/test") {
                self.detected_frameworks.push(TestFramework::Playwright);
            }
            if content.contains("\"cypress\"") {
                self.detected_frameworks.push(TestFramework::Cypress);
            }
        }

        // Python: Check for pytest.ini, setup.cfg, or pyproject.toml
        if self.root.join("pytest.ini").exists()
            || self.root.join("pyproject.toml").exists()
            || self.root.join("setup.cfg").exists()
        {
            // Check if pytest is configured
            if let Ok(content) = std::fs::read_to_string(self.root.join("pyproject.toml")) {
                if content.contains("[tool.pytest") {
                    self.detected_frameworks.push(TestFramework::Pytest);
                }
            } else {
                // Default to pytest if test files exist
                self.detected_frameworks.push(TestFramework::Pytest);
            }
        }

        // Go: Check for _test.go files
        if self.root.join("go.mod").exists() {
            self.detected_frameworks.push(TestFramework::GoTest);
        }

        &self.detected_frameworks
    }

    /// Discover test files
    pub fn discover_tests(&mut self) -> &[TestFile] {
        self.test_files.clear();

        if self.detected_frameworks.is_empty() {
            self.detect_frameworks();
        }

        for framework in &self.detected_frameworks.clone() {
            let files = self.find_test_files(*framework);
            self.test_files.extend(files);
        }

        &self.test_files
    }

    /// Find test files for a specific framework
    fn find_test_files(&self, framework: TestFramework) -> Vec<TestFile> {
        let mut files = Vec::new();

        let patterns = match framework {
            TestFramework::CargoTest => vec!["**/src/**/*.rs", "**/tests/**/*.rs"],
            TestFramework::Jest
            | TestFramework::Vitest
            | TestFramework::Mocha
            | TestFramework::Ava => {
                vec![
                    "**/*.test.js",
                    "**/*.test.ts",
                    "**/*.spec.js",
                    "**/*.spec.ts",
                    "**/test/**/*.js",
                    "**/test/**/*.ts",
                    "**/__tests__/**/*.js",
                    "**/__tests__/**/*.ts",
                ]
            }
            TestFramework::Playwright => {
                vec!["**/tests/**/*.spec.ts", "**/*.spec.ts", "**/e2e/**/*.ts"]
            }
            TestFramework::Cypress => vec!["**/cypress/**/*.cy.js", "**/cypress/**/*.cy.ts"],
            TestFramework::Pytest | TestFramework::Unittest => {
                vec!["**/test_*.py", "**/*_test.py", "**/tests/**/*.py"]
            }
            TestFramework::GoTest => vec!["**/*_test.go"],
            TestFramework::Custom => vec![],
        };

        for pattern in patterns {
            if let Ok(entries) = glob::glob(&self.root.join(pattern).to_string_lossy()) {
                for entry in entries.flatten() {
                    if let Some(test_file) = self.parse_test_file(&entry, framework) {
                        files.push(test_file);
                    }
                }
            }
        }

        files
    }

    /// Parse a test file to extract test cases
    fn parse_test_file(&self, path: &Path, framework: TestFramework) -> Option<TestFile> {
        let content = std::fs::read_to_string(path).ok()?;
        let cases = self.extract_test_cases(&content, framework);

        if cases.is_empty() {
            // For Rust files, check if it has #[test] or #[cfg(test)]
            if framework == TestFramework::CargoTest
                && !content.contains("#[test]")
                && !content.contains("#[cfg(test)]")
            {
                return None;
            }
        }

        Some(TestFile {
            path: path.to_path_buf(),
            framework,
            test_count: cases.len(),
            cases,
        })
    }

    /// Extract test cases from file content
    fn extract_test_cases(&self, content: &str, framework: TestFramework) -> Vec<TestCase> {
        let mut cases = Vec::new();

        match framework {
            TestFramework::CargoTest => {
                // Find #[test] functions
                let re = regex::Regex::new(r"#\[test\]\s*(?:#\[.*\]\s*)*(?:async\s+)?fn\s+(\w+)")
                    .unwrap();
                for (line_num, line) in content.lines().enumerate() {
                    if line.trim().starts_with("#[test]") {
                        // Look for function name in next few lines
                        let remaining =
                            content.lines().skip(line_num).take(5).collect::<Vec<_>>().join("\n");
                        if let Some(caps) = re.captures(&remaining) {
                            cases.push(TestCase {
                                name: caps[1].to_string(),
                                line: (line_num + 1) as u32,
                                is_async: remaining.contains("async fn"),
                                tags: Vec::new(),
                            });
                        }
                    }
                }
            }
            TestFramework::Jest
            | TestFramework::Vitest
            | TestFramework::Mocha
            | TestFramework::Ava => {
                // Find test(), it(), describe() calls
                let re = regex::Regex::new(r#"(?:test|it|describe)\s*\(\s*['"`]([^'"`]+)['"`]"#)
                    .unwrap();
                for (line_num, line) in content.lines().enumerate() {
                    if let Some(caps) = re.captures(line) {
                        cases.push(TestCase {
                            name: caps[1].to_string(),
                            line: (line_num + 1) as u32,
                            is_async: line.contains("async"),
                            tags: Vec::new(),
                        });
                    }
                }
            }
            TestFramework::Pytest => {
                // Find def test_ or async def test_ functions
                let re = regex::Regex::new(r"(?:async\s+)?def\s+(test_\w+)").unwrap();
                for (line_num, line) in content.lines().enumerate() {
                    if let Some(caps) = re.captures(line) {
                        cases.push(TestCase {
                            name: caps[1].to_string(),
                            line: (line_num + 1) as u32,
                            is_async: line.contains("async def"),
                            tags: Vec::new(),
                        });
                    }
                }
            }
            TestFramework::GoTest => {
                // Find func Test* functions
                let re = regex::Regex::new(r"func\s+(Test\w+)\s*\(").unwrap();
                for (line_num, line) in content.lines().enumerate() {
                    if let Some(caps) = re.captures(line) {
                        cases.push(TestCase {
                            name: caps[1].to_string(),
                            line: (line_num + 1) as u32,
                            is_async: false,
                            tags: Vec::new(),
                        });
                    }
                }
            }
            _ => {}
        }

        cases
    }

    /// Get total test count
    #[must_use]
    pub fn total_tests(&self) -> usize {
        self.test_files.iter().map(|f| f.test_count).sum()
    }

    /// Get test files by framework
    pub fn files_by_framework(&self) -> HashMap<TestFramework, Vec<&TestFile>> {
        let mut map = HashMap::new();
        for file in &self.test_files {
            map.entry(file.framework).or_insert_with(Vec::new).push(file);
        }
        map
    }

    /// Convert to DX Serializer format
    #[must_use]
    pub fn to_dx_format(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("root={}", self.root.display()));
        lines.push(format!("frameworks:{}[", self.detected_frameworks.len()));
        for framework in &self.detected_frameworks {
            lines.push(format!("  {};", framework.name()));
        }
        lines.push("]".to_string());

        lines.push(format!("total_tests={}", self.total_tests()));
        lines.push(format!("files:{}[", self.test_files.len()));

        for file in &self.test_files {
            let file_str = file.path.to_string_lossy().replace(' ', "_");
            lines.push(format!("  {} {} {}[", file_str, file.framework.name(), file.test_count));
            for case in &file.cases {
                let async_marker = if case.is_async { "async" } else { "sync" };
                lines.push(format!("    {} {} L{};", case.name, async_marker, case.line));
            }
            lines.push("  ];".to_string());
        }

        lines.push("]".to_string());
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_framework_detection() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

        let mut discovery = TestDiscovery::new(temp.path());
        let frameworks = discovery.detect_frameworks();

        assert!(frameworks.contains(&TestFramework::CargoTest));
    }

    #[test]
    fn test_extract_rust_tests() {
        let discovery = TestDiscovery::new(".");
        let content = r#"
            #[test]
            fn test_basic() {
                assert!(true);
            }

            #[test]
            async fn test_async() {
                assert!(true);
            }
        "#;

        let cases = discovery.extract_test_cases(content, TestFramework::CargoTest);
        assert_eq!(cases.len(), 2);
        assert!(cases.iter().any(|c| c.name == "test_basic"));
        assert!(cases.iter().any(|c| c.name == "test_async" && c.is_async));
    }

    #[test]
    fn test_extract_jest_tests() {
        let discovery = TestDiscovery::new(".");
        let content = r#"
            describe('MyComponent', () => {
                test('should render', () => {
                    expect(true).toBe(true);
                });

                it('should handle click', async () => {
                    await click();
                });
            });
        "#;

        let cases = discovery.extract_test_cases(content, TestFramework::Jest);
        assert!(cases.len() >= 2);
    }
}
