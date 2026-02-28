//! Core type definitions for dx-py-test-runner

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

use crate::assertion::AssertionFailure;

/// Unique identifier for a test case
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TestId(pub u64);

impl TestId {
    /// Create a new TestId from components
    pub fn new(file_hash: u64, line: u32, name_hash: u64) -> Self {
        // Combine hashes to create unique ID
        let combined = file_hash ^ (line as u64) ^ name_hash;
        Self(combined)
    }
}

/// Unique identifier for a fixture
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FixtureId(pub u64);

impl FixtureId {
    pub fn new(name_hash: u64) -> Self {
        Self(name_hash)
    }
}

/// A marker/decorator on a test function
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Marker {
    pub name: String,
    pub args: Vec<String>,
}

impl Marker {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            args: Vec::new(),
        }
    }

    pub fn with_args(name: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            name: name.into(),
            args,
        }
    }
}

/// A discovered test case
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestCase {
    pub id: TestId,
    pub name: String,
    pub file_path: PathBuf,
    pub line_number: u32,
    pub class_name: Option<String>,
    pub markers: Vec<Marker>,
    pub fixtures: Vec<FixtureId>,
    pub parameters: Vec<String>,
}

impl TestCase {
    pub fn new(name: impl Into<String>, file_path: impl Into<PathBuf>, line_number: u32) -> Self {
        let name = name.into();
        let file_path = file_path.into();
        let file_hash = blake3::hash(file_path.to_string_lossy().as_bytes()).as_bytes()[0..8]
            .iter()
            .fold(0u64, |acc, &b| (acc << 8) | b as u64);
        let name_hash = blake3::hash(name.as_bytes()).as_bytes()[0..8]
            .iter()
            .fold(0u64, |acc, &b| (acc << 8) | b as u64);

        Self {
            id: TestId::new(file_hash, line_number, name_hash),
            name,
            file_path,
            line_number,
            class_name: None,
            markers: Vec::new(),
            fixtures: Vec::new(),
            parameters: Vec::new(),
        }
    }

    pub fn with_class(mut self, class_name: impl Into<String>) -> Self {
        self.class_name = Some(class_name.into());
        self
    }

    pub fn with_marker(mut self, marker: Marker) -> Self {
        self.markers.push(marker);
        self
    }

    pub fn with_fixture(mut self, fixture: FixtureId) -> Self {
        self.fixtures.push(fixture);
        self
    }

    pub fn with_parameters(mut self, parameters: Vec<String>) -> Self {
        self.parameters = parameters;
        self
    }

    /// Get the fully qualified test name
    pub fn full_name(&self) -> String {
        match &self.class_name {
            Some(class) => format!("{}::{}", class, self.name),
            None => self.name.clone(),
        }
    }
}

/// Statistics about assertions in a test
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AssertionStats {
    pub passed: u32,
    pub failed: u32,
}

impl AssertionStats {
    pub fn new(passed: u32, failed: u32) -> Self {
        Self { passed, failed }
    }

    pub fn total(&self) -> u32 {
        self.passed + self.failed
    }
}

/// Status of a test execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestStatus {
    Pass,
    Fail,
    Skip { reason: String },
    Error { message: String },
}

impl TestStatus {
    pub fn is_success(&self) -> bool {
        matches!(self, TestStatus::Pass | TestStatus::Skip { .. })
    }

    pub fn is_failure(&self) -> bool {
        matches!(self, TestStatus::Fail | TestStatus::Error { .. })
    }
}

/// Result of executing a test
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestResult {
    pub test_id: TestId,
    pub status: TestStatus,
    pub duration: Duration,
    pub stdout: String,
    pub stderr: String,
    pub traceback: Option<String>,
    pub assertions: AssertionStats,
    /// Detailed assertion failure information with introspection
    pub assertion_failure: Option<AssertionFailure>,
}

impl TestResult {
    pub fn pass(test_id: TestId, duration: Duration) -> Self {
        Self {
            test_id,
            status: TestStatus::Pass,
            duration,
            stdout: String::new(),
            stderr: String::new(),
            traceback: None,
            assertions: AssertionStats::default(),
            assertion_failure: None,
        }
    }

    pub fn fail(test_id: TestId, duration: Duration, traceback: impl Into<String>) -> Self {
        Self {
            test_id,
            status: TestStatus::Fail,
            duration,
            stdout: String::new(),
            stderr: String::new(),
            traceback: Some(traceback.into()),
            assertions: AssertionStats::default(),
            assertion_failure: None,
        }
    }

    pub fn skip(test_id: TestId, reason: impl Into<String>) -> Self {
        Self {
            test_id,
            status: TestStatus::Skip {
                reason: reason.into(),
            },
            duration: Duration::ZERO,
            stdout: String::new(),
            stderr: String::new(),
            traceback: None,
            assertions: AssertionStats::default(),
            assertion_failure: None,
        }
    }

    pub fn error(test_id: TestId, message: impl Into<String>) -> Self {
        Self {
            test_id,
            status: TestStatus::Error {
                message: message.into(),
            },
            duration: Duration::ZERO,
            stdout: String::new(),
            stderr: String::new(),
            traceback: None,
            assertions: AssertionStats::default(),
            assertion_failure: None,
        }
    }

    pub fn with_output(mut self, stdout: String, stderr: String) -> Self {
        self.stdout = stdout;
        self.stderr = stderr;
        self
    }

    pub fn with_assertions(mut self, assertions: AssertionStats) -> Self {
        self.assertions = assertions;
        self
    }

    /// Add detailed assertion failure information
    pub fn with_assertion_failure(mut self, failure: AssertionFailure) -> Self {
        self.assertion_failure = Some(failure);
        self
    }
}

/// Summary of a test run
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TestSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub errors: usize,
    pub duration: Duration,
}

impl TestSummary {
    pub fn from_results(results: &[TestResult]) -> Self {
        let mut summary = Self {
            total: results.len(),
            ..Default::default()
        };

        for result in results {
            summary.duration += result.duration;
            match &result.status {
                TestStatus::Pass => summary.passed += 1,
                TestStatus::Fail => summary.failed += 1,
                TestStatus::Skip { .. } => summary.skipped += 1,
                TestStatus::Error { .. } => summary.errors += 1,
            }
        }

        summary
    }

    pub fn is_success(&self) -> bool {
        self.failed == 0 && self.errors == 0
    }
}
