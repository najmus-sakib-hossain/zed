//! DX-Py Framework Validator
//!
//! This crate provides infrastructure for validating DX-Py compatibility
//! with real-world Python frameworks by running their test suites.

pub mod benchmark;
pub mod degradation;
pub mod django;
pub mod failure;
pub mod fastapi;
pub mod flask;
pub mod matrix;
pub mod pandas;
pub mod parser;
pub mod regression;
pub mod runner;

pub use benchmark::{
    validate_benchmark_metrics, BenchmarkConfig, BenchmarkError, BenchmarkMetrics,
    BenchmarkReportGenerator, BenchmarkRunner, BenchmarkValidation, DjangoBenchmarks,
    NumpyBenchmarks, PandasBenchmarks, RealWorldBenchmark,
};
pub use degradation::{
    CompatibilityCheckResult, CompatibilityChecker, DegradationError, ExtensionFailureInfo,
    FailureReason, FeatureCompatibility, IncompatibilityInfo, IncompatibilityType, IssueSeverity,
    KnownIssue, PartialCompatibilityReport, SupportLevel, Workaround,
};
pub use django::{
    DjangoCoreTestResult, DjangoTestCategory, DjangoValidationConfig, DjangoValidationError,
    DjangoValidator,
};
pub use failure::{FailureCategorizer, FailureCategory, TestFailure};
pub use fastapi::{
    FastApiTestCategory, FastApiTestResult, FastApiValidationConfig, FastApiValidationError,
    FastApiValidator,
};
pub use flask::{
    FlaskTestCategory, FlaskTestResult, FlaskValidationConfig, FlaskValidationError, FlaskValidator,
};
pub use matrix::{CompatibilityMatrix, FrameworkResult, MatrixEntry};
pub use pandas::{
    PandasTestCategory, PandasTestResult, PandasValidationConfig, PandasValidationError,
    PandasValidator,
};
pub use parser::{ParsedTestResult, TestFormat, TestResultParser};
pub use regression::{Change, RegressionDetector, RegressionReport};
pub use runner::{TestOutput, TestRunConfig, TestRunner};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Information about a framework being validated
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkInfo {
    /// Framework name (e.g., "Django", "Flask", "NumPy")
    pub name: String,
    /// Framework version
    pub version: String,
    /// Command to run tests (e.g., "pytest", "python -m unittest")
    pub test_command: String,
    /// Working directory for test execution
    pub working_dir: Option<String>,
    /// Environment variables to set
    pub env_vars: HashMap<String, String>,
    /// Minimum acceptable pass rate (0.0 to 1.0)
    pub min_pass_rate: f64,
}

impl FrameworkInfo {
    /// Create a new framework info with default settings
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            test_command: "pytest".to_string(),
            working_dir: None,
            env_vars: HashMap::new(),
            min_pass_rate: 0.90,
        }
    }

    /// Set the test command
    pub fn with_test_command(mut self, cmd: impl Into<String>) -> Self {
        self.test_command = cmd.into();
        self
    }

    /// Set the working directory
    pub fn with_working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Set minimum pass rate
    pub fn with_min_pass_rate(mut self, rate: f64) -> Self {
        self.min_pass_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Add an environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }
}

/// Result of validating a framework
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkTestResult {
    /// Framework information
    pub framework: FrameworkInfo,
    /// Total number of tests
    pub total_tests: usize,
    /// Number of passed tests
    pub passed: usize,
    /// Number of failed tests
    pub failed: usize,
    /// Number of skipped tests
    pub skipped: usize,
    /// Number of error tests
    pub errors: usize,
    /// Failures categorized by type
    pub failure_categories: HashMap<FailureCategory, Vec<TestFailure>>,
    /// Total duration of test run
    pub duration: Duration,
    /// Timestamp when validation was run
    pub timestamp: DateTime<Utc>,
    /// Raw output from test runner
    pub raw_output: Option<String>,
}

impl FrameworkTestResult {
    /// Calculate the pass rate (0.0 to 1.0)
    pub fn pass_rate(&self) -> f64 {
        if self.total_tests == 0 {
            return 0.0;
        }
        self.passed as f64 / self.total_tests as f64
    }

    /// Check if the framework meets its minimum pass rate
    pub fn meets_minimum(&self) -> bool {
        self.pass_rate() >= self.framework.min_pass_rate
    }

    /// Get the number of actual failures (failed + errors)
    pub fn total_failures(&self) -> usize {
        self.failed + self.errors
    }
}

/// Comparison with CPython results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpythonComparison {
    /// CPython version used for comparison
    pub cpython_version: String,
    /// CPython test results
    pub cpython_result: FrameworkTestResult,
    /// Tests that pass in CPython but fail in DX-Py
    pub regressions: Vec<String>,
    /// Tests that fail in CPython but pass in DX-Py
    pub improvements: Vec<String>,
}

/// A point-in-time snapshot of compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilitySnapshot {
    /// Timestamp of the snapshot
    pub timestamp: DateTime<Utc>,
    /// DX-Py version
    pub dx_py_version: String,
    /// All framework results
    pub results: Vec<FrameworkTestResult>,
}

impl CompatibilitySnapshot {
    /// Create a new snapshot
    pub fn new(dx_py_version: impl Into<String>, results: Vec<FrameworkTestResult>) -> Self {
        Self {
            timestamp: Utc::now(),
            dx_py_version: dx_py_version.into(),
            results,
        }
    }

    /// Calculate overall pass rate across all frameworks
    pub fn overall_pass_rate(&self) -> f64 {
        let total: usize = self.results.iter().map(|r| r.total_tests).sum();
        let passed: usize = self.results.iter().map(|r| r.passed).sum();
        if total == 0 {
            return 0.0;
        }
        passed as f64 / total as f64
    }
}
