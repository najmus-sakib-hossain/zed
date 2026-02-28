//! Test runner implementation

use anyhow::Result;
use std::time::Instant;

use super::{TestResult, TestStatus, TestSuite, TestSummary};

/// Run configuration
#[derive(Debug, Clone)]
pub struct RunConfig {
    pub parallel: bool,
    pub jobs: Option<usize>,
    pub fail_fast: bool,
    pub filter: Option<String>,
    pub verbose: bool,
}

/// Run test suites
pub fn run_suites(suites: &[TestSuite], config: &RunConfig) -> Result<TestSummary> {
    let start = Instant::now();
    let mut summary = TestSummary::default();

    for suite in suites {
        // Filter tests if pattern provided
        let tests: Vec<_> = if let Some(ref filter) = config.filter {
            suite
                .tests
                .iter()
                .filter(|t| t.name.contains(filter) || t.full_name.contains(filter))
                .collect()
        } else {
            suite.tests.iter().collect()
        };

        if tests.is_empty() {
            continue;
        }

        if config.parallel {
            // Parallel execution (rayon feature would be used here)
            let results: Vec<TestResult> =
                tests.iter().map(|test| run_test(&suite.name, &test.name)).collect();

            for result in results {
                update_summary(&mut summary, &result, config)?;
                summary.results.push(result);
            }
        } else {
            // Sequential execution
            for test in tests {
                let result = run_test(&suite.name, &test.name);

                if config.fail_fast && result.status == TestStatus::Failed {
                    summary.results.push(result);
                    summary.failed += 1;
                    summary.total += 1;
                    anyhow::bail!("Test failed (fail-fast mode)");
                }

                update_summary(&mut summary, &result, config)?;
                summary.results.push(result);
            }
        }
    }

    summary.duration = start.elapsed();
    Ok(summary)
}

fn update_summary(
    summary: &mut TestSummary,
    result: &TestResult,
    config: &RunConfig,
) -> Result<()> {
    summary.total += 1;

    match result.status {
        TestStatus::Passed => {
            summary.passed += 1;
            if config.verbose {
                println!("✓ {}", result.name);
            }
        }
        TestStatus::Failed => {
            summary.failed += 1;
            println!("✗ {}", result.name);
            if let Some(ref msg) = result.message {
                println!("  {}", msg);
            }
        }
        TestStatus::Skipped => {
            summary.skipped += 1;
            if config.verbose {
                println!("○ {} (skipped)", result.name);
            }
        }
        TestStatus::TimedOut => {
            summary.failed += 1;
            println!("⏱ {} (timed out)", result.name);
        }
    }

    Ok(())
}

fn run_test(suite: &str, test: &str) -> TestResult {
    let start = Instant::now();
    let full_name = format!("{}::{}", suite, test);

    // TODO: Actually run the test
    // For now, simulate a passing test

    TestResult {
        name: full_name,
        status: TestStatus::Passed,
        duration: start.elapsed(),
        message: None,
        stack_trace: None,
        output: None,
    }
}

/// Test runner for Rust tests
pub struct RustTestRunner;

impl RustTestRunner {
    pub fn run(filter: Option<&str>) -> Result<TestSummary> {
        use std::process::Command;

        let mut cmd = Command::new("cargo");
        cmd.arg("test");

        if let Some(f) = filter {
            cmd.arg(f);
        }

        cmd.arg("--").arg("--format=json");

        let output = cmd.output()?;

        // Parse JSON output
        let summary = parse_cargo_test_output(&output.stdout)?;

        Ok(summary)
    }
}

fn parse_cargo_test_output(output: &[u8]) -> Result<TestSummary> {
    // Parse cargo test JSON output
    // Each line is a JSON event
    let mut summary = TestSummary::default();

    for line in output.split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }

        // Parse JSON event
        // TODO: Full JSON parsing
        let line_str = String::from_utf8_lossy(line);

        if line_str.contains("\"event\":\"ok\"") {
            summary.passed += 1;
            summary.total += 1;
        } else if line_str.contains("\"event\":\"failed\"") {
            summary.failed += 1;
            summary.total += 1;
        } else if line_str.contains("\"event\":\"ignored\"") {
            summary.skipped += 1;
            summary.total += 1;
        }
    }

    Ok(summary)
}

/// Test runner for JavaScript tests
pub struct JsTestRunner;

impl JsTestRunner {
    pub fn run(path: &std::path::Path, filter: Option<&str>) -> Result<TestSummary> {
        use std::process::Command;

        // Try different test runners
        let runners = ["vitest", "jest", "mocha", "node --test"];

        for runner in &runners {
            let parts: Vec<&str> = runner.split_whitespace().collect();
            let program = parts[0];

            if Command::new(program).arg("--version").output().is_ok() {
                return Self::run_with(runner, path, filter);
            }
        }

        anyhow::bail!("No JavaScript test runner found")
    }

    fn run_with(runner: &str, path: &std::path::Path, filter: Option<&str>) -> Result<TestSummary> {
        use std::process::Command;

        let parts: Vec<&str> = runner.split_whitespace().collect();
        let mut cmd = Command::new(parts[0]);

        for arg in &parts[1..] {
            cmd.arg(arg);
        }

        cmd.current_dir(path);

        if let Some(f) = filter {
            cmd.arg("--grep").arg(f);
        }

        let _output = cmd.output()?;

        // TODO: Parse output
        Ok(TestSummary::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_config_default() {
        let config = RunConfig {
            parallel: true,
            jobs: None,
            fail_fast: false,
            filter: None,
            verbose: false,
        };

        assert!(config.parallel);
        assert!(!config.fail_fast);
    }
}
