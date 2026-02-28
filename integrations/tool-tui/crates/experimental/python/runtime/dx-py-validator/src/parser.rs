//! Test output parser for pytest and unittest formats

use crate::failure::TestFailure;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during parsing
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Failed to parse test output: {0}")]
    InvalidFormat(String),

    #[error("No test results found in output")]
    NoResults,

    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
}

/// Supported test output formats
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TestFormat {
    /// pytest output format
    #[default]
    Pytest,
    /// unittest output format
    Unittest,
    /// Generic format (best effort parsing)
    Generic,
}

/// Parsed test result
#[derive(Debug, Clone, Default)]
pub struct ParsedTestResult {
    /// Total number of tests
    pub total: usize,
    /// Number of passed tests
    pub passed: usize,
    /// Number of failed tests
    pub failed: usize,
    /// Number of skipped tests
    pub skipped: usize,
    /// Number of error tests
    pub errors: usize,
    /// Individual test failures
    pub failures: Vec<TestFailure>,
}

impl ParsedTestResult {
    /// Calculate pass rate
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        self.passed as f64 / self.total as f64
    }
}

// Regex patterns for pytest output
static PYTEST_SUMMARY: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^=+ (\d+) passed(?:, (\d+) failed)?(?:, (\d+) skipped)?(?:, (\d+) error)?")
        .unwrap()
});

static PYTEST_SHORT_SUMMARY: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)(\d+) passed(?:, (\d+) failed)?(?:, (\d+) skipped)?(?:, (\d+) error)?")
        .unwrap()
});

static PYTEST_FAILURE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^FAILED ([^\s]+)").unwrap());

static PYTEST_ERROR: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^ERROR ([^\s]+)").unwrap());

// Regex patterns for unittest output
static UNITTEST_SUMMARY: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^Ran (\d+) tests? in").unwrap());

#[allow(dead_code)]
static UNITTEST_RESULT: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^(OK|FAILED)").unwrap());

static UNITTEST_FAILURES: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)failures=(\d+)").unwrap());

static UNITTEST_ERRORS: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)errors=(\d+)").unwrap());

static UNITTEST_SKIPPED: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)skipped=(\d+)").unwrap());

/// Parser for test output
pub struct TestResultParser {
    format: TestFormat,
}

impl TestResultParser {
    /// Create a new parser for the given format
    pub fn new(format: TestFormat) -> Self {
        Self { format }
    }

    /// Parse test output and return structured results
    pub fn parse(&self, output: &str) -> Result<ParsedTestResult, ParseError> {
        match self.format {
            TestFormat::Pytest => self.parse_pytest(output),
            TestFormat::Unittest => self.parse_unittest(output),
            TestFormat::Generic => self.parse_generic(output),
        }
    }

    /// Parse pytest output
    fn parse_pytest(&self, output: &str) -> Result<ParsedTestResult, ParseError> {
        let mut result = ParsedTestResult::default();

        // Try to find the summary line
        if let Some(caps) = PYTEST_SUMMARY
            .captures(output)
            .or_else(|| PYTEST_SHORT_SUMMARY.captures(output))
        {
            result.passed = caps.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            result.failed = caps.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            result.skipped = caps.get(3).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            result.errors = caps.get(4).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);

            result.total = result.passed + result.failed + result.skipped + result.errors;
        } else {
            // Try to count individual test results
            result.passed = output.matches(" PASSED").count();
            result.failed = output.matches(" FAILED").count();
            result.skipped = output.matches(" SKIPPED").count();
            result.errors = output.matches(" ERROR").count();
            result.total = result.passed + result.failed + result.skipped + result.errors;
        }

        // Extract individual failures
        for caps in PYTEST_FAILURE.captures_iter(output) {
            if let Some(test_name) = caps.get(1) {
                result.failures.push(TestFailure {
                    test_name: test_name.as_str().to_string(),
                    error_message: self.extract_failure_message(output, test_name.as_str()),
                    traceback: self.extract_traceback(output, test_name.as_str()),
                });
            }
        }

        // Extract errors
        for caps in PYTEST_ERROR.captures_iter(output) {
            if let Some(test_name) = caps.get(1) {
                result.failures.push(TestFailure {
                    test_name: test_name.as_str().to_string(),
                    error_message: self.extract_failure_message(output, test_name.as_str()),
                    traceback: self.extract_traceback(output, test_name.as_str()),
                });
            }
        }

        if result.total == 0 && !output.is_empty() {
            // If we couldn't parse anything but there's output, return what we have
            return Ok(result);
        }

        Ok(result)
    }

    /// Parse unittest output
    fn parse_unittest(&self, output: &str) -> Result<ParsedTestResult, ParseError> {
        let mut result = ParsedTestResult::default();

        // Get total tests
        if let Some(caps) = UNITTEST_SUMMARY.captures(output) {
            result.total = caps.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
        }

        // Get failures
        if let Some(caps) = UNITTEST_FAILURES.captures(output) {
            result.failed = caps.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
        }

        // Get errors
        if let Some(caps) = UNITTEST_ERRORS.captures(output) {
            result.errors = caps.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
        }

        // Get skipped
        if let Some(caps) = UNITTEST_SKIPPED.captures(output) {
            result.skipped = caps.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
        }

        // Calculate passed
        result.passed = result.total.saturating_sub(result.failed + result.errors + result.skipped);

        Ok(result)
    }

    /// Generic parsing (best effort)
    fn parse_generic(&self, output: &str) -> Result<ParsedTestResult, ParseError> {
        // Try pytest first, then unittest
        if let Ok(result) = self.parse_pytest(output) {
            if result.total > 0 {
                return Ok(result);
            }
        }

        if let Ok(result) = self.parse_unittest(output) {
            if result.total > 0 {
                return Ok(result);
            }
        }

        // Last resort: count common patterns
        let mut result = ParsedTestResult::default();
        result.passed = output.matches("PASS").count() + output.matches("pass").count();
        result.failed = output.matches("FAIL").count() + output.matches("fail").count();
        result.skipped = output.matches("SKIP").count() + output.matches("skip").count();
        result.total = result.passed + result.failed + result.skipped;

        Ok(result)
    }

    /// Extract failure message for a specific test
    fn extract_failure_message(&self, output: &str, test_name: &str) -> String {
        // Look for the failure section for this test
        let pattern = format!(
            r"(?s){}.*?(?:AssertionError|Error|Exception):\s*([^\n]+)",
            regex::escape(test_name)
        );

        if let Ok(re) = Regex::new(&pattern) {
            if let Some(caps) = re.captures(output) {
                if let Some(msg) = caps.get(1) {
                    return msg.as_str().trim().to_string();
                }
            }
        }

        "Unknown error".to_string()
    }

    /// Extract traceback for a specific test
    fn extract_traceback(&self, output: &str, test_name: &str) -> Option<String> {
        // Look for traceback section
        let start_pattern = format!("FAILED {}", test_name);
        if let Some(start) = output.find(&start_pattern) {
            let section = &output[start..];
            // Find the end of this failure section
            if let Some(end) = section.find("\n\n") {
                return Some(section[..end].to_string());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pytest_summary() {
        let output = r#"
============================= test session starts ==============================
collected 100 items

test_example.py::test_one PASSED
test_example.py::test_two FAILED

============================= 95 passed, 3 failed, 2 skipped ===================
"#;

        let parser = TestResultParser::new(TestFormat::Pytest);
        let result = parser.parse(output).unwrap();

        assert_eq!(result.passed, 95);
        assert_eq!(result.failed, 3);
        assert_eq!(result.skipped, 2);
        assert_eq!(result.total, 100);
    }

    #[test]
    fn test_parse_unittest_summary() {
        let output = r#"
..F.E..
----------------------------------------------------------------------
Ran 7 tests in 0.001s

FAILED (failures=1, errors=1)
"#;

        let parser = TestResultParser::new(TestFormat::Unittest);
        let result = parser.parse(output).unwrap();

        assert_eq!(result.total, 7);
        assert_eq!(result.failed, 1);
        assert_eq!(result.errors, 1);
        assert_eq!(result.passed, 5);
    }

    #[test]
    fn test_parse_empty_output() {
        let parser = TestResultParser::new(TestFormat::Pytest);
        let result = parser.parse("").unwrap();

        assert_eq!(result.total, 0);
        assert_eq!(result.passed, 0);
    }

    #[test]
    fn test_pass_rate() {
        let result = ParsedTestResult {
            total: 100,
            passed: 90,
            failed: 5,
            skipped: 3,
            errors: 2,
            failures: vec![],
        };

        assert!((result.pass_rate() - 0.90).abs() < 0.001);
    }
}
