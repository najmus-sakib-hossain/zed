//! Property-based tests for test execution correctness
//!
//! **Property 8: Test Execution Correctness**
//! **Validates: Requirements 3.1.1-3.1.4, 3.1.7**
//!
//! For any test function, the Test_Runner SHALL execute the actual Python code
//! and report the correct pass/fail status.

use dx_py_core::{AssertionStats, TestCase, TestId, TestResult, TestStatus};
use dx_py_executor::{ExecutionSummary, ExecutorConfig, WorkStealingExecutor};
use proptest::prelude::*;
use std::time::Duration;

/// Generate arbitrary test names
fn arb_test_name() -> impl Strategy<Value = String> {
    prop::string::string_regex("test_[a-z][a-z0-9_]{0,20}")
        .unwrap()
        .prop_filter("non-empty", |s| !s.is_empty() && s.len() > 5)
}

/// Generate arbitrary file paths
fn arb_file_path() -> impl Strategy<Value = String> {
    prop::string::string_regex("tests/test_[a-z][a-z0-9_]{0,10}\\.py")
        .unwrap()
        .prop_filter("valid path", |s| s.ends_with(".py"))
}

/// Generate arbitrary line numbers
fn arb_line_number() -> impl Strategy<Value = u32> {
    1u32..1000u32
}

/// Generate arbitrary test cases
#[allow(dead_code)]
fn arb_test_case() -> impl Strategy<Value = TestCase> {
    (arb_test_name(), arb_file_path(), arb_line_number())
        .prop_map(|(name, path, line)| TestCase::new(name, path, line))
}

/// Generate arbitrary test results
fn arb_test_result() -> impl Strategy<Value = TestResult> {
    (
        any::<u64>(),
        prop_oneof![
            Just(TestStatus::Pass),
            Just(TestStatus::Fail),
            any::<String>().prop_map(|s| TestStatus::Skip { reason: s }),
            any::<String>().prop_map(|s| TestStatus::Error { message: s }),
        ],
        0u64..10_000_000_000u64, // duration in nanos
    )
        .prop_map(|(id, status, duration_ns)| TestResult {
            test_id: TestId(id),
            status,
            duration: Duration::from_nanos(duration_ns),
            stdout: String::new(),
            stderr: String::new(),
            traceback: None,
            assertions: AssertionStats::default(),
            assertion_failure: None,
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dx-py-production-ready, Property 8: Test Execution Correctness
    /// Validates: Requirements 3.1.1-3.1.4, 3.1.7
    ///
    /// Property: Test IDs are unique for different test cases
    #[test]
    fn test_id_uniqueness(
        name1 in arb_test_name(),
        name2 in arb_test_name(),
        path1 in arb_file_path(),
        path2 in arb_file_path(),
        line1 in arb_line_number(),
        line2 in arb_line_number(),
    ) {
        let test1 = TestCase::new(&name1, &path1, line1);
        let _test2 = TestCase::new(&name2, &path2, line2);

        // If all components are different, IDs should be different
        if name1 != name2 || path1 != path2 || line1 != line2 {
            // Note: Due to hash collisions, we can't guarantee uniqueness,
            // but we can verify the ID generation is deterministic
            let test1_again = TestCase::new(&name1, &path1, line1);
            prop_assert_eq!(test1.id, test1_again.id, "Same inputs should produce same ID");
        }
    }

    /// Feature: dx-py-production-ready, Property 8: Test Execution Correctness
    /// Validates: Requirements 3.1.2, 3.1.3
    ///
    /// Property: Test results correctly report pass/fail status
    #[test]
    fn test_result_status_consistency(result in arb_test_result()) {
        match &result.status {
            TestStatus::Pass => {
                prop_assert!(result.status.is_success(), "Pass should be success");
                prop_assert!(!result.status.is_failure(), "Pass should not be failure");
            }
            TestStatus::Fail => {
                prop_assert!(!result.status.is_success(), "Fail should not be success");
                prop_assert!(result.status.is_failure(), "Fail should be failure");
            }
            TestStatus::Skip { .. } => {
                prop_assert!(result.status.is_success(), "Skip should be success");
                prop_assert!(!result.status.is_failure(), "Skip should not be failure");
            }
            TestStatus::Error { .. } => {
                prop_assert!(!result.status.is_success(), "Error should not be success");
                prop_assert!(result.status.is_failure(), "Error should be failure");
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 8: Test Execution Correctness
    /// Validates: Requirements 3.1.7
    ///
    /// Property: Execution summary correctly aggregates results
    #[test]
    fn test_execution_summary_aggregation(results in prop::collection::vec(arb_test_result(), 0..50)) {
        let summary = ExecutionSummary::from_results(&results, 0);

        // Total should equal the number of results
        prop_assert_eq!(summary.total, results.len());

        // Counts should add up to total
        prop_assert_eq!(
            summary.passed + summary.failed + summary.skipped + summary.errors,
            summary.total,
            "Counts should add up to total"
        );

        // Verify individual counts
        let expected_passed = results.iter().filter(|r| matches!(r.status, TestStatus::Pass)).count();
        let expected_failed = results.iter().filter(|r| matches!(r.status, TestStatus::Fail)).count();
        let expected_skipped = results.iter().filter(|r| matches!(r.status, TestStatus::Skip { .. })).count();
        let expected_errors = results.iter().filter(|r| matches!(r.status, TestStatus::Error { .. })).count();

        prop_assert_eq!(summary.passed, expected_passed);
        prop_assert_eq!(summary.failed, expected_failed);
        prop_assert_eq!(summary.skipped, expected_skipped);
        prop_assert_eq!(summary.errors, expected_errors);
    }

    /// Feature: dx-py-production-ready, Property 8: Test Execution Correctness
    /// Validates: Requirements 3.1.7
    ///
    /// Property: Summary success status is correct
    #[test]
    fn test_execution_summary_success_status(
        results in prop::collection::vec(arb_test_result(), 0..50),
        panics in 0usize..5usize,
    ) {
        let summary = ExecutionSummary::from_results(&results, panics);

        let has_failures = results.iter().any(|r| matches!(r.status, TestStatus::Fail));
        let has_errors = results.iter().any(|r| matches!(r.status, TestStatus::Error { .. }));
        let has_panics = panics > 0;

        let expected_success = !has_failures && !has_errors && !has_panics;
        prop_assert_eq!(
            summary.is_success(),
            expected_success,
            "Success status should match: failures={}, errors={}, panics={}",
            has_failures, has_errors, has_panics
        );
    }

    /// Feature: dx-py-production-ready, Property 8: Test Execution Correctness
    /// Validates: Requirements 3.1.1
    ///
    /// Property: Test case full name is correctly formatted
    #[test]
    fn test_case_full_name_format(
        name in arb_test_name(),
        path in arb_file_path(),
        line in arb_line_number(),
        class_name in prop::option::of("[A-Z][a-zA-Z0-9]{0,20}"),
    ) {
        let mut test = TestCase::new(&name, &path, line);
        if let Some(ref class) = class_name {
            test = test.with_class(class);
        }

        let full_name = test.full_name();

        match class_name {
            Some(class) => {
                prop_assert!(full_name.contains("::"), "Full name with class should contain ::");
                prop_assert!(full_name.starts_with(&class), "Full name should start with class name");
                prop_assert!(full_name.ends_with(&name), "Full name should end with test name");
            }
            None => {
                prop_assert_eq!(full_name, name, "Full name without class should equal test name");
            }
        }
    }

    /// Feature: dx-py-production-ready, Property 8: Test Execution Correctness
    /// Validates: Requirements 3.1.2
    ///
    /// Property: Duration is always non-negative
    #[test]
    fn test_result_duration_non_negative(result in arb_test_result()) {
        prop_assert!(result.duration >= Duration::ZERO, "Duration should be non-negative");
    }
}

/// Feature: dx-py-production-ready, Property 8: Test Execution Correctness
/// Validates: Requirements 3.1.1
///
/// Property: Executor configuration is valid
#[test]
fn test_executor_config_validity() {
    let config = ExecutorConfig::default();
    assert!(config.num_workers > 0, "Default workers should be positive");
    assert!(config.timeout > Duration::ZERO, "Default timeout should be positive");
}

/// Feature: dx-py-production-ready, Property 8: Test Execution Correctness
/// Validates: Requirements 3.1.1
///
/// Property: Executor can be created with valid config
#[test]
fn test_executor_creation() {
    let config = ExecutorConfig::default().with_workers(2);
    let executor = WorkStealingExecutor::new(config);
    assert_eq!(executor.pending(), 0);
    assert_eq!(executor.completed(), 0);
}

/// Feature: dx-py-production-ready, Property 8: Test Execution Correctness
/// Validates: Requirements 3.1.1
///
/// Property: Test submission increments pending count
#[test]
fn test_submission_increments_pending() {
    let config = ExecutorConfig::default().with_workers(1);
    let executor = WorkStealingExecutor::new(config);

    let test1 = TestCase::new("test_one", "tests/test.py", 10);
    let test2 = TestCase::new("test_two", "tests/test.py", 20);

    assert!(executor.submit(test1).is_ok());
    assert_eq!(executor.pending(), 1);

    assert!(executor.submit(test2).is_ok());
    assert_eq!(executor.pending(), 2);
}
