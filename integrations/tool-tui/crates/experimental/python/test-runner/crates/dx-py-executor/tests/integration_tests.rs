//! Integration tests for test runner worker process
//!
//! These tests verify that the test runner can:
//! 1. Execute passing tests
//! 2. Report failing tests with assertion messages
//! 3. Handle errors gracefully
//!
//! **Validates: Requirements 8.1-8.7**

use dx_py_core::{TestCase, TestStatus};
use dx_py_daemon::{DaemonConfig, DaemonPool};
use std::fs;
use std::io::Write;
use std::process::Command;
use std::time::Duration;
use tempfile::TempDir;

/// Find a working Python executable
fn find_python() -> Option<String> {
    // Try common Python executable names
    let candidates = [
        "python",
        "python3",
        "py",
        ".venv/Scripts/python.exe",
        ".venv/bin/python",
    ];

    for candidate in candidates {
        if let Ok(output) = Command::new(candidate).arg("--version").output() {
            if output.status.success() {
                return Some(candidate.to_string());
            }
        }
    }

    None
}

/// Create a temporary Python test file with the given content
fn create_test_file(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    let mut file = fs::File::create(&path).expect("Failed to create test file");
    file.write_all(content.as_bytes())
        .expect("Failed to write test file");
    path
}

/// Create a daemon pool with the found Python executable
fn create_pool(pool_size: usize) -> Option<DaemonPool> {
    let python = find_python()?;
    let config = DaemonConfig::default()
        .with_pool_size(pool_size)
        .with_python(python)
        .with_timeout(Duration::from_secs(30));

    DaemonPool::new(config).ok()
}

/// Integration test: Passing test execution
/// Validates: Requirements 8.1, 8.2
#[test]
fn test_passing_test_execution() {
    let pool = match create_pool(1) {
        Some(p) => p,
        None => {
            eprintln!("Skipping test: Python not found");
            return;
        }
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a simple passing test
    let test_content = r#"
def test_passing():
    assert 1 + 1 == 2
"#;
    let test_path = create_test_file(&temp_dir, "test_passing.py", test_content);

    // Create test case
    let test = TestCase::new("test_passing", test_path.to_str().unwrap(), 2);

    // Acquire worker and execute test
    let worker_id = pool.acquire_worker().expect("Failed to acquire worker");
    let result = pool.execute_test(worker_id, &test).expect("Failed to execute test");
    pool.release_worker(worker_id).expect("Failed to release worker");

    // Verify result
    assert_eq!(result.status, TestStatus::Pass, "Test should pass");
    assert!(result.duration > Duration::ZERO, "Duration should be positive");
}

/// Integration test: Failing test with assertion error
/// Validates: Requirements 8.1, 8.3
#[test]
fn test_failing_test_with_assertion() {
    let pool = match create_pool(1) {
        Some(p) => p,
        None => {
            eprintln!("Skipping test: Python not found");
            return;
        }
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a failing test with assertion message
    let test_content = r#"
def test_failing():
    assert 1 == 2, "Expected 1 to equal 2"
"#;
    let test_path = create_test_file(&temp_dir, "test_failing.py", test_content);

    // Create test case
    let test = TestCase::new("test_failing", test_path.to_str().unwrap(), 2);

    // Execute test
    let worker_id = pool.acquire_worker().expect("Failed to acquire worker");
    let result = pool.execute_test(worker_id, &test).expect("Failed to execute test");
    pool.release_worker(worker_id).expect("Failed to release worker");

    // Verify result
    assert_eq!(result.status, TestStatus::Fail, "Test should fail");
    assert!(result.traceback.is_some(), "Should have traceback");
    let traceback = result.traceback.unwrap();
    assert!(
        traceback.contains("AssertionError"),
        "Traceback should contain AssertionError"
    );
}

/// Integration test: Test with exception (not assertion)
/// Validates: Requirements 8.1, 8.4
#[test]
fn test_error_with_exception() {
    let pool = match create_pool(1) {
        Some(p) => p,
        None => {
            eprintln!("Skipping test: Python not found");
            return;
        }
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a test that raises an exception
    let test_content = r#"
def test_error():
    raise ValueError("Something went wrong")
"#;
    let test_path = create_test_file(&temp_dir, "test_error.py", test_content);

    // Create test case
    let test = TestCase::new("test_error", test_path.to_str().unwrap(), 2);

    // Execute test
    let worker_id = pool.acquire_worker().expect("Failed to acquire worker");
    let result = pool.execute_test(worker_id, &test).expect("Failed to execute test");
    pool.release_worker(worker_id).expect("Failed to release worker");

    // Verify result
    assert!(
        matches!(result.status, TestStatus::Error { .. }),
        "Test should be error"
    );
    assert!(result.traceback.is_some(), "Should have traceback");
    let traceback = result.traceback.unwrap();
    assert!(
        traceback.contains("ValueError"),
        "Traceback should contain ValueError"
    );
}

/// Integration test: Test with stdout capture
/// Validates: Requirements 8.1, 8.2
#[test]
fn test_stdout_capture() {
    let pool = match create_pool(1) {
        Some(p) => p,
        None => {
            eprintln!("Skipping test: Python not found");
            return;
        }
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a test that prints output
    let test_content = r#"
def test_with_output():
    print("Hello from test")
    assert True
"#;
    let test_path = create_test_file(&temp_dir, "test_output.py", test_content);

    // Create test case
    let test = TestCase::new("test_with_output", test_path.to_str().unwrap(), 2);

    // Execute test
    let worker_id = pool.acquire_worker().expect("Failed to acquire worker");
    let result = pool.execute_test(worker_id, &test).expect("Failed to execute test");
    pool.release_worker(worker_id).expect("Failed to release worker");

    // Verify result
    assert_eq!(result.status, TestStatus::Pass, "Test should pass");
    assert!(
        result.stdout.contains("Hello from test"),
        "Should capture stdout"
    );
}

/// Integration test: Test class method execution
/// Validates: Requirements 8.1, 8.2
#[test]
fn test_class_method_execution() {
    let pool = match create_pool(1) {
        Some(p) => p,
        None => {
            eprintln!("Skipping test: Python not found");
            return;
        }
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a test class
    let test_content = r#"
class TestClass:
    def test_method(self):
        assert 2 + 2 == 4
"#;
    let test_path = create_test_file(&temp_dir, "test_class.py", test_content);

    // Create test case with class
    let test = TestCase::new("test_method", test_path.to_str().unwrap(), 3).with_class("TestClass");

    // Execute test
    let worker_id = pool.acquire_worker().expect("Failed to acquire worker");
    let result = pool.execute_test(worker_id, &test).expect("Failed to execute test");
    pool.release_worker(worker_id).expect("Failed to release worker");

    // Verify result
    assert_eq!(result.status, TestStatus::Pass, "Test should pass");
}

/// Integration test: Multiple tests in sequence
/// Validates: Requirements 8.1, 8.5, 8.6
#[test]
fn test_multiple_tests_sequence() {
    let pool = match create_pool(1) {
        Some(p) => p,
        None => {
            eprintln!("Skipping test: Python not found");
            return;
        }
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create multiple test files
    let test1_content = r#"
def test_one():
    assert True
"#;
    let test2_content = r#"
def test_two():
    assert 1 + 1 == 2
"#;
    let test3_content = r#"
def test_three():
    assert "hello".upper() == "HELLO"
"#;

    let test1_path = create_test_file(&temp_dir, "test_one.py", test1_content);
    let test2_path = create_test_file(&temp_dir, "test_two.py", test2_content);
    let test3_path = create_test_file(&temp_dir, "test_three.py", test3_content);

    // Execute tests in sequence
    let tests = vec![
        TestCase::new("test_one", test1_path.to_str().unwrap(), 2),
        TestCase::new("test_two", test2_path.to_str().unwrap(), 2),
        TestCase::new("test_three", test3_path.to_str().unwrap(), 2),
    ];

    let worker_id = pool.acquire_worker().expect("Failed to acquire worker");

    for test in &tests {
        let result = pool.execute_test(worker_id, test).expect("Failed to execute test");
        assert_eq!(
            result.status,
            TestStatus::Pass,
            "Test {} should pass",
            test.name
        );
    }

    pool.release_worker(worker_id).expect("Failed to release worker");
}

/// Integration test: Worker recovery after error
/// Validates: Requirements 8.5, 8.6, 8.7
#[test]
fn test_worker_recovery_after_error() {
    let pool = match create_pool(1) {
        Some(p) => p,
        None => {
            eprintln!("Skipping test: Python not found");
            return;
        }
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a test that errors
    let error_content = r#"
def test_error():
    raise RuntimeError("Intentional error")
"#;
    // Create a test that passes
    let pass_content = r#"
def test_pass():
    assert True
"#;

    let error_path = create_test_file(&temp_dir, "test_error.py", error_content);
    let pass_path = create_test_file(&temp_dir, "test_pass.py", pass_content);

    let worker_id = pool.acquire_worker().expect("Failed to acquire worker");

    // Execute error test
    let error_test = TestCase::new("test_error", error_path.to_str().unwrap(), 2);
    let error_result = pool
        .execute_test(worker_id, &error_test)
        .expect("Failed to execute test");
    assert!(
        matches!(error_result.status, TestStatus::Error { .. }),
        "Error test should report error"
    );

    // Execute passing test - worker should still work
    let pass_test = TestCase::new("test_pass", pass_path.to_str().unwrap(), 2);
    let pass_result = pool
        .execute_test(worker_id, &pass_test)
        .expect("Failed to execute test");
    assert_eq!(
        pass_result.status,
        TestStatus::Pass,
        "Pass test should pass after error"
    );

    pool.release_worker(worker_id).expect("Failed to release worker");
}

/// Integration test: Import error handling
/// Validates: Requirements 8.4
#[test]
fn test_import_error_handling() {
    let pool = match create_pool(1) {
        Some(p) => p,
        None => {
            eprintln!("Skipping test: Python not found");
            return;
        }
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a test with import error
    let test_content = r#"
import nonexistent_module_xyz

def test_import():
    assert True
"#;
    let test_path = create_test_file(&temp_dir, "test_import.py", test_content);

    // Create test case
    let test = TestCase::new("test_import", test_path.to_str().unwrap(), 4);

    // Execute test
    let worker_id = pool.acquire_worker().expect("Failed to acquire worker");
    let result = pool.execute_test(worker_id, &test).expect("Failed to execute test");
    pool.release_worker(worker_id).expect("Failed to release worker");

    // Verify result - should be error due to import failure
    assert!(
        matches!(result.status, TestStatus::Error { .. }),
        "Test should be error due to import failure"
    );
    assert!(result.traceback.is_some(), "Should have traceback");
    let traceback = result.traceback.unwrap();
    assert!(
        traceback.contains("ModuleNotFoundError") || traceback.contains("ImportError"),
        "Traceback should contain import error"
    );
}

/// Integration test: Syntax error handling
/// Validates: Requirements 8.4
#[test]
fn test_syntax_error_handling() {
    let pool = match create_pool(1) {
        Some(p) => p,
        None => {
            eprintln!("Skipping test: Python not found");
            return;
        }
    };

    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a test with syntax error
    let test_content = r#"
def test_syntax(
    # Missing closing parenthesis
    assert True
"#;
    let test_path = create_test_file(&temp_dir, "test_syntax.py", test_content);

    // Create test case
    let test = TestCase::new("test_syntax", test_path.to_str().unwrap(), 2);

    // Execute test
    let worker_id = pool.acquire_worker().expect("Failed to acquire worker");
    let result = pool.execute_test(worker_id, &test).expect("Failed to execute test");
    pool.release_worker(worker_id).expect("Failed to release worker");

    // Verify result - should be error due to syntax error
    assert!(
        matches!(result.status, TestStatus::Error { .. }),
        "Test should be error due to syntax error"
    );
}

/// Integration test: Worker ping functionality
/// Validates: Requirements 8.5
#[test]
fn test_worker_ping() {
    let pool = match create_pool(1) {
        Some(p) => p,
        None => {
            eprintln!("Skipping test: Python not found");
            return;
        }
    };

    // Worker should be alive
    assert!(pool.is_worker_alive(0), "Worker should be alive");

    // Ping should succeed
    assert!(pool.ping_worker(0), "Ping should succeed");
}

/// Integration test: Pool shutdown
/// Validates: Requirements 8.5
#[test]
fn test_pool_shutdown() {
    let pool = match create_pool(2) {
        Some(p) => p,
        None => {
            eprintln!("Skipping test: Python not found");
            return;
        }
    };

    // Verify pool is running
    assert!(!pool.is_shutdown(), "Pool should not be shutdown");
    assert_eq!(pool.available_workers(), 2, "Should have 2 available workers");

    // Shutdown pool
    pool.shutdown().expect("Failed to shutdown pool");

    // Verify pool is shutdown
    assert!(pool.is_shutdown(), "Pool should be shutdown");
}
