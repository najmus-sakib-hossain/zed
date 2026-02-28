//! E2E test module for cross-platform testing
//!
//! This module provides end-to-end test suites for:
//! - macOS menu bar app
//! - iOS app
//! - Android app
//! - Gateway integration

pub mod android;
pub mod gateway;
pub mod ios;
pub mod macos;

pub use android::{AndroidTestConfig, AndroidTestSuite};
pub use gateway::{GatewayTestConfig, GatewayTestSuite};
pub use ios::{IOSTestConfig, IOSTestSuite};
pub use macos::{MacOSTestConfig, MacOSTestSuite};

/// Unified test result type
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub platform: Platform,
    pub passed: bool,
    pub duration_ms: u64,
    pub error: Option<String>,
}

/// Target platform for E2E tests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    MacOS,
    IOS,
    Android,
    Windows,
    Linux,
    Gateway,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::MacOS => write!(f, "macOS"),
            Platform::IOS => write!(f, "iOS"),
            Platform::Android => write!(f, "Android"),
            Platform::Windows => write!(f, "Windows"),
            Platform::Linux => write!(f, "Linux"),
            Platform::Gateway => write!(f, "Gateway"),
        }
    }
}

/// Run all E2E tests across platforms
pub async fn run_all_e2e_tests() -> Vec<TestResult> {
    let mut all_results = Vec::new();

    println!("╔══════════════════════════════════════════════════╗");
    println!("║           DX Agent E2E Test Suite                 ║");
    println!("╚══════════════════════════════════════════════════╝\n");

    // Gateway tests (run first as it's a dependency)
    println!("━━━ Gateway Tests ━━━");
    let gateway_config = GatewayTestConfig::default();
    let mut gateway_suite = GatewayTestSuite::new(gateway_config);
    let gateway_results = gateway_suite.run_all().await;
    all_results.extend(gateway_results.into_iter().map(|r| TestResult {
        name: r.name,
        platform: Platform::Gateway,
        passed: r.passed,
        duration_ms: r.duration_ms,
        error: r.error,
    }));

    // Platform-specific tests
    #[cfg(target_os = "macos")]
    {
        println!("\n━━━ macOS Tests ━━━");
        let macos_config = MacOSTestConfig::default();
        let mut macos_suite = MacOSTestSuite::new(macos_config);
        let macos_results = macos_suite.run_all().await;
        all_results.extend(macos_results.into_iter().map(|r| TestResult {
            name: r.name,
            platform: Platform::MacOS,
            passed: r.passed,
            duration_ms: r.duration_ms,
            error: r.error,
        }));

        println!("\n━━━ iOS Simulator Tests ━━━");
        let ios_config = IOSTestConfig::default();
        let mut ios_suite = IOSTestSuite::new(ios_config);
        let ios_results = ios_suite.run_all().await;
        all_results.extend(ios_results.into_iter().map(|r| TestResult {
            name: r.name,
            platform: Platform::IOS,
            passed: r.passed,
            duration_ms: r.duration_ms,
            error: r.error,
        }));
    }

    // Android tests (can run on any platform with ADB)
    println!("\n━━━ Android Tests ━━━");
    let android_config = AndroidTestConfig::default();
    let mut android_suite = AndroidTestSuite::new(android_config);
    let android_results = android_suite.run_all().await;
    all_results.extend(android_results.into_iter().map(|r| TestResult {
        name: r.name,
        platform: Platform::Android,
        passed: r.passed,
        duration_ms: r.duration_ms,
        error: r.error,
    }));

    // Print overall summary
    print_overall_summary(&all_results);

    all_results
}

fn print_overall_summary(results: &[TestResult]) {
    println!("\n╔══════════════════════════════════════════════════╗");
    println!("║               Overall Summary                     ║");
    println!("╚══════════════════════════════════════════════════╝\n");

    let by_platform: std::collections::HashMap<Platform, Vec<&TestResult>> =
        results.iter().fold(std::collections::HashMap::new(), |mut acc, r| {
            acc.entry(r.platform).or_default().push(r);
            acc
        });

    for (platform, tests) in &by_platform {
        let passed = tests.iter().filter(|t| t.passed).count();
        let total = tests.len();
        let total_time: u64 = tests.iter().map(|t| t.duration_ms).sum();

        let status = if passed == total { "✅" } else { "⚠️" };
        println!("  {} {}: {}/{} passed ({} ms)", status, platform, passed, total, total_time);
    }

    let total_passed = results.iter().filter(|r| r.passed).count();
    let total_tests = results.len();
    let total_time: u64 = results.iter().map(|r| r.duration_ms).sum();

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if total_passed == total_tests {
        println!("✅ ALL TESTS PASSED: {}/{} ({} ms)", total_passed, total_tests, total_time);
    } else {
        println!("❌ SOME TESTS FAILED: {}/{} ({} ms)", total_passed, total_tests, total_time);

        println!("\nFailed tests:");
        for result in results.iter().filter(|r| !r.passed) {
            println!("  - [{}] {}: {:?}", result.platform, result.name, result.error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_all_e2e() {
        let results = run_all_e2e_tests().await;

        // All tests should pass (even if some are skipped)
        let failures: Vec<_> = results.iter().filter(|r| !r.passed).collect();

        if !failures.is_empty() {
            for f in &failures {
                eprintln!("Failed: {} - {:?}", f.name, f.error);
            }
        }

        assert!(failures.is_empty(), "Some E2E tests failed");
    }
}
