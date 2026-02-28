//! Test Runner
//!
//! Executes tests and captures results in DX Serializer format.

use super::{TestDiscovery, TestFramework};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Test execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    TimedOut,
    Error,
}

impl TestStatus {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            TestStatus::Passed => "passed",
            TestStatus::Failed => "failed",
            TestStatus::Skipped => "skipped",
            TestStatus::TimedOut => "timeout",
            TestStatus::Error => "error",
        }
    }
}

/// Result of a single test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String,
    pub file: PathBuf,
    pub line: u32,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub message: Option<String>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
}

/// Collection of test results for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuite {
    pub file: PathBuf,
    pub framework: TestFramework,
    pub results: Vec<TestResult>,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration_ms: u64,
}

impl TestSuite {
    #[must_use]
    pub fn new(file: PathBuf, framework: TestFramework) -> Self {
        Self {
            file,
            framework,
            results: Vec::new(),
            passed: 0,
            failed: 0,
            skipped: 0,
            duration_ms: 0,
        }
    }

    pub fn add_result(&mut self, result: TestResult) {
        match result.status {
            TestStatus::Passed => self.passed += 1,
            TestStatus::Failed | TestStatus::Error | TestStatus::TimedOut => self.failed += 1,
            TestStatus::Skipped => self.skipped += 1,
        }
        self.duration_ms += result.duration_ms;
        self.results.push(result);
    }

    #[must_use]
    pub fn total(&self) -> usize {
        self.passed + self.failed + self.skipped
    }
}

/// Complete test output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestOutput {
    pub suites: Vec<TestSuite>,
    pub total_passed: usize,
    pub total_failed: usize,
    pub total_skipped: usize,
    pub total_duration_ms: u64,
    pub exit_code: i32,
}

impl TestOutput {
    #[must_use]
    pub fn new() -> Self {
        Self {
            suites: Vec::new(),
            total_passed: 0,
            total_failed: 0,
            total_skipped: 0,
            total_duration_ms: 0,
            exit_code: 0,
        }
    }

    pub fn add_suite(&mut self, suite: TestSuite) {
        self.total_passed += suite.passed;
        self.total_failed += suite.failed;
        self.total_skipped += suite.skipped;
        self.total_duration_ms += suite.duration_ms;
        if suite.failed > 0 {
            self.exit_code = 1;
        }
        self.suites.push(suite);
    }

    #[must_use]
    pub fn total(&self) -> usize {
        self.total_passed + self.total_failed + self.total_skipped
    }

    #[must_use]
    pub fn success(&self) -> bool {
        self.total_failed == 0
    }

    /// Convert to DX Serializer format (LLM optimized)
    #[must_use]
    pub fn to_dx_format(&self) -> String {
        let mut lines = Vec::new();

        // Summary
        lines.push(format!("status={}", if self.success() { "pass" } else { "fail" }));
        lines.push(format!("total={}", self.total()));
        lines.push(format!("passed={}", self.total_passed));
        lines.push(format!("failed={}", self.total_failed));
        lines.push(format!("skipped={}", self.total_skipped));
        lines.push(format!("duration={}ms", self.total_duration_ms));

        // Suites with failures only for compact output
        let failed_suites: Vec<_> = self.suites.iter().filter(|s| s.failed > 0).collect();

        if !failed_suites.is_empty() {
            lines.push(format!("failures:{}[", failed_suites.len()));

            for suite in failed_suites {
                let file_str = suite.file.to_string_lossy().replace(' ', "_");
                lines.push(format!("  {file_str}["));

                for result in &suite.results {
                    if result.status != TestStatus::Passed {
                        let msg = result.message.as_deref().unwrap_or("").replace(' ', "_");
                        lines.push(format!(
                            "    {} {} {}ms L{} \"{}\";",
                            result.name,
                            result.status.as_str(),
                            result.duration_ms,
                            result.line,
                            msg
                        ));
                    }
                }

                lines.push("  ];".to_string());
            }

            lines.push("]".to_string());
        }

        lines.join("\n")
    }

    /// Convert to DX Serializer format (human readable)
    #[must_use]
    pub fn to_dx_human_format(&self) -> String {
        let mut output = String::new();

        // Header
        output.push_str(&format!(
            "Test Results: {} | ✓ {} ✗ {} ○ {} | {}ms\n",
            if self.success() { "PASS" } else { "FAIL" },
            self.total_passed,
            self.total_failed,
            self.total_skipped,
            self.total_duration_ms
        ));
        output.push_str(&"─".repeat(60));
        output.push('\n');

        for suite in &self.suites {
            let status = if suite.failed == 0 { "✓" } else { "✗" };
            output.push_str(&format!(
                "{} {} ({}/{} passed)\n",
                status,
                suite.file.display(),
                suite.passed,
                suite.total()
            ));

            for result in &suite.results {
                let icon = match result.status {
                    TestStatus::Passed => "  ✓",
                    TestStatus::Failed => "  ✗",
                    TestStatus::Skipped => "  ○",
                    TestStatus::TimedOut => "  ⏱",
                    TestStatus::Error => "  ⚠",
                };

                output.push_str(&format!("{} {} ({}ms)\n", icon, result.name, result.duration_ms));

                if let Some(msg) = &result.message {
                    for line in msg.lines().take(3) {
                        output.push_str(&format!("      {line}\n"));
                    }
                }
            }
        }

        output
    }
}

impl Default for TestOutput {
    fn default() -> Self {
        Self::new()
    }
}

/// Test runner configuration
#[derive(Debug, Clone)]
pub struct TestRunnerConfig {
    pub timeout: Duration,
    pub parallel: bool,
    pub filter: Option<String>,
    pub verbose: bool,
    pub coverage: bool,
}

impl Default for TestRunnerConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(300),
            parallel: true,
            filter: None,
            verbose: false,
            coverage: false,
        }
    }
}

/// Test runner
pub struct TestRunner {
    root: PathBuf,
    config: TestRunnerConfig,
}

impl TestRunner {
    /// Create a new test runner
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            config: TestRunnerConfig::default(),
        }
    }

    /// Create with custom config
    pub fn with_config(root: impl Into<PathBuf>, config: TestRunnerConfig) -> Self {
        Self {
            root: root.into(),
            config,
        }
    }

    /// Run all discovered tests
    #[must_use]
    pub fn run_all(&self) -> TestOutput {
        let mut discovery = TestDiscovery::new(&self.root);
        discovery.discover_tests();

        let mut output = TestOutput::new();
        let frameworks = discovery.files_by_framework();

        for (framework, _files) in frameworks {
            let suite_output = self.run_framework(framework);
            for suite in suite_output.suites {
                output.add_suite(suite);
            }
        }

        output
    }

    /// Run tests for a specific framework
    #[must_use]
    pub fn run_framework(&self, framework: TestFramework) -> TestOutput {
        let start = Instant::now();

        let command = if self.config.coverage {
            framework.coverage_command().unwrap_or(framework.run_command())
        } else {
            framework.run_command()
        };

        if command.is_empty() {
            return TestOutput::new();
        }

        let mut args: Vec<&str> = command.split_whitespace().collect();
        let program = args.remove(0);

        // Add framework-specific options
        match framework {
            TestFramework::CargoTest => {
                args.extend(["--", "--format", "json", "-Z", "unstable-options"]);
            }
            TestFramework::Jest | TestFramework::Vitest => {
                args.push("--json");
            }
            TestFramework::Pytest => {
                args.extend(["--tb=short", "-q"]);
            }
            _ => {}
        }

        // Add filter if specified
        if let Some(ref filter) = self.config.filter {
            match framework {
                TestFramework::CargoTest => args.push(filter),
                TestFramework::Jest | TestFramework::Vitest => {
                    args.push("-t");
                    args.push(filter);
                }
                TestFramework::Pytest => {
                    args.push("-k");
                    args.push(filter);
                }
                _ => {}
            }
        }

        let result = Command::new(program)
            .args(&args)
            .current_dir(&self.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        let duration = start.elapsed();

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                self.parse_output(framework, &stdout, &stderr, duration)
            }
            Err(e) => {
                let mut test_output = TestOutput::new();
                let mut suite = TestSuite::new(self.root.clone(), framework);
                suite.add_result(TestResult {
                    name: "test_execution".to_string(),
                    file: self.root.clone(),
                    line: 0,
                    status: TestStatus::Error,
                    duration_ms: duration.as_millis() as u64,
                    message: Some(format!("Failed to run tests: {e}")),
                    stdout: None,
                    stderr: None,
                });
                test_output.add_suite(suite);
                test_output
            }
        }
    }

    /// Run specific test file
    #[must_use]
    pub fn run_file(&self, path: &Path) -> TestOutput {
        // Detect framework from file extension and patterns
        let framework = self.detect_file_framework(path);

        let command = framework.run_command();
        if command.is_empty() {
            return TestOutput::new();
        }

        let mut args: Vec<&str> = command.split_whitespace().collect();
        let program = args.remove(0);

        // Add file path
        let file_str = path.to_string_lossy();
        args.push(&file_str);

        let start = Instant::now();
        let result = Command::new(program)
            .args(&args)
            .current_dir(&self.root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

        let duration = start.elapsed();

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                self.parse_output(framework, &stdout, &stderr, duration)
            }
            Err(e) => {
                let mut test_output = TestOutput::new();
                let mut suite = TestSuite::new(path.to_path_buf(), framework);
                suite.add_result(TestResult {
                    name: "test_execution".to_string(),
                    file: path.to_path_buf(),
                    line: 0,
                    status: TestStatus::Error,
                    duration_ms: duration.as_millis() as u64,
                    message: Some(format!("Failed to run tests: {e}")),
                    stdout: None,
                    stderr: None,
                });
                test_output.add_suite(suite);
                test_output
            }
        }
    }

    /// Detect framework from file
    fn detect_file_framework(&self, path: &Path) -> TestFramework {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        match ext {
            "rs" => TestFramework::CargoTest,
            "py" => TestFramework::Pytest,
            "go" => TestFramework::GoTest,
            "js" | "ts" | "jsx" | "tsx" => {
                // Check file name patterns
                if name.contains(".spec.") || name.contains(".test.") {
                    // Check for vitest config in project
                    if self.root.join("vitest.config.ts").exists()
                        || self.root.join("vitest.config.js").exists()
                    {
                        TestFramework::Vitest
                    } else {
                        TestFramework::Jest
                    }
                } else if name.contains(".cy.") {
                    TestFramework::Cypress
                } else if path.to_string_lossy().contains("e2e") {
                    TestFramework::Playwright
                } else {
                    TestFramework::Jest
                }
            }
            _ => TestFramework::Custom,
        }
    }

    /// Parse test output based on framework
    fn parse_output(
        &self,
        framework: TestFramework,
        stdout: &str,
        stderr: &str,
        duration: Duration,
    ) -> TestOutput {
        match framework {
            TestFramework::CargoTest => self.parse_cargo_output(stdout, stderr, duration),
            TestFramework::Jest | TestFramework::Vitest => {
                self.parse_jest_output(stdout, stderr, duration)
            }
            TestFramework::Pytest => self.parse_pytest_output(stdout, stderr, duration),
            TestFramework::GoTest => self.parse_go_output(stdout, stderr, duration),
            _ => self.parse_generic_output(stdout, stderr, duration),
        }
    }

    /// Parse Cargo test output (JSON format)
    fn parse_cargo_output(&self, stdout: &str, _stderr: &str, _duration: Duration) -> TestOutput {
        let mut output = TestOutput::new();
        let mut suites: HashMap<PathBuf, TestSuite> = HashMap::new();

        for line in stdout.lines() {
            if !line.starts_with('{') {
                continue;
            }

            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line)
                && (json["type"] == "test" && json["event"] == "ok" || json["event"] == "failed")
            {
                let name = json["name"].as_str().unwrap_or("unknown");
                let status = if json["event"] == "ok" {
                    TestStatus::Passed
                } else {
                    TestStatus::Failed
                };
                let duration_ms = json["exec_time"].as_f64().map_or(0, |f| (f * 1000.0) as u64);

                let file = PathBuf::from("src/lib.rs"); // Default, ideally parse from name
                let suite = suites
                    .entry(file.clone())
                    .or_insert_with(|| TestSuite::new(file.clone(), TestFramework::CargoTest));

                suite.add_result(TestResult {
                    name: name.to_string(),
                    file,
                    line: 0,
                    status,
                    duration_ms,
                    message: json["stdout"].as_str().map(std::string::ToString::to_string),
                    stdout: None,
                    stderr: None,
                });
            }
        }

        for suite in suites.into_values() {
            output.add_suite(suite);
        }

        output
    }

    /// Parse Jest/Vitest output (JSON format)
    fn parse_jest_output(&self, stdout: &str, _stderr: &str, _duration: Duration) -> TestOutput {
        let mut output = TestOutput::new();

        // Try to parse JSON output
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(stdout)
            && let Some(results) = json["testResults"].as_array()
        {
            for test_file in results {
                let file = PathBuf::from(test_file["name"].as_str().unwrap_or("unknown"));
                let mut suite = TestSuite::new(file.clone(), TestFramework::Jest);

                if let Some(assertions) = test_file["assertionResults"].as_array() {
                    for assertion in assertions {
                        let name = assertion["title"].as_str().unwrap_or("unknown");
                        let status = match assertion["status"].as_str() {
                            Some("passed") => TestStatus::Passed,
                            Some("failed") => TestStatus::Failed,
                            Some("pending" | "skipped") => TestStatus::Skipped,
                            _ => TestStatus::Error,
                        };

                        suite.add_result(TestResult {
                            name: name.to_string(),
                            file: file.clone(),
                            line: 0,
                            status,
                            duration_ms: assertion["duration"].as_u64().unwrap_or(0),
                            message: assertion["failureMessages"].as_array().map(|msgs| {
                                msgs.iter()
                                    .filter_map(|m| m.as_str())
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            }),
                            stdout: None,
                            stderr: None,
                        });
                    }
                }

                output.add_suite(suite);
            }
        }

        output
    }

    /// Parse pytest output
    fn parse_pytest_output(&self, stdout: &str, stderr: &str, _duration: Duration) -> TestOutput {
        let mut output = TestOutput::new();
        let mut current_file: Option<PathBuf> = None;
        let mut suite: Option<TestSuite> = None;

        // Parse pytest short output format
        let re = regex::Regex::new(r"(\S+)::(\S+)\s+(PASSED|FAILED|SKIPPED|ERROR)").unwrap();

        for line in stdout.lines().chain(stderr.lines()) {
            if let Some(caps) = re.captures(line) {
                let file = PathBuf::from(&caps[1]);
                let name = &caps[2];
                let status = match &caps[3] {
                    "PASSED" => TestStatus::Passed,
                    "FAILED" => TestStatus::Failed,
                    "SKIPPED" => TestStatus::Skipped,
                    _ => TestStatus::Error,
                };

                if current_file.as_ref() != Some(&file) {
                    if let Some(s) = suite.take() {
                        output.add_suite(s);
                    }
                    current_file = Some(file.clone());
                    suite = Some(TestSuite::new(file.clone(), TestFramework::Pytest));
                }

                if let Some(ref mut s) = suite {
                    s.add_result(TestResult {
                        name: name.to_string(),
                        file,
                        line: 0,
                        status,
                        duration_ms: 0,
                        message: None,
                        stdout: None,
                        stderr: None,
                    });
                }
            }
        }

        if let Some(s) = suite {
            output.add_suite(s);
        }

        output
    }

    /// Parse Go test output
    fn parse_go_output(&self, stdout: &str, _stderr: &str, _duration: Duration) -> TestOutput {
        let mut output = TestOutput::new();
        let mut suites: HashMap<PathBuf, TestSuite> = HashMap::new();

        let re = regex::Regex::new(r"(---\s+)?(PASS|FAIL|SKIP):\s+(\S+)\s+\(([0-9.]+)s\)").unwrap();

        for line in stdout.lines() {
            if let Some(caps) = re.captures(line) {
                let status = match &caps[2] {
                    "PASS" => TestStatus::Passed,
                    "FAIL" => TestStatus::Failed,
                    "SKIP" => TestStatus::Skipped,
                    _ => TestStatus::Error,
                };
                let name = &caps[3];
                let duration_ms = caps[4].parse::<f64>().map(|f| (f * 1000.0) as u64).unwrap_or(0);

                let file = PathBuf::from("main_test.go"); // Default
                let suite = suites
                    .entry(file.clone())
                    .or_insert_with(|| TestSuite::new(file.clone(), TestFramework::GoTest));

                suite.add_result(TestResult {
                    name: name.to_string(),
                    file,
                    line: 0,
                    status,
                    duration_ms,
                    message: None,
                    stdout: None,
                    stderr: None,
                });
            }
        }

        for suite in suites.into_values() {
            output.add_suite(suite);
        }

        output
    }

    /// Parse generic output (fallback)
    fn parse_generic_output(&self, stdout: &str, stderr: &str, duration: Duration) -> TestOutput {
        let mut output = TestOutput::new();
        let mut suite = TestSuite::new(self.root.clone(), TestFramework::Custom);

        // Simple pass/fail detection
        let all_output = format!("{stdout}\n{stderr}");
        let passed = all_output.contains("PASS")
            || all_output.contains("passed")
            || all_output.contains("ok");
        let failed = all_output.contains("FAIL")
            || all_output.contains("failed")
            || all_output.contains("error");

        suite.add_result(TestResult {
            name: "test_run".to_string(),
            file: self.root.clone(),
            line: 0,
            status: if failed {
                TestStatus::Failed
            } else if passed {
                TestStatus::Passed
            } else {
                TestStatus::Error
            },
            duration_ms: duration.as_millis() as u64,
            message: if failed {
                Some(stderr.to_string())
            } else {
                None
            },
            stdout: Some(stdout.to_string()),
            stderr: Some(stderr.to_string()),
        });

        output.add_suite(suite);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_output_creation() {
        let output = TestOutput::new();
        assert!(output.success());
        assert_eq!(output.total(), 0);
    }

    #[test]
    fn test_suite_tracking() {
        let mut suite = TestSuite::new(PathBuf::from("test.rs"), TestFramework::CargoTest);

        suite.add_result(TestResult {
            name: "test_pass".to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            status: TestStatus::Passed,
            duration_ms: 100,
            message: None,
            stdout: None,
            stderr: None,
        });

        suite.add_result(TestResult {
            name: "test_fail".to_string(),
            file: PathBuf::from("test.rs"),
            line: 20,
            status: TestStatus::Failed,
            duration_ms: 50,
            message: Some("assertion failed".to_string()),
            stdout: None,
            stderr: None,
        });

        assert_eq!(suite.passed, 1);
        assert_eq!(suite.failed, 1);
        assert_eq!(suite.total(), 2);
    }

    #[test]
    fn test_dx_format_output() {
        let mut output = TestOutput::new();
        let mut suite = TestSuite::new(PathBuf::from("test.rs"), TestFramework::CargoTest);

        suite.add_result(TestResult {
            name: "test_example".to_string(),
            file: PathBuf::from("test.rs"),
            line: 10,
            status: TestStatus::Passed,
            duration_ms: 100,
            message: None,
            stdout: None,
            stderr: None,
        });

        output.add_suite(suite);

        let dx_output = output.to_dx_format();
        assert!(dx_output.contains("status=pass"));
        assert!(dx_output.contains("total=1"));
    }
}
