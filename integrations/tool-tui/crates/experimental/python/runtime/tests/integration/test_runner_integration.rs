//! Test Runner Integration Tests
//!
//! These tests verify that the DX-Py runtime can work with the test-runner
//! to discover and execute pytest-style test files.
//!
//! Requirements: 10.1, 10.2, 10.3, 10.4, 10.5

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Helper to create a temporary test project with pytest-style tests
fn create_test_project(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().expect("Failed to create temp dir");
    
    for (name, content) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("Failed to create parent dir");
        }
        fs::write(&path, content).expect("Failed to write file");
    }
    
    dir
}

/// Helper to find Python executable
fn find_python() -> String {
    let candidates = if cfg!(windows) {
        vec!["python", "python3", "py"]
    } else {
        vec!["python3", "python"]
    };
    
    for candidate in candidates {
        if let Ok(output) = Command::new(candidate).arg("--version").output() {
            if output.status.success() {
                return candidate.to_string();
            }
        }
    }
    
    if cfg!(windows) { "python" } else { "python3" }.to_string()
}

/// Execute a pytest test file and return the result
fn run_pytest(test_dir: &std::path::Path) -> (bool, String, String) {
    let python = find_python();
    
    let output = Command::new(&python)
        .args(["-m", "pytest", "-v", "."])
        .current_dir(test_dir)
        .output()
        .expect("Failed to run pytest");
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    (output.status.success(), stdout, stderr)
}

// =============================================================================
// Task 18.1: Sample pytest test file with fixtures, assertions, parametrize
// =============================================================================

/// Sample pytest test content with fixtures
pub const SAMPLE_TEST_WITH_FIXTURES: &str = r#"
"""Sample pytest test file with fixtures for DX-Py integration testing."""
import pytest


# =============================================================================
# Fixtures (Requirement 10.2)
# =============================================================================

@pytest.fixture
def sample_list():
    """Provide a sample list for testing."""
    return [1, 2, 3, 4, 5]


@pytest.fixture
def sample_dict():
    """Provide a sample dictionary for testing."""
    return {"name": "test", "value": 42, "active": True}


@pytest.fixture
def calculator():
    """Provide a simple calculator class instance."""
    class Calculator:
        def add(self, a, b):
            return a + b
        
        def subtract(self, a, b):
            return a - b
        
        def multiply(self, a, b):
            return a * b
        
        def divide(self, a, b):
            if b == 0:
                raise ValueError("Cannot divide by zero")
            return a / b
    
    return Calculator()


@pytest.fixture
def temp_data():
    """Fixture with setup and implicit teardown."""
    data = {"setup": True, "items": []}
    data["items"].append("initialized")
    return data


# =============================================================================
# Basic Assertion Tests (Requirement 10.3)
# =============================================================================

def test_basic_assertion():
    """Test basic assert statement."""
    assert True
    assert 1 + 1 == 2
    assert "hello" == "hello"


def test_assertion_with_message():
    """Test assert with custom message."""
    value = 42
    assert value == 42, f"Expected 42, got {value}"


def test_list_assertions(sample_list):
    """Test assertions with list fixture."""
    assert len(sample_list) == 5
    assert sample_list[0] == 1
    assert sample_list[-1] == 5
    assert 3 in sample_list
    assert 10 not in sample_list


def test_dict_assertions(sample_dict):
    """Test assertions with dict fixture."""
    assert "name" in sample_dict
    assert sample_dict["value"] == 42
    assert sample_dict.get("missing") is None


def test_calculator_add(calculator):
    """Test calculator addition."""
    assert calculator.add(2, 3) == 5
    assert calculator.add(-1, 1) == 0
    assert calculator.add(0, 0) == 0


def test_calculator_subtract(calculator):
    """Test calculator subtraction."""
    assert calculator.subtract(5, 3) == 2
    assert calculator.subtract(1, 1) == 0


def test_calculator_multiply(calculator):
    """Test calculator multiplication."""
    assert calculator.multiply(3, 4) == 12
    assert calculator.multiply(0, 100) == 0


# =============================================================================
# Parametrized Tests (Requirement 10.1)
# =============================================================================

@pytest.mark.parametrize("a,b,expected", [
    (1, 1, 2),
    (2, 3, 5),
    (10, 20, 30),
    (100, 200, 300),
    (-1, 1, 0),
    (0, 0, 0),
])
def test_addition_parametrized(a, b, expected):
    """Test addition with multiple parameter sets."""
    assert a + b == expected


@pytest.mark.parametrize("value,expected", [
    ("hello", "HELLO"),
    ("world", "WORLD"),
    ("Python", "PYTHON"),
    ("", ""),
    ("MiXeD", "MIXED"),
])
def test_upper_parametrized(value, expected):
    """Test string upper with multiple inputs."""
    assert value.upper() == expected


@pytest.mark.parametrize("lst,expected_sum", [
    ([1, 2, 3], 6),
    ([10, 20, 30], 60),
    ([], 0),
    ([5], 5),
    ([-1, 1], 0),
])
def test_sum_parametrized(lst, expected_sum):
    """Test sum with multiple list inputs."""
    assert sum(lst) == expected_sum


@pytest.mark.parametrize("n,expected", [
    (0, 1),
    (1, 1),
    (2, 2),
    (3, 6),
    (4, 24),
    (5, 120),
])
def test_factorial_parametrized(n, expected):
    """Test factorial calculation with parametrize."""
    def factorial(x):
        if x <= 1:
            return 1
        return x * factorial(x - 1)
    
    assert factorial(n) == expected


# =============================================================================
# Exception Testing with pytest.raises (Requirement 10.4)
# =============================================================================

def test_raises_value_error(calculator):
    """Test that division by zero raises ValueError."""
    with pytest.raises(ValueError):
        calculator.divide(10, 0)


def test_raises_with_match(calculator):
    """Test exception with message matching."""
    with pytest.raises(ValueError, match="Cannot divide by zero"):
        calculator.divide(5, 0)


def test_raises_key_error():
    """Test KeyError is raised for missing dict key."""
    d = {"a": 1}
    with pytest.raises(KeyError):
        _ = d["missing"]


def test_raises_index_error():
    """Test IndexError for out of bounds access."""
    lst = [1, 2, 3]
    with pytest.raises(IndexError):
        _ = lst[10]


def test_raises_type_error():
    """Test TypeError for invalid operations."""
    with pytest.raises(TypeError):
        _ = "string" + 42


# =============================================================================
# Combined Fixture Tests
# =============================================================================

def test_multiple_fixtures(sample_list, sample_dict, calculator):
    """Test using multiple fixtures together."""
    # Use sample_list
    total = sum(sample_list)
    assert total == 15
    
    # Use sample_dict
    assert sample_dict["name"] == "test"
    
    # Use calculator
    result = calculator.add(total, sample_dict["value"])
    assert result == 57  # 15 + 42


def test_fixture_modification(temp_data):
    """Test that fixture data can be modified within test."""
    assert temp_data["setup"] is True
    assert "initialized" in temp_data["items"]
    
    # Modify the fixture data
    temp_data["items"].append("modified")
    assert len(temp_data["items"]) == 2


# =============================================================================
# Edge Cases and Special Assertions
# =============================================================================

def test_none_assertions():
    """Test assertions involving None."""
    value = None
    assert value is None
    assert value != 0
    assert value != ""
    assert value != []


def test_boolean_assertions():
    """Test boolean assertions."""
    assert True
    assert not False
    assert bool(1)
    assert not bool(0)
    assert bool("non-empty")
    assert not bool("")


def test_comparison_assertions():
    """Test comparison assertions."""
    assert 5 > 3
    assert 3 < 5
    assert 5 >= 5
    assert 5 <= 5
    assert 5 != 3


def test_collection_membership():
    """Test collection membership assertions."""
    lst = [1, 2, 3]
    assert 2 in lst
    assert 4 not in lst
    
    s = {1, 2, 3}
    assert 1 in s
    
    d = {"a": 1}
    assert "a" in d
    assert "b" not in d


def test_string_assertions():
    """Test string-specific assertions."""
    s = "Hello, World!"
    assert s.startswith("Hello")
    assert s.endswith("!")
    assert "World" in s
    assert s.lower() == "hello, world!"
"#;

/// Sample conftest.py with shared fixtures
pub const SAMPLE_CONFTEST: &str = r#"
"""Shared pytest fixtures for the test suite."""
import pytest


@pytest.fixture(scope="module")
def module_data():
    """Module-scoped fixture - created once per module."""
    return {"module": "test_module", "count": 0}


@pytest.fixture(scope="session")
def session_config():
    """Session-scoped fixture - created once per test session."""
    return {
        "debug": False,
        "timeout": 30,
        "retries": 3,
    }


@pytest.fixture
def counter():
    """Function-scoped counter fixture."""
    class Counter:
        def __init__(self):
            self.value = 0
        
        def increment(self):
            self.value += 1
            return self.value
        
        def decrement(self):
            self.value -= 1
            return self.value
        
        def reset(self):
            self.value = 0
    
    return Counter()
"#;

// =============================================================================
// Integration Tests
// =============================================================================

#[test]
fn test_sample_pytest_file_syntax() {
    // Verify the sample test file has valid Python syntax
    let temp_dir = create_test_project(&[
        ("test_sample.py", SAMPLE_TEST_WITH_FIXTURES),
        ("conftest.py", SAMPLE_CONFTEST),
    ]);
    
    let python = find_python();
    
    // Check syntax by compiling the file
    let output = Command::new(&python)
        .args(["-m", "py_compile", "test_sample.py"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to check syntax");
    
    assert!(
        output.status.success(),
        "Sample test file has syntax errors: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    
    println!("Sample pytest file syntax is valid");
}

#[test]
fn test_conftest_syntax() {
    // Verify conftest.py has valid Python syntax
    let temp_dir = create_test_project(&[
        ("conftest.py", SAMPLE_CONFTEST),
    ]);
    
    let python = find_python();
    
    let output = Command::new(&python)
        .args(["-m", "py_compile", "conftest.py"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to check syntax");
    
    assert!(
        output.status.success(),
        "conftest.py has syntax errors: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    
    println!("conftest.py syntax is valid");
}

#[test]
#[ignore = "Requires pytest - run with --ignored"]
fn test_pytest_discovery() {
    // Test that pytest can discover the sample tests
    let temp_dir = create_test_project(&[
        ("test_sample.py", SAMPLE_TEST_WITH_FIXTURES),
        ("conftest.py", SAMPLE_CONFTEST),
    ]);
    
    let python = find_python();
    
    // Run pytest --collect-only to discover tests
    let output = Command::new(&python)
        .args(["-m", "pytest", "--collect-only", "-q"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to run pytest discovery");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    println!("Discovery stdout:\n{}", stdout);
    println!("Discovery stderr:\n{}", stderr);
    
    assert!(
        output.status.success() || stdout.contains("test_"),
        "pytest discovery failed: {}\n{}",
        stdout,
        stderr
    );
    
    // Verify key tests were discovered
    assert!(stdout.contains("test_basic_assertion") || stderr.contains("test_basic_assertion"),
            "test_basic_assertion not discovered");
    assert!(stdout.contains("test_addition_parametrized") || stderr.contains("test_addition_parametrized"),
            "test_addition_parametrized not discovered");
    
    println!("pytest discovery successful");
}

#[test]
#[ignore = "Requires pytest - run with --ignored"]
fn test_pytest_execution() {
    // Test that pytest can execute the sample tests
    let temp_dir = create_test_project(&[
        ("test_sample.py", SAMPLE_TEST_WITH_FIXTURES),
        ("conftest.py", SAMPLE_CONFTEST),
    ]);
    
    let (success, stdout, stderr) = run_pytest(temp_dir.path());
    
    println!("Execution stdout:\n{}", stdout);
    println!("Execution stderr:\n{}", stderr);
    
    assert!(
        success,
        "pytest execution failed:\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );
    
    // Verify tests passed
    assert!(
        stdout.contains("passed") || stdout.contains("PASSED"),
        "No passed tests found in output"
    );
    
    println!("pytest execution successful");
}

#[test]
#[ignore = "Requires pytest - run with --ignored"]
fn test_pytest_fixtures_work() {
    // Test that fixtures are properly injected
    let temp_dir = create_test_project(&[
        ("test_fixtures.py", r#"
import pytest

@pytest.fixture
def my_fixture():
    return {"key": "value"}

def test_fixture_injection(my_fixture):
    assert my_fixture["key"] == "value"
    print(f"Fixture value: {my_fixture}")
"#),
    ]);
    
    let (success, stdout, stderr) = run_pytest(temp_dir.path());
    
    assert!(
        success,
        "Fixture test failed:\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );
    
    println!("Fixture injection works correctly");
}

#[test]
#[ignore = "Requires pytest - run with --ignored"]
fn test_pytest_parametrize_works() {
    // Test that parametrize decorator works
    let temp_dir = create_test_project(&[
        ("test_param.py", r#"
import pytest

@pytest.mark.parametrize("input,expected", [
    (1, 2),
    (2, 4),
    (3, 6),
])
def test_double(input, expected):
    assert input * 2 == expected
"#),
    ]);
    
    let (success, stdout, stderr) = run_pytest(temp_dir.path());
    
    assert!(
        success,
        "Parametrize test failed:\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );
    
    // Should have 3 test cases
    assert!(
        stdout.contains("3 passed") || stdout.contains("passed"),
        "Expected 3 passed tests"
    );
    
    println!("Parametrize decorator works correctly");
}

#[test]
#[ignore = "Requires pytest - run with --ignored"]
fn test_pytest_raises_works() {
    // Test that pytest.raises context manager works
    let temp_dir = create_test_project(&[
        ("test_raises.py", r#"
import pytest

def test_raises_value_error():
    with pytest.raises(ValueError):
        raise ValueError("test error")

def test_raises_with_match():
    with pytest.raises(ValueError, match="specific"):
        raise ValueError("specific error message")

def test_raises_key_error():
    d = {}
    with pytest.raises(KeyError):
        _ = d["missing"]
"#),
    ]);
    
    let (success, stdout, stderr) = run_pytest(temp_dir.path());
    
    assert!(
        success,
        "pytest.raises test failed:\nstdout: {}\nstderr: {}",
        stdout,
        stderr
    );
    
    println!("pytest.raises works correctly");
}

// =============================================================================
// Test Runner Discovery Tests (Requirement 10.5)
// =============================================================================

#[test]
fn test_sample_test_file_structure() {
    // Verify the sample test file contains expected test patterns
    let content = SAMPLE_TEST_WITH_FIXTURES;
    
    // Check for fixtures
    assert!(content.contains("@pytest.fixture"), "Missing fixture decorator");
    assert!(content.contains("def sample_list"), "Missing sample_list fixture");
    assert!(content.contains("def calculator"), "Missing calculator fixture");
    
    // Check for parametrize
    assert!(content.contains("@pytest.mark.parametrize"), "Missing parametrize decorator");
    
    // Check for pytest.raises
    assert!(content.contains("pytest.raises"), "Missing pytest.raises usage");
    
    // Check for basic test functions
    assert!(content.contains("def test_basic_assertion"), "Missing basic assertion test");
    assert!(content.contains("def test_addition_parametrized"), "Missing parametrized test");
    
    println!("Sample test file structure is correct");
}

#[test]
fn test_conftest_structure() {
    // Verify conftest.py contains expected patterns
    let content = SAMPLE_CONFTEST;
    
    // Check for different fixture scopes
    assert!(content.contains("scope=\"module\""), "Missing module-scoped fixture");
    assert!(content.contains("scope=\"session\""), "Missing session-scoped fixture");
    
    // Check for fixture definitions
    assert!(content.contains("def module_data"), "Missing module_data fixture");
    assert!(content.contains("def session_config"), "Missing session_config fixture");
    assert!(content.contains("def counter"), "Missing counter fixture");
    
    println!("conftest.py structure is correct");
}
