//! Test runner for executing Python test suites via subprocess

use crate::failure::FailureCategorizer;
use crate::parser::{TestFormat, TestResultParser};
use crate::{FailureCategory, FrameworkInfo, FrameworkTestResult, TestFailure};
use chrono::Utc;
use std::collections::HashMap;
use std::process::Stdio;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Errors that can occur during test execution
#[derive(Debug, Error)]
pub enum RunnerError {
    #[error("Failed to spawn test process: {0}")]
    SpawnError(#[from] std::io::Error),

    #[error("Test execution timed out after {0:?}")]
    Timeout(Duration),

    #[error("Failed to parse test output: {0}")]
    ParseError(String),

    #[error("Python interpreter not found: {0}")]
    InterpreterNotFound(String),
}

/// Configuration for test execution
#[derive(Debug, Clone)]
pub struct TestRunConfig {
    /// Python interpreter to use (e.g., "dx-py", "python3")
    pub interpreter: String,
    /// Maximum time to wait for tests to complete
    pub timeout: Duration,
    /// Whether to capture stdout/stderr
    pub capture_output: bool,
    /// Test output format to parse
    pub output_format: TestFormat,
    /// Additional arguments to pass to test runner
    pub extra_args: Vec<String>,
}

impl Default for TestRunConfig {
    fn default() -> Self {
        Self {
            interpreter: "dx-py".to_string(),
            timeout: Duration::from_secs(3600), // 1 hour default
            capture_output: true,
            output_format: TestFormat::Pytest,
            extra_args: Vec::new(),
        }
    }
}

impl TestRunConfig {
    /// Create config for DX-Py interpreter
    pub fn dx_py() -> Self {
        Self::default()
    }

    /// Create config for CPython interpreter
    pub fn cpython() -> Self {
        Self {
            interpreter: "python3".to_string(),
            ..Default::default()
        }
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set output format
    pub fn with_format(mut self, format: TestFormat) -> Self {
        self.output_format = format;
        self
    }

    /// Add extra arguments
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.extra_args = args;
        self
    }
}

/// Output from a test run
#[derive(Debug, Clone)]
pub struct TestOutput {
    /// Combined stdout and stderr
    pub output: String,
    /// Exit code
    pub exit_code: Option<i32>,
    /// Duration of the test run
    pub duration: Duration,
}

/// Test runner that executes Python test suites
pub struct TestRunner {
    config: TestRunConfig,
    parser: TestResultParser,
    categorizer: FailureCategorizer,
}

impl TestRunner {
    /// Create a new test runner with the given configuration
    pub fn new(config: TestRunConfig) -> Self {
        let parser = TestResultParser::new(config.output_format.clone());
        Self {
            config,
            parser,
            categorizer: FailureCategorizer::new(),
        }
    }

    /// Run tests for a framework and return results
    pub async fn run(&self, framework: &FrameworkInfo) -> Result<FrameworkTestResult, RunnerError> {
        let output = self.execute_tests(framework).await?;
        self.parse_results(framework, output)
    }

    /// Execute the test command and capture output
    async fn execute_tests(&self, framework: &FrameworkInfo) -> Result<TestOutput, RunnerError> {
        let start = Instant::now();

        // Build the command
        let mut cmd = Command::new(&self.config.interpreter);

        // Parse the test command into parts
        let parts: Vec<&str> = framework.test_command.split_whitespace().collect();
        if !parts.is_empty() {
            cmd.arg("-m");
            cmd.args(&parts);
        }

        // Add extra arguments
        cmd.args(&self.config.extra_args);

        // Set working directory if specified
        if let Some(ref dir) = framework.working_dir {
            cmd.current_dir(dir);
        }

        // Set environment variables
        for (key, value) in &framework.env_vars {
            cmd.env(key, value);
        }

        // Configure output capture
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Spawn the process
        let mut child = cmd.spawn()?;

        // Capture output with timeout
        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                return Err(RunnerError::SpawnError(std::io::Error::other(
                    "Failed to capture stdout - pipe not available",
                )));
            }
        };
        let stderr = match child.stderr.take() {
            Some(stderr) => stderr,
            None => {
                return Err(RunnerError::SpawnError(std::io::Error::other(
                    "Failed to capture stderr - pipe not available",
                )));
            }
        };

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let mut output = String::new();

        // Read output with timeout
        let timeout_result = tokio::time::timeout(self.config.timeout, async {
            loop {
                tokio::select! {
                    line = stdout_reader.next_line() => {
                        match line {
                            Ok(Some(l)) => {
                                output.push_str(&l);
                                output.push('\n');
                            }
                            Ok(None) => break,
                            Err(e) => return Err(RunnerError::SpawnError(e)),
                        }
                    }
                    line = stderr_reader.next_line() => {
                        match line {
                            Ok(Some(l)) => {
                                output.push_str(&l);
                                output.push('\n');
                            }
                            Ok(None) => {}
                            Err(e) => return Err(RunnerError::SpawnError(e)),
                        }
                    }
                }
            }
            Ok(())
        })
        .await;

        match timeout_result {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                // Kill the process on timeout
                let _ = child.kill().await;
                return Err(RunnerError::Timeout(self.config.timeout));
            }
        }

        let status = child.wait().await?;
        let duration = start.elapsed();

        Ok(TestOutput {
            output,
            exit_code: status.code(),
            duration,
        })
    }

    /// Parse test output into structured results
    fn parse_results(
        &self,
        framework: &FrameworkInfo,
        output: TestOutput,
    ) -> Result<FrameworkTestResult, RunnerError> {
        let parsed = self
            .parser
            .parse(&output.output)
            .map_err(|e| RunnerError::ParseError(e.to_string()))?;

        // Categorize failures
        let mut failure_categories: HashMap<FailureCategory, Vec<TestFailure>> = HashMap::new();

        for failure in &parsed.failures {
            let category = self.categorizer.categorize(failure);
            failure_categories.entry(category).or_default().push(failure.clone());
        }

        Ok(FrameworkTestResult {
            framework: framework.clone(),
            total_tests: parsed.total,
            passed: parsed.passed,
            failed: parsed.failed,
            skipped: parsed.skipped,
            errors: parsed.errors,
            failure_categories,
            duration: output.duration,
            timestamp: Utc::now(),
            raw_output: if self.config.capture_output {
                Some(output.output)
            } else {
                None
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = TestRunConfig::default();
        assert_eq!(config.interpreter, "dx-py");
        assert!(config.capture_output);
    }

    #[test]
    fn test_config_cpython() {
        let config = TestRunConfig::cpython();
        assert_eq!(config.interpreter, "python3");
    }

    #[test]
    fn test_framework_info_builder() {
        let info = FrameworkInfo::new("Django", "4.2")
            .with_test_command("pytest django/tests")
            .with_min_pass_rate(0.90)
            .with_env("DJANGO_SETTINGS_MODULE", "tests.settings");

        assert_eq!(info.name, "Django");
        assert_eq!(info.version, "4.2");
        assert_eq!(info.min_pass_rate, 0.90);
        assert!(info.env_vars.contains_key("DJANGO_SETTINGS_MODULE"));
    }
}
