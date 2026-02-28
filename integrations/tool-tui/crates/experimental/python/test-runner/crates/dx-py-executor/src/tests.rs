//! Tests for the work-stealing executor

use super::*;

#[test]
fn test_executor_config_default() {
    let config = ExecutorConfig::default();
    assert!(config.num_workers > 0);
    assert!(config.fault_tolerant);
    assert_eq!(config.python_path, "python");
    assert_eq!(config.timeout, Duration::from_secs(60));
}

#[test]
fn test_executor_config_builder() {
    let config = ExecutorConfig::default()
        .with_workers(4)
        .with_fault_tolerance(false)
        .with_python("python3")
        .with_timeout(Duration::from_secs(30));

    assert_eq!(config.num_workers, 4);
    assert!(!config.fault_tolerant);
    assert_eq!(config.python_path, "python3");
    assert_eq!(config.timeout, Duration::from_secs(30));
}

#[test]
fn test_executor_submit() {
    let config = ExecutorConfig::default().with_workers(1);
    let executor = WorkStealingExecutor::new(config);

    let test = TestCase::new("test_example", "tests/test_example.py", 10);
    assert!(executor.submit(test).is_ok());
    assert_eq!(executor.pending(), 1);
}

#[test]
fn test_executor_submit_all() {
    let config = ExecutorConfig::default().with_workers(1);
    let executor = WorkStealingExecutor::new(config);

    let tests = vec![
        TestCase::new("test_one", "tests/test_one.py", 10),
        TestCase::new("test_two", "tests/test_two.py", 20),
        TestCase::new("test_three", "tests/test_three.py", 30),
    ];

    assert!(executor.submit_all(tests).is_ok());
    assert_eq!(executor.pending(), 3);
}

#[test]
fn test_executor_shutdown() {
    let config = ExecutorConfig::default().with_workers(1);
    let executor = WorkStealingExecutor::new(config);

    executor.shutdown();

    let test = TestCase::new("test_example", "tests/test_example.py", 10);
    assert!(executor.submit(test).is_err());
}

#[test]
fn test_execution_summary_from_results() {
    let results = vec![
        TestResult::pass(TestId::new(1, 1, 1), Duration::from_millis(100)),
        TestResult::pass(TestId::new(2, 2, 2), Duration::from_millis(150)),
        TestResult::fail(TestId::new(3, 3, 3), Duration::from_millis(200), "failed"),
        TestResult::skip(TestId::new(4, 4, 4), "skipped"),
        TestResult::error(TestId::new(5, 5, 5), "error"),
    ];

    let summary = ExecutionSummary::from_results(&results, 0);

    assert_eq!(summary.total, 5);
    assert_eq!(summary.passed, 2);
    assert_eq!(summary.failed, 1);
    assert_eq!(summary.skipped, 1);
    assert_eq!(summary.errors, 1);
    assert_eq!(summary.panics, 0);
    assert!(!summary.is_success());
}

#[test]
fn test_execution_summary_success() {
    let results = vec![
        TestResult::pass(TestId::new(1, 1, 1), Duration::from_millis(100)),
        TestResult::pass(TestId::new(2, 2, 2), Duration::from_millis(150)),
        TestResult::skip(TestId::new(3, 3, 3), "skipped"),
    ];

    let summary = ExecutionSummary::from_results(&results, 0);

    assert_eq!(summary.total, 3);
    assert_eq!(summary.passed, 2);
    assert_eq!(summary.skipped, 1);
    assert!(summary.is_success());
}

#[test]
fn test_execution_summary_with_panics() {
    let results = vec![TestResult::pass(
        TestId::new(1, 1, 1),
        Duration::from_millis(100),
    )];

    let summary = ExecutionSummary::from_results(&results, 1);

    assert_eq!(summary.panics, 1);
    assert!(!summary.is_success());
}
