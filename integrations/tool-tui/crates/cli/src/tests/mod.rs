//! Test module for DX CLI
//!
//! This module provides comprehensive test suites for:
//! - End-to-end tests (macOS, iOS, Android, Gateway)
//! - Integration tests (TTS, Voice Wake, Productivity, Automation, Media)
//! - Unit tests (individual module tests)

pub mod e2e;
pub mod integration;

pub use e2e::{
    AndroidTestConfig, AndroidTestSuite, GatewayTestConfig, GatewayTestSuite, IOSTestConfig,
    IOSTestSuite, MacOSTestConfig, MacOSTestSuite, Platform, run_all_e2e_tests,
};
pub use integration::{IntegrationTestConfig, IntegrationTestSuite, TestCategory};

use std::time::Duration;

/// Unified test configuration
#[derive(Debug, Clone)]
pub struct TestConfig {
    /// Run E2E tests
    pub run_e2e: bool,
    /// Run integration tests
    pub run_integration: bool,
    /// Test timeout
    pub timeout: Duration,
    /// Verbose output
    pub verbose: bool,
    /// Filter by platform
    pub platforms: Option<Vec<Platform>>,
    /// Filter by category
    pub categories: Option<Vec<TestCategory>>,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            run_e2e: true,
            run_integration: true,
            timeout: Duration::from_secs(60),
            verbose: false,
            platforms: None,
            categories: None,
        }
    }
}

/// Unified test result
#[derive(Debug, Clone)]
pub struct UnifiedTestResult {
    pub name: String,
    pub test_type: TestType,
    pub passed: bool,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestType {
    E2E,
    Integration,
    Unit,
}

impl std::fmt::Display for TestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestType::E2E => write!(f, "E2E"),
            TestType::Integration => write!(f, "Integration"),
            TestType::Unit => write!(f, "Unit"),
        }
    }
}

/// Run all tests with unified configuration
pub async fn run_all_tests(config: TestConfig) -> Vec<UnifiedTestResult> {
    let mut all_results = Vec::new();

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║            DX Agent Complete Test Suite                   ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    if config.run_e2e {
        println!("▶ Running E2E Tests...\n");
        let e2e_results = run_all_e2e_tests().await;
        all_results.extend(e2e_results.into_iter().map(|r| UnifiedTestResult {
            name: r.name,
            test_type: TestType::E2E,
            passed: r.passed,
            duration_ms: r.duration_ms,
            error: r.error,
        }));
    }

    if config.run_integration {
        println!("\n▶ Running Integration Tests...\n");
        let int_config = IntegrationTestConfig::default();
        let mut int_suite = IntegrationTestSuite::new(int_config);
        let int_results = int_suite.run_all().await;
        all_results.extend(int_results.into_iter().map(|r| UnifiedTestResult {
            name: r.name,
            test_type: TestType::Integration,
            passed: r.passed,
            duration_ms: r.duration_ms,
            error: r.error,
        }));
    }

    // Print final summary
    print_final_summary(&all_results);

    all_results
}

fn print_final_summary(results: &[UnifiedTestResult]) {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║                    Final Summary                          ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    // Group by test type
    let e2e_results: Vec<_> = results.iter().filter(|r| r.test_type == TestType::E2E).collect();
    let int_results: Vec<_> =
        results.iter().filter(|r| r.test_type == TestType::Integration).collect();

    if !e2e_results.is_empty() {
        let passed = e2e_results.iter().filter(|r| r.passed).count();
        let total = e2e_results.len();
        let time: u64 = e2e_results.iter().map(|r| r.duration_ms).sum();
        let status = if passed == total { "✅" } else { "⚠️" };
        println!("  {} E2E Tests: {}/{} passed ({} ms)", status, passed, total, time);
    }

    if !int_results.is_empty() {
        let passed = int_results.iter().filter(|r| r.passed).count();
        let total = int_results.len();
        let time: u64 = int_results.iter().map(|r| r.duration_ms).sum();
        let status = if passed == total { "✅" } else { "⚠️" };
        println!("  {} Integration Tests: {}/{} passed ({} ms)", status, passed, total, time);
    }

    let total_passed = results.iter().filter(|r| r.passed).count();
    let total_tests = results.len();
    let total_time: u64 = results.iter().map(|r| r.duration_ms).sum();

    println!("\n════════════════════════════════════════════════════════════");

    if total_passed == total_tests {
        println!("🎉 ALL {} TESTS PASSED in {} ms", total_tests, total_time);
    } else {
        println!("⚠️  {}/{} TESTS PASSED in {} ms", total_passed, total_tests, total_time);

        let failures: Vec<_> = results.iter().filter(|r| !r.passed).collect();
        if !failures.is_empty() {
            println!("\nFailed tests:");
            for result in failures {
                println!("  ❌ [{}] {}: {:?}", result.test_type, result.name, result.error);
            }
        }
    }

    println!("════════════════════════════════════════════════════════════\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_all() {
        let config = TestConfig::default();
        let results = run_all_tests(config).await;

        // All tests should pass
        assert!(results.iter().all(|r| r.passed));
    }

    #[test]
    fn test_config_default() {
        let config = TestConfig::default();
        assert!(config.run_e2e);
        assert!(config.run_integration);
    }
}
