//! Benchmark framework orchestration

use super::{BenchmarkConfig, BenchmarkRunner, OutputFormat, PythonRuntime};
use crate::analysis::StatisticalAnalyzer;
use crate::data::{BenchmarkResults, ResultStore, StoredResult};
use crate::report::{ComparisonReport, ReportGenerator};
use crate::suites::{PackageSuite, RuntimeSuite, TestRunnerSuite};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

/// Errors that can occur during framework operations
#[derive(Debug, Error)]
pub enum FrameworkError {
    #[error("Unknown suite: {0}")]
    UnknownSuite(String),

    #[error("Benchmark execution failed: {0}")]
    ExecutionFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Store error: {0}")]
    Store(String),

    #[error("Result not found: {0}")]
    ResultNotFound(String),
}

/// Result of running a benchmark suite
#[derive(Debug, Clone)]
pub struct SuiteRunResult {
    pub suite_name: String,
    pub results: BenchmarkResults,
    pub comparison: Option<ComparisonReport>,
    pub markdown_report: String,
    pub json_report: String,
    pub stored_id: Option<String>,
}

/// Central orchestrator for benchmark execution
pub struct BenchmarkFramework {
    pub config: BenchmarkConfig,
    pub runner: BenchmarkRunner,
    pub analyzer: StatisticalAnalyzer,
    pub reporter: ReportGenerator,
    pub result_store: ResultStore,
}

impl BenchmarkFramework {
    /// Create a new benchmark framework with the specified configuration
    pub fn new(config: BenchmarkConfig) -> Self {
        let runner = BenchmarkRunner::new(
            config.warmup_iterations,
            config.measurement_iterations,
            Duration::from_secs(config.timeout_seconds),
        );

        Self {
            config: config.clone(),
            runner,
            analyzer: StatisticalAnalyzer::new(),
            reporter: ReportGenerator::new(config.output_dir.clone()),
            result_store: ResultStore::new(config.output_dir.clone()),
        }
    }

    /// Get list of available benchmark suites
    pub fn available_suites() -> Vec<&'static str> {
        vec!["runtime", "package", "test_runner"]
    }

    /// Run a specific benchmark suite by name
    pub fn run_suite(&mut self, suite_name: &str) -> Result<SuiteRunResult, FrameworkError> {
        let seed = self.config.seed.unwrap_or(42);

        let mut results = BenchmarkResults::new(suite_name, self.config.clone());

        match suite_name {
            "runtime" => {
                let mut suite = RuntimeSuite::new(seed);
                self.run_runtime_suite(&mut suite, &mut results)?;
            }
            "package" => {
                let suite = PackageSuite::new(seed);
                self.run_package_suite(&suite, &mut results)?;
            }
            "test_runner" => {
                let mut suite = TestRunnerSuite::new(seed);
                self.run_test_runner_suite(&mut suite, &mut results)?;
            }
            _ => return Err(FrameworkError::UnknownSuite(suite_name.to_string())),
        }

        // Generate comparison report if we have baseline and subject results
        let comparison = self.create_comparison_if_available(&results);

        // Generate reports in both formats
        let markdown_report = self.generate_markdown_report(&results, &comparison);
        let json_report = self.generate_json_report(&results);

        // Store results
        let stored_id = self.store_results(&results)?;

        // Write output files based on configuration
        self.write_output_files(suite_name, &markdown_report, &json_report)?;

        Ok(SuiteRunResult {
            suite_name: suite_name.to_string(),
            results,
            comparison,
            markdown_report,
            json_report,
            stored_id: Some(stored_id),
        })
    }

    /// Run all configured benchmark suites
    pub fn run_all(&mut self) -> Result<Vec<SuiteRunResult>, FrameworkError> {
        let suites = if self.config.suites.is_empty() {
            Self::available_suites().iter().map(|s| s.to_string()).collect()
        } else {
            self.config.suites.clone()
        };

        let mut results = Vec::new();
        for suite_name in suites {
            let result = self.run_suite(&suite_name)?;
            results.push(result);
        }

        Ok(results)
    }

    /// Reproduce a previous benchmark run using stored configuration
    pub fn reproduce(&mut self, run_id: &str) -> Result<SuiteRunResult, FrameworkError> {
        // Load the previous result
        let stored = self
            .result_store
            .load(run_id)
            .map_err(|e| FrameworkError::Store(e.to_string()))?;

        // Update our config to match the stored config
        self.config = stored.config.clone();
        self.runner = BenchmarkRunner::new(
            self.config.warmup_iterations,
            self.config.measurement_iterations,
            Duration::from_secs(self.config.timeout_seconds),
        );

        // Re-run the suite
        self.run_suite(&stored.results.suite)
    }

    /// Compare current results with a previous run
    pub fn compare_with_previous(
        &self,
        current: &BenchmarkResults,
        previous_id: &str,
    ) -> Result<ComparisonReport, FrameworkError> {
        let previous = self
            .result_store
            .load(previous_id)
            .map_err(|e| FrameworkError::Store(e.to_string()))?;

        Ok(self.reporter.create_comparison(&previous.results, current))
    }

    /// Get historical results for a suite
    pub fn get_historical(&self, suite: &str, count: usize) -> Vec<StoredResult> {
        self.result_store.get_historical(suite, count)
    }

    // Private helper methods

    fn run_runtime_suite(
        &self,
        suite: &mut RuntimeSuite,
        results: &mut BenchmarkResults,
    ) -> Result<(), FrameworkError> {
        // Run micro-benchmarks
        for spec in suite.micro_benchmarks() {
            if self.should_run_benchmark(&spec.name) {
                // Run on CPython
                if let Ok(result) = self.runner.run_python_benchmark(
                    &format!("{}_cpython", spec.name),
                    &spec.cpython_code,
                    PythonRuntime::CPython,
                ) {
                    results.add_result(result);
                }

                // Run on DX-Py
                if let Ok(result) = self.runner.run_python_benchmark(
                    &format!("{}_dxpy", spec.name),
                    &spec.dxpy_code,
                    PythonRuntime::DxPy,
                ) {
                    results.add_result(result);
                }
            }
        }

        // Run macro-benchmarks
        for spec in suite.macro_benchmarks() {
            if self.should_run_benchmark(&spec.name) {
                if let Ok(result) = self.runner.run_python_benchmark(
                    &format!("{}_cpython", spec.name),
                    &spec.cpython_code,
                    PythonRuntime::CPython,
                ) {
                    results.add_result(result);
                }

                if let Ok(result) = self.runner.run_python_benchmark(
                    &format!("{}_dxpy", spec.name),
                    &spec.dxpy_code,
                    PythonRuntime::DxPy,
                ) {
                    results.add_result(result);
                }
            }
        }

        // Run startup/memory benchmarks
        for spec in suite.startup_memory_benchmarks() {
            if self.should_run_benchmark(&spec.name) {
                if let Ok(result) = self.runner.run_python_benchmark(
                    &format!("{}_cpython", spec.name),
                    &spec.cpython_code,
                    PythonRuntime::CPython,
                ) {
                    results.add_result(result);
                }

                if let Ok(result) = self.runner.run_python_benchmark(
                    &format!("{}_dxpy", spec.name),
                    &spec.dxpy_code,
                    PythonRuntime::DxPy,
                ) {
                    results.add_result(result);
                }
            }
        }

        Ok(())
    }

    fn run_package_suite(
        &self,
        suite: &PackageSuite,
        results: &mut BenchmarkResults,
    ) -> Result<(), FrameworkError> {
        // Run all package manager benchmarks
        for spec in suite.all_benchmarks() {
            if self.should_run_benchmark(&spec.name) {
                // Run with UV
                if let Ok(result) = self.runner.run_python_benchmark(
                    &format!("{}_uv", spec.name),
                    &spec.cpython_code,
                    PythonRuntime::CPython,
                ) {
                    results.add_result(result);
                }

                // Run with DX-Py package manager
                if let Ok(result) = self.runner.run_python_benchmark(
                    &format!("{}_dxpy", spec.name),
                    &spec.dxpy_code,
                    PythonRuntime::DxPy,
                ) {
                    results.add_result(result);
                }
            }
        }

        // Run real-world benchmarks
        for spec in suite.real_world_benchmarks() {
            if self.should_run_benchmark(&spec.name) {
                if let Ok(result) = self.runner.run_python_benchmark(
                    &format!("{}_uv", spec.name),
                    &spec.cpython_code,
                    PythonRuntime::CPython,
                ) {
                    results.add_result(result);
                }

                if let Ok(result) = self.runner.run_python_benchmark(
                    &format!("{}_dxpy", spec.name),
                    &spec.dxpy_code,
                    PythonRuntime::DxPy,
                ) {
                    results.add_result(result);
                }
            }
        }

        Ok(())
    }

    fn run_test_runner_suite(
        &self,
        suite: &mut TestRunnerSuite,
        results: &mut BenchmarkResults,
    ) -> Result<(), FrameworkError> {
        // Run discovery benchmarks
        for spec in suite.discovery_benchmarks() {
            if self.should_run_benchmark(&spec.name) {
                // Run with pytest
                if let Ok(result) = self.runner.run_python_benchmark(
                    &format!("{}_pytest", spec.name),
                    &spec.cpython_code,
                    PythonRuntime::CPython,
                ) {
                    results.add_result(result);
                }

                // Run with DX-Py test runner
                if let Ok(result) = self.runner.run_python_benchmark(
                    &format!("{}_dxpy", spec.name),
                    &spec.dxpy_code,
                    PythonRuntime::DxPy,
                ) {
                    results.add_result(result);
                }
            }
        }

        // Run execution benchmarks
        for spec in suite.execution_benchmarks() {
            if self.should_run_benchmark(&spec.name) {
                if let Ok(result) = self.runner.run_python_benchmark(
                    &format!("{}_pytest", spec.name),
                    &spec.cpython_code,
                    PythonRuntime::CPython,
                ) {
                    results.add_result(result);
                }

                if let Ok(result) = self.runner.run_python_benchmark(
                    &format!("{}_dxpy", spec.name),
                    &spec.dxpy_code,
                    PythonRuntime::DxPy,
                ) {
                    results.add_result(result);
                }
            }
        }

        Ok(())
    }

    fn should_run_benchmark(&self, name: &str) -> bool {
        match &self.config.filter {
            Some(filter) => name.contains(filter),
            None => true,
        }
    }

    fn create_comparison_if_available(
        &self,
        results: &BenchmarkResults,
    ) -> Option<ComparisonReport> {
        // Try to find matching baseline/subject pairs
        let mut baseline_results = BenchmarkResults::new(&results.suite, self.config.clone());
        let mut subject_results = BenchmarkResults::new(&results.suite, self.config.clone());

        for bench in &results.benchmarks {
            if bench.name.ends_with("_cpython")
                || bench.name.ends_with("_uv")
                || bench.name.ends_with("_pytest")
            {
                baseline_results.add_result(bench.clone());
            } else if bench.name.ends_with("_dxpy") {
                subject_results.add_result(bench.clone());
            }
        }

        if !baseline_results.benchmarks.is_empty() && !subject_results.benchmarks.is_empty() {
            // Normalize names for comparison
            let mut normalized_baseline = baseline_results.clone();
            let mut normalized_subject = subject_results.clone();

            for bench in &mut normalized_baseline.benchmarks {
                bench.name = bench
                    .name
                    .trim_end_matches("_cpython")
                    .trim_end_matches("_uv")
                    .trim_end_matches("_pytest")
                    .to_string();
            }

            for bench in &mut normalized_subject.benchmarks {
                bench.name = bench.name.trim_end_matches("_dxpy").to_string();
            }

            Some(self.reporter.create_comparison(&normalized_baseline, &normalized_subject))
        } else {
            None
        }
    }

    fn generate_markdown_report(
        &self,
        results: &BenchmarkResults,
        comparison: &Option<ComparisonReport>,
    ) -> String {
        match comparison {
            Some(comp) => self.reporter.generate_markdown(results, comp),
            None => {
                // Generate a basic report without comparison
                let mut md = String::new();
                md.push_str(&format!("# Benchmark Report: {}\n\n", results.suite));
                md.push_str(&format!(
                    "Generated: {}\n\n",
                    results.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
                ));
                md.push_str("## Results\n\n");
                md.push_str("| Benchmark | Mean (ms) | Std Dev (ms) |\n");
                md.push_str("|-----------|-----------|-------------|\n");

                for bench in &results.benchmarks {
                    let stats = self.analyzer.compute_statistics(&bench.timings);
                    md.push_str(&format!(
                        "| {} | {:.3} | {:.3} |\n",
                        bench.name,
                        stats.mean * 1000.0,
                        stats.std_dev * 1000.0
                    ));
                }

                md
            }
        }
    }

    fn generate_json_report(&self, results: &BenchmarkResults) -> String {
        self.reporter.generate_json(results)
    }

    fn store_results(&self, results: &BenchmarkResults) -> Result<String, FrameworkError> {
        self.result_store
            .save(results, &self.config)
            .map_err(|e| FrameworkError::Store(e.to_string()))
    }

    fn write_output_files(
        &self,
        suite_name: &str,
        markdown: &str,
        json: &str,
    ) -> Result<(), FrameworkError> {
        // Ensure output directory exists
        if !self.config.output_dir.exists() {
            fs::create_dir_all(&self.config.output_dir)?;
        }

        match self.config.output_format {
            OutputFormat::Markdown => {
                self.write_markdown_file(suite_name, markdown)?;
            }
            OutputFormat::Json => {
                self.write_json_file(suite_name, json)?;
            }
            OutputFormat::Both => {
                self.write_markdown_file(suite_name, markdown)?;
                self.write_json_file(suite_name, json)?;
            }
        }

        Ok(())
    }

    fn write_markdown_file(&self, suite_name: &str, content: &str) -> Result<(), FrameworkError> {
        let path = self.output_path(suite_name, "md");
        fs::write(&path, content)?;
        Ok(())
    }

    fn write_json_file(&self, suite_name: &str, content: &str) -> Result<(), FrameworkError> {
        let path = self.output_path(suite_name, "json");
        fs::write(&path, content)?;
        Ok(())
    }

    fn output_path(&self, suite_name: &str, extension: &str) -> PathBuf {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        self.config
            .output_dir
            .join(format!("{}_{}.{}", suite_name, timestamp, extension))
    }

    /// Validate that both markdown and JSON outputs are valid
    pub fn validate_dual_output(markdown: &str, json: &str) -> DualOutputValidation {
        let markdown_valid = !markdown.is_empty() && markdown.contains('#');
        let json_valid = serde_json::from_str::<serde_json::Value>(json).is_ok();

        DualOutputValidation {
            markdown_valid,
            json_valid,
            both_valid: markdown_valid && json_valid,
        }
    }
}

/// Validation result for dual output format
#[derive(Debug, Clone)]
pub struct DualOutputValidation {
    pub markdown_valid: bool,
    pub json_valid: bool,
    pub both_valid: bool,
}
