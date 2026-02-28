//! Tests for the daemon pool

use super::*;
use std::path::PathBuf;

#[test]
fn test_daemon_config_default() {
    let config = DaemonConfig::default();
    assert!(config.pool_size > 0);
    assert_eq!(config.python_path, "python");
    assert_eq!(config.timeout, Duration::from_secs(60));
}

#[test]
fn test_daemon_config_builder() {
    let config = DaemonConfig::default()
        .with_pool_size(4)
        .with_python("python3")
        .with_timeout(Duration::from_secs(30))
        .with_preload(vec!["pytest".to_string()]);

    assert_eq!(config.pool_size, 4);
    assert_eq!(config.python_path, "python3");
    assert_eq!(config.timeout, Duration::from_secs(30));
    assert_eq!(config.preload_modules, vec!["pytest".to_string()]);
}

#[test]
fn test_worker_state() {
    let mut worker = TestWorker::new(0);
    assert_eq!(worker.state, WorkerState::Idle);
    assert!(!worker.is_available()); // No process yet

    worker.mark_busy();
    assert_eq!(worker.state, WorkerState::Busy);

    worker.mark_idle();
    assert_eq!(worker.state, WorkerState::Idle);

    worker.mark_crashed();
    assert_eq!(worker.state, WorkerState::Crashed);
}

#[test]
fn test_test_case_creation() {
    let test = TestCase::new("test_example", "tests/test_example.py", 10);
    assert_eq!(test.name, "test_example");
    assert_eq!(test.file_path, PathBuf::from("tests/test_example.py"));
    assert_eq!(test.line_number, 10);
    assert!(test.class_name.is_none());
}

#[test]
fn test_test_case_with_class() {
    let test = TestCase::new("test_method", "tests/test_class.py", 20).with_class("TestClass");
    assert_eq!(test.name, "test_method");
    assert_eq!(test.class_name, Some("TestClass".to_string()));
    assert_eq!(test.full_name(), "TestClass::test_method");
}

#[test]
fn test_test_result_pass() {
    let test_id = TestId::new(123, 10, 456);
    let result = TestResult::pass(test_id, Duration::from_millis(100));
    assert_eq!(result.test_id, test_id);
    assert_eq!(result.status, TestStatus::Pass);
    assert!(result.status.is_success());
    assert!(!result.status.is_failure());
}

#[test]
fn test_test_result_fail() {
    let test_id = TestId::new(123, 10, 456);
    let result = TestResult::fail(test_id, Duration::from_millis(100), "assertion failed");
    assert_eq!(result.status, TestStatus::Fail);
    assert!(!result.status.is_success());
    assert!(result.status.is_failure());
    assert_eq!(result.traceback, Some("assertion failed".to_string()));
}

#[test]
fn test_test_result_skip() {
    let test_id = TestId::new(123, 10, 456);
    let result = TestResult::skip(test_id, "not implemented");
    assert!(matches!(&result.status, TestStatus::Skip { reason } if reason == "not implemented"));
    assert!(result.status.is_success());
}

#[test]
fn test_test_result_error() {
    let test_id = TestId::new(123, 10, 456);
    let result = TestResult::error(test_id, "import error");
    assert!(matches!(&result.status, TestStatus::Error { message } if message == "import error"));
    assert!(result.status.is_failure());
}

#[test]
fn test_worker_crash_stats() {
    let mut worker = TestWorker::new(0);
    assert_eq!(worker.get_restart_count(), 0);
    assert!(worker.get_last_crash_reason().is_none());

    worker.record_crash("Test crash".to_string());
    assert_eq!(worker.get_restart_count(), 1);
    assert_eq!(worker.get_last_crash_reason(), Some("Test crash"));

    worker.record_crash("Another crash".to_string());
    assert_eq!(worker.get_restart_count(), 2);
    assert_eq!(worker.get_last_crash_reason(), Some("Another crash"));
}

#[test]
fn test_worker_request_serialization() {
    let request = WorkerRequest::Run {
        module_path: "test.py".to_string(),
        function_name: "test_example".to_string(),
        class_name: None,
    };
    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("\"type\":\"run\""));
    assert!(json.contains("\"module_path\":\"test.py\""));
}

#[test]
fn test_worker_response_deserialization() {
    let json = r#"{"status": "pass", "duration_ns": 1000000, "stdout": "", "stderr": ""}"#;
    let response: WorkerResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.status, "pass");
    assert_eq!(response.duration_ns, Some(1000000));
}

#[test]
fn test_worker_response_with_error() {
    let json = r#"{"status": "error", "message": "Import failed", "traceback": "Traceback..."}"#;
    let response: WorkerResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.status, "error");
    assert_eq!(response.message, Some("Import failed".to_string()));
    assert!(response.traceback.is_some());
}

#[test]
fn test_worker_response_with_assertion_failure() {
    let json = r#"{"status": "fail", "message": "Assertion failed", "duration_ns": 500000, "stdout": "output", "stderr": "", "traceback": "AssertionError: ..."}"#;
    let response: WorkerResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.status, "fail");
    assert_eq!(response.message, Some("Assertion failed".to_string()));
    assert_eq!(response.duration_ns, Some(500000));
    assert_eq!(response.stdout, Some("output".to_string()));
    assert!(response.traceback.is_some());
}
