//! Benchmark runner implementation

use serde::{Deserialize, Serialize};
use std::process::{Command, Output};
use std::time::{Duration, Instant};
use thiserror::Error;

/// Minimum required measurement iterations for statistical validity
pub const MIN_MEASUREMENT_ITERATIONS: u32 = 30;

/// Python runtime type for benchmarks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PythonRuntime {
    CPython,
    DxPy,
}

/// Metadata associated with a benchmark result
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BenchmarkMetadata {
    pub runtime: Option<PythonRuntime>,
    pub description: String,
    pub tags: Vec<String>,
}

/// Validation status for benchmark output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationStatus {
    /// Output matches expected (CPython) output
    Validated,
    /// Output differs from expected output
    OutputMismatch,
    /// Benchmark failed to execute
    ExecutionFailed,
    /// Feature not supported by runtime
    NotSupported,
    /// No validation performed (baseline benchmark)
    NoValidation,
}

impl Default for ValidationStatus {
    fn default() -> Self {
        ValidationStatus::NoValidation
    }
}

/// Result of a single benchmark execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub timings: Vec<Duration>,
    pub memory_samples: Vec<usize>,
    pub metadata: BenchmarkMetadata,
    pub warmup_completed: bool,
    pub timed_out: bool,
    /// Captured stdout from the benchmark
    pub stdout: Option<String>,
    /// Captured stderr from the benchmark
    pub stderr: Option<String>,
    /// Validation status comparing output to baseline
    pub validation_status: ValidationStatus,
    /// Error message if execution failed
    pub error_message: Option<String>,
}

impl BenchmarkResult {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            timings: vec![],
            memory_samples: vec![],
            metadata: BenchmarkMetadata::default(),
            warmup_completed: false,
            timed_out: false,
            stdout: None,
            stderr: None,
            validation_status: ValidationStatus::NoValidation,
            error_message: None,
        }
    }

    /// Check if this benchmark result is valid for timing comparison
    pub fn is_valid_for_comparison(&self) -> bool {
        !self.timed_out
            && self.warmup_completed
            && !self.timings.is_empty()
            && self.validation_status != ValidationStatus::OutputMismatch
            && self.validation_status != ValidationStatus::ExecutionFailed
            && self.validation_status != ValidationStatus::NotSupported
    }

    /// Mark as not supported
    pub fn mark_not_supported(mut self, reason: &str) -> Self {
        self.validation_status = ValidationStatus::NotSupported;
        self.error_message = Some(reason.to_string());
        self
    }

    /// Mark as execution failed
    pub fn mark_execution_failed(mut self, reason: &str) -> Self {
        self.validation_status = ValidationStatus::ExecutionFailed;
        self.error_message = Some(reason.to_string());
        self
    }

    /// Mark as output mismatch
    pub fn mark_output_mismatch(mut self, expected: &str, actual: &str) -> Self {
        self.validation_status = ValidationStatus::OutputMismatch;
        self.error_message = Some(format!(
            "Output mismatch:\nExpected: {}\nActual: {}",
            expected.chars().take(200).collect::<String>(),
            actual.chars().take(200).collect::<String>()
        ));
        self
    }

    /// Mark as validated
    pub fn mark_validated(mut self) -> Self {
        self.validation_status = ValidationStatus::Validated;
        self
    }
}

/// Errors that can occur during benchmark execution
#[derive(Debug, Error)]
pub enum BenchmarkError {
    #[error(
        "Invalid iteration count: measurement iterations must be >= {MIN_MEASUREMENT_ITERATIONS}"
    )]
    InvalidIterationCount,

    #[error("Invalid timeout: timeout must be > 0")]
    InvalidTimeout,

    #[error("Benchmark timed out after {0:?}")]
    Timeout(Duration),

    #[error("External command failed: {0}")]
    CommandFailed(String),

    #[error("External tool not found: {0}")]
    ToolNotFound(String),
}

/// Configuration validation result
#[derive(Debug, Clone)]
pub struct ConfigValidation {
    pub is_valid: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// Benchmark runner that executes benchmarks with warmup and measurement phases
pub struct BenchmarkRunner {
    pub warmup_iterations: u32,
    pub measurement_iterations: u32,
    pub timeout: Duration,
}

impl BenchmarkRunner {
    /// Create a new benchmark runner with the specified configuration
    pub fn new(warmup_iterations: u32, measurement_iterations: u32, timeout: Duration) -> Self {
        Self {
            warmup_iterations,
            measurement_iterations,
            timeout,
        }
    }

    /// Validate the runner configuration
    pub fn validate(&self) -> ConfigValidation {
        let mut validation = ConfigValidation {
            is_valid: true,
            warnings: vec![],
            errors: vec![],
        };

        // Check minimum measurement iterations
        if self.measurement_iterations < MIN_MEASUREMENT_ITERATIONS {
            validation.warnings.push(format!(
                "Measurement iterations ({}) is below recommended minimum ({}). Results may not be statistically valid.",
                self.measurement_iterations, MIN_MEASUREMENT_ITERATIONS
            ));
        }

        // Check timeout
        if self.timeout.is_zero() {
            validation.is_valid = false;
            validation.errors.push("Timeout must be greater than 0".to_string());
        }

        validation
    }

    /// Run a benchmark function with warmup and measurement phases
    pub fn run_benchmark<F>(&self, name: &str, mut f: F) -> BenchmarkResult
    where
        F: FnMut(),
    {
        let mut result = BenchmarkResult::new(name);
        let start_time = Instant::now();

        // Warmup phase - execute but don't record timings
        for _ in 0..self.warmup_iterations {
            if start_time.elapsed() > self.timeout {
                result.timed_out = true;
                return result;
            }
            f();
        }
        result.warmup_completed = true;

        // Measurement phase - record timings
        for _ in 0..self.measurement_iterations {
            if start_time.elapsed() > self.timeout {
                result.timed_out = true;
                return result;
            }

            let iter_start = Instant::now();
            f();
            let elapsed = iter_start.elapsed();
            result.timings.push(elapsed);
        }

        result
    }

    /// Run an external command and measure its execution time
    pub fn run_external_command(
        &self,
        name: &str,
        cmd: &[&str],
    ) -> Result<BenchmarkResult, BenchmarkError> {
        if cmd.is_empty() {
            return Err(BenchmarkError::CommandFailed("Empty command".to_string()));
        }

        let mut result = BenchmarkResult::new(name);
        let start_time = Instant::now();

        // Warmup phase
        for _ in 0..self.warmup_iterations {
            if start_time.elapsed() > self.timeout {
                result.timed_out = true;
                return Ok(result);
            }
            self.execute_command(cmd)?;
        }
        result.warmup_completed = true;

        // Measurement phase
        for _ in 0..self.measurement_iterations {
            if start_time.elapsed() > self.timeout {
                result.timed_out = true;
                return Ok(result);
            }

            let iter_start = Instant::now();
            self.execute_command(cmd)?;
            let elapsed = iter_start.elapsed();
            result.timings.push(elapsed);
        }

        Ok(result)
    }

    /// Execute a command and return its output
    fn execute_command(&self, cmd: &[&str]) -> Result<Output, BenchmarkError> {
        let output = Command::new(cmd[0]).args(&cmd[1..]).output().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                BenchmarkError::ToolNotFound(cmd[0].to_string())
            } else {
                BenchmarkError::CommandFailed(e.to_string())
            }
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BenchmarkError::CommandFailed(format!(
                "Command failed with status {}: {}",
                output.status, stderr
            )));
        }

        Ok(output)
    }

    /// Execute a command and capture its output without checking success
    fn execute_command_with_output(&self, cmd: &[&str]) -> Result<(Output, String, String), BenchmarkError> {
        let output = Command::new(cmd[0]).args(&cmd[1..]).output().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                BenchmarkError::ToolNotFound(cmd[0].to_string())
            } else {
                BenchmarkError::CommandFailed(e.to_string())
            }
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok((output, stdout, stderr))
    }

    /// Run a Python benchmark on the specified runtime
    pub fn run_python_benchmark(
        &self,
        name: &str,
        script: &str,
        runtime: PythonRuntime,
    ) -> Result<BenchmarkResult, BenchmarkError> {
        // Try different Python commands in order of preference
        let python_commands: Vec<Vec<&str>> = match runtime {
            PythonRuntime::CPython => vec![
                vec!["uv", "run", "python", "-c", script],
                vec!["python", "-c", script],
                vec!["python3", "-c", script],
            ],
            PythonRuntime::DxPy => vec![
                // Local dx-py executable in benchmark directory
                vec!["./dx-py.exe", "-c", script],
                vec!["dx-py.exe", "-c", script],
                vec!["dx-py", "-c", script],
            ],
        };

        for cmd in &python_commands {
            match self.run_external_command(name, cmd) {
                Ok(mut result) => {
                    result.metadata.runtime = Some(runtime);
                    return Ok(result);
                }
                Err(BenchmarkError::ToolNotFound(_)) => continue,
                Err(e) => return Err(e),
            }
        }

        Err(BenchmarkError::ToolNotFound(format!(
            "No Python runtime found for {:?}",
            runtime
        )))
    }

    /// Run a Python benchmark with output validation
    /// First runs on CPython to get expected output, then runs on DX-Py and validates
    pub fn run_validated_python_benchmark(
        &self,
        name: &str,
        script: &str,
    ) -> Result<(BenchmarkResult, BenchmarkResult), BenchmarkError> {
        // First, run on CPython to get expected output
        let cpython_result = self.run_python_benchmark_with_output(
            &format!("{}_cpython", name),
            script,
            PythonRuntime::CPython,
        )?;

        // Then run on DX-Py
        let dxpy_result = self.run_python_benchmark_with_output(
            &format!("{}_dxpy", name),
            script,
            PythonRuntime::DxPy,
        );

        match dxpy_result {
            Ok(mut dxpy) => {
                // Validate output matches
                let cpython_stdout = cpython_result.stdout.as_deref().unwrap_or("");
                let dxpy_stdout = dxpy.stdout.as_deref().unwrap_or("");

                // Normalize outputs for comparison (trim whitespace, normalize line endings)
                let cpython_normalized = normalize_output(cpython_stdout);
                let dxpy_normalized = normalize_output(dxpy_stdout);

                if cpython_normalized == dxpy_normalized {
                    dxpy = dxpy.mark_validated();
                } else {
                    dxpy = dxpy.mark_output_mismatch(&cpython_normalized, &dxpy_normalized);
                }

                Ok((cpython_result, dxpy))
            }
            Err(BenchmarkError::CommandFailed(msg)) => {
                // DX-Py failed to execute - mark as not supported
                let mut failed_result = BenchmarkResult::new(format!("{}_dxpy", name));
                failed_result.metadata.runtime = Some(PythonRuntime::DxPy);
                failed_result = failed_result.mark_not_supported(&msg);
                Ok((cpython_result, failed_result))
            }
            Err(BenchmarkError::ToolNotFound(tool)) => {
                let mut failed_result = BenchmarkResult::new(format!("{}_dxpy", name));
                failed_result.metadata.runtime = Some(PythonRuntime::DxPy);
                failed_result = failed_result.mark_not_supported(&format!("Tool not found: {}", tool));
                Ok((cpython_result, failed_result))
            }
            Err(e) => Err(e),
        }
    }

    /// Run a Python benchmark and capture output
    fn run_python_benchmark_with_output(
        &self,
        name: &str,
        script: &str,
        runtime: PythonRuntime,
    ) -> Result<BenchmarkResult, BenchmarkError> {
        let python_commands: Vec<Vec<&str>> = match runtime {
            PythonRuntime::CPython => vec![
                vec!["uv", "run", "python", "-c", script],
                vec!["python", "-c", script],
                vec!["python3", "-c", script],
            ],
            PythonRuntime::DxPy => vec![
                vec!["./dx-py.exe", "-c", script],
                vec!["dx-py.exe", "-c", script],
                vec!["dx-py", "-c", script],
            ],
        };

        for cmd in &python_commands {
            match self.run_external_command_with_output(name, cmd) {
                Ok(mut result) => {
                    result.metadata.runtime = Some(runtime);
                    return Ok(result);
                }
                Err(BenchmarkError::ToolNotFound(_)) => continue,
                Err(e) => return Err(e),
            }
        }

        Err(BenchmarkError::ToolNotFound(format!(
            "No Python runtime found for {:?}",
            runtime
        )))
    }

    /// Run an external command and capture output
    fn run_external_command_with_output(
        &self,
        name: &str,
        cmd: &[&str],
    ) -> Result<BenchmarkResult, BenchmarkError> {
        if cmd.is_empty() {
            return Err(BenchmarkError::CommandFailed("Empty command".to_string()));
        }

        let mut result = BenchmarkResult::new(name);
        let start_time = Instant::now();

        // First execution to capture output
        let (output, stdout, stderr) = self.execute_command_with_output(cmd)?;
        
        if !output.status.success() {
            result.stderr = Some(stderr.clone());
            return Err(BenchmarkError::CommandFailed(format!(
                "Command failed with status {}: {}",
                output.status, stderr
            )));
        }

        result.stdout = Some(stdout);
        result.stderr = Some(stderr);

        // Warmup phase
        for _ in 0..self.warmup_iterations {
            if start_time.elapsed() > self.timeout {
                result.timed_out = true;
                return Ok(result);
            }
            self.execute_command(cmd)?;
        }
        result.warmup_completed = true;

        // Measurement phase
        for _ in 0..self.measurement_iterations {
            if start_time.elapsed() > self.timeout {
                result.timed_out = true;
                return Ok(result);
            }

            let iter_start = Instant::now();
            self.execute_command(cmd)?;
            let elapsed = iter_start.elapsed();
            result.timings.push(elapsed);
        }

        Ok(result)
    }

    /// Check if measurement iterations meet minimum requirement
    pub fn meets_minimum_iterations(&self) -> bool {
        self.measurement_iterations >= MIN_MEASUREMENT_ITERATIONS
    }
}


/// Normalize output for comparison
/// - Trims leading/trailing whitespace
/// - Normalizes line endings to \n
/// - Removes trailing whitespace from each line
fn normalize_output(output: &str) -> String {
    output
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_output() {
        assert_eq!(normalize_output("hello\nworld"), "hello\nworld");
        assert_eq!(normalize_output("hello  \nworld  "), "hello\nworld");
        assert_eq!(normalize_output("  hello\n  world  "), "hello\n  world");
        assert_eq!(normalize_output("hello\r\nworld"), "hello\nworld");
    }

    #[test]
    fn test_validation_status_default() {
        let result = BenchmarkResult::new("test");
        assert_eq!(result.validation_status, ValidationStatus::NoValidation);
    }

    #[test]
    fn test_is_valid_for_comparison() {
        let mut result = BenchmarkResult::new("test");
        result.warmup_completed = true;
        result.timings.push(Duration::from_millis(100));
        result.validation_status = ValidationStatus::Validated;
        assert!(result.is_valid_for_comparison());

        let mut failed = BenchmarkResult::new("test");
        failed.validation_status = ValidationStatus::NotSupported;
        assert!(!failed.is_valid_for_comparison());
    }

    #[test]
    fn test_mark_not_supported() {
        let result = BenchmarkResult::new("test").mark_not_supported("Feature X not implemented");
        assert_eq!(result.validation_status, ValidationStatus::NotSupported);
        assert!(result.error_message.is_some());
    }
}
