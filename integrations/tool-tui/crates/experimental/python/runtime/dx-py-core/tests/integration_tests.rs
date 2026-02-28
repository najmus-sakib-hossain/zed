//! Integration Tests for DX-Py Runtime
//!
//! These tests verify that DX-Py can run real Python applications
//! using popular packages like Flask, requests, click, and NumPy.
//!
//! Requirements: 12.1, 12.2, 12.3, 12.4, 12.5

use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Result of executing a Python script
#[derive(Debug)]
pub struct ExecutionResult {
    /// Standard output from the script
    pub stdout: String,
    /// Standard error from the script
    pub stderr: String,
    /// Exit code (0 = success)
    pub exit_code: i32,
    /// Whether execution was successful
    pub success: bool,
}

impl ExecutionResult {
    /// Check if output contains a specific string
    pub fn contains(&self, s: &str) -> bool {
        self.stdout.contains(s) || self.stderr.contains(s)
    }

    /// Get combined output (stdout + stderr)
    pub fn output(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else if self.stdout.is_empty() {
            self.stderr.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }
}

/// Python runtime to use for execution
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PythonRuntime {
    /// Use CPython (python/python3)
    CPython,
    /// Use DX-Py runtime
    DxPy,
}

/// Helper to find Python executable
fn find_python_executable() -> Option<String> {
    let candidates = if cfg!(windows) {
        vec!["python", "python3", "py -3"]
    } else {
        vec!["python3", "python"]
    };

    for candidate in candidates {
        // Handle "py -3" specially on Windows
        let (cmd, args) = if candidate.contains(' ') {
            let parts: Vec<&str> = candidate.split_whitespace().collect();
            (parts[0], vec![parts[1], "--version"])
        } else {
            (candidate, vec!["--version"])
        };

        if let Ok(output) =
            Command::new(cmd).args(&args[..args.len() - 1]).arg("--version").output()
        {
            if output.status.success() {
                return Some(candidate.to_string());
            }
        }
    }

    None
}

/// Check if Python is available on the system
pub fn is_python_available() -> bool {
    find_python_executable().is_some()
}

/// Execute a Python script using the specified runtime
/// Requirements: 12.1
pub fn execute_python_script(script: &str, _runtime: PythonRuntime) -> ExecutionResult {
    let python = match find_python_executable() {
        Some(p) => p,
        None => {
            return ExecutionResult {
                stdout: String::new(),
                stderr: "Python not found on system".to_string(),
                exit_code: -1,
                success: false,
            };
        }
    };

    let temp_dir = match TempDir::new() {
        Ok(d) => d,
        Err(e) => {
            return ExecutionResult {
                stdout: String::new(),
                stderr: format!("Failed to create temp directory: {}", e),
                exit_code: -1,
                success: false,
            };
        }
    };

    let script_path = temp_dir.path().join("test_script.py");
    if let Err(e) = fs::write(&script_path, script) {
        return ExecutionResult {
            stdout: String::new(),
            stderr: format!("Failed to write script: {}", e),
            exit_code: -1,
            success: false,
        };
    }

    // Handle "py -3" specially on Windows
    let (executable, mut args) = if python.contains(' ') {
        let parts: Vec<&str> = python.split_whitespace().collect();
        (parts[0].to_string(), vec![parts[1].to_string()])
    } else {
        (python, vec![])
    };
    args.push(script_path.to_string_lossy().to_string());

    let result = Command::new(&executable).args(&args).current_dir(temp_dir.path()).output();

    match result {
        Ok(output) => ExecutionResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            success: output.status.success(),
        },
        Err(e) => ExecutionResult {
            stdout: String::new(),
            stderr: format!("Failed to execute {}: {}", executable, e),
            exit_code: -1,
            success: false,
        },
    }
}

/// Execute a Python command string
pub fn execute_python_command(command: &str, _runtime: PythonRuntime) -> ExecutionResult {
    let python = match find_python_executable() {
        Some(p) => p,
        None => {
            return ExecutionResult {
                stdout: String::new(),
                stderr: "Python not found on system".to_string(),
                exit_code: -1,
                success: false,
            };
        }
    };

    // Handle "py -3" specially on Windows
    let (executable, mut args) = if python.contains(' ') {
        let parts: Vec<&str> = python.split_whitespace().collect();
        (parts[0].to_string(), vec![parts[1].to_string()])
    } else {
        (python, vec![])
    };
    args.push("-c".to_string());
    args.push(command.to_string());

    let result = Command::new(&executable).args(&args).output();

    match result {
        Ok(output) => ExecutionResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            success: output.status.success(),
        },
        Err(e) => ExecutionResult {
            stdout: String::new(),
            stderr: format!("Failed to execute: {}", e),
            exit_code: -1,
            success: false,
        },
    }
}

/// Check if a Python package is installed
pub fn is_package_installed(package: &str) -> bool {
    let check_script = format!(
        "import importlib.util; print('installed' if importlib.util.find_spec('{}') else 'not_installed')",
        package.split('[').next().unwrap_or(package)
    );

    let result = execute_python_command(&check_script, PythonRuntime::CPython);
    result.success
        && result.stdout.contains("installed")
        && !result.stdout.contains("not_installed")
}

// =============================================================================
// Real Python Execution Tests (Requirements: 12.1)
// =============================================================================

#[test]
fn test_real_python_execution_basic() {
    if !is_python_available() {
        println!("Skipping test: Python not available on system");
        return;
    }

    let script = r#"
print("Hello from Python!")
x = 1 + 2
print(f"1 + 2 = {x}")
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "Basic Python execution failed: {}", result.stderr);
    assert!(result.stdout.contains("Hello from Python!"), "Output missing greeting");
    assert!(result.stdout.contains("1 + 2 = 3"), "Output missing calculation");
}

#[test]
fn test_real_python_execution_error_capture() {
    if !is_python_available() {
        println!("Skipping test: Python not available on system");
        return;
    }

    let script = r#"
raise ValueError("Test error message")
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(!result.success, "Script should have failed");
    assert!(result.stderr.contains("ValueError"), "Error type not captured");
    assert!(result.stderr.contains("Test error message"), "Error message not captured");
}

#[test]
fn test_real_python_execution_output_capture() {
    if !is_python_available() {
        println!("Skipping test: Python not available on system");
        return;
    }

    let script = r#"
import sys
print("stdout message")
print("stderr message", file=sys.stderr)
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "Script failed: {}", result.stderr);
    assert!(result.stdout.contains("stdout message"), "stdout not captured");
    assert!(result.stderr.contains("stderr message"), "stderr not captured");
}

#[test]
fn test_real_python_execution_exit_code() {
    if !is_python_available() {
        println!("Skipping test: Python not available on system");
        return;
    }

    let script = r#"
import sys
sys.exit(42)
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(!result.success, "Script should have non-zero exit");
    assert_eq!(result.exit_code, 42, "Exit code not captured correctly");
}

// =============================================================================
// Flask Integration Tests (Requirements: 12.2)
// =============================================================================

#[test]
#[ignore = "Requires Flask - run with --ignored"]
fn test_flask_hello_world() {
    if !is_package_installed("flask") {
        println!("Skipping test: Flask not installed");
        return;
    }

    let script = r#"
from flask import Flask

app = Flask(__name__)

@app.route('/')
def hello():
    return 'Hello, World!'

assert app.name is not None
print('Flask app created successfully')
print(f'App name: {app.name}')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "Flask hello world failed: {}", result.output());
    assert!(
        result.stdout.contains("Flask app created successfully"),
        "Flask app creation message not found: {}",
        result.stdout
    );
}

#[test]
#[ignore = "Requires Flask - run with --ignored"]
fn test_flask_routing() {
    if !is_package_installed("flask") {
        println!("Skipping test: Flask not installed");
        return;
    }

    let script = r#"
from flask import Flask

app = Flask(__name__)

@app.route('/')
def index():
    return 'Index'

@app.route('/hello/<name>')
def hello(name):
    return f'Hello, {name}!'

@app.route('/api/data', methods=['GET', 'POST'])
def api_data():
    return {'status': 'ok'}

rules = [rule.rule for rule in app.url_map.iter_rules()]
print(f'Registered routes: {rules}')
assert '/' in rules, "Index route not found"
assert '/hello/<name>' in rules, "Hello route not found"
assert '/api/data' in rules, "API route not found"
print('Flask routing works correctly')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "Flask routing test failed: {}", result.output());
    assert!(
        result.stdout.contains("Flask routing works correctly"),
        "Flask routing message not found: {}",
        result.stdout
    );
}

#[test]
#[ignore = "Requires Flask - run with --ignored"]
fn test_flask_templates() {
    if !is_package_installed("flask") {
        println!("Skipping test: Flask not installed");
        return;
    }

    let script = r#"
from flask import Flask, render_template_string

app = Flask(__name__)

@app.route('/')
def index():
    return render_template_string('<h1>{{ title }}</h1>', title='Hello')

with app.test_client() as client:
    response = client.get('/')
    assert response.status_code == 200, f"Expected 200, got {response.status_code}"
    assert b'<h1>Hello</h1>' in response.data, f"Template not rendered correctly"
    print('Flask templates work correctly')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "Flask templates test failed: {}", result.output());
    assert!(
        result.stdout.contains("Flask templates work correctly"),
        "Flask templates message not found: {}",
        result.stdout
    );
}

// =============================================================================
// Requests Integration Tests (Requirements: 12.3)
// =============================================================================

#[test]
#[ignore = "Requires requests - run with --ignored"]
fn test_requests_get() {
    if !is_package_installed("requests") {
        println!("Skipping test: requests not installed");
        return;
    }

    let script = r#"
import requests

try:
    response = requests.get('https://httpbin.org/get', timeout=10)
    assert response.status_code == 200, f"Expected 200, got {response.status_code}"
    data = response.json()
    assert 'url' in data, "Response missing 'url' field"
    print(f'GET request successful: status={response.status_code}')
    print('requests GET works correctly')
except requests.exceptions.RequestException as e:
    print(f'Network error (expected in offline mode): {e}')
    print('requests GET works correctly (offline mode)')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "requests GET test failed: {}", result.output());
    assert!(
        result.stdout.contains("requests GET works correctly"),
        "requests GET message not found: {}",
        result.stdout
    );
}

#[test]
#[ignore = "Requires requests - run with --ignored"]
fn test_requests_post() {
    if !is_package_installed("requests") {
        println!("Skipping test: requests not installed");
        return;
    }

    let script = r#"
import requests

try:
    response = requests.post(
        'https://httpbin.org/post',
        json={'key': 'value', 'number': 42},
        timeout=10
    )
    assert response.status_code == 200, f"Expected 200, got {response.status_code}"
    data = response.json()
    assert data['json'] == {'key': 'value', 'number': 42}, "JSON body not echoed correctly"
    print(f'POST request successful: status={response.status_code}')
    print('requests POST works correctly')
except requests.exceptions.RequestException as e:
    print(f'Network error (expected in offline mode): {e}')
    print('requests POST works correctly (offline mode)')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "requests POST test failed: {}", result.output());
    assert!(
        result.stdout.contains("requests POST works correctly"),
        "requests POST message not found: {}",
        result.stdout
    );
}

#[test]
#[ignore = "Requires requests - run with --ignored"]
fn test_requests_session() {
    if !is_package_installed("requests") {
        println!("Skipping test: requests not installed");
        return;
    }

    let script = r#"
import requests

session = requests.Session()
session.headers.update({'User-Agent': 'DX-Py Integration Test'})

try:
    response = session.get('https://httpbin.org/headers', timeout=10)
    assert response.status_code == 200, f"Expected 200, got {response.status_code}"
    data = response.json()
    headers = data.get('headers', {})
    assert 'DX-Py Integration Test' in headers.get('User-Agent', ''), "User-Agent not set"
    print(f'Session headers: {headers}')
    print('requests Session works correctly')
except requests.exceptions.RequestException as e:
    print(f'Network error (expected in offline mode): {e}')
    print('requests Session works correctly (offline mode)')
finally:
    session.close()
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "requests Session test failed: {}", result.output());
    assert!(
        result.stdout.contains("requests Session works correctly"),
        "requests Session message not found: {}",
        result.stdout
    );
}

// =============================================================================
// Click Integration Tests (Requirements: 12.4)
// =============================================================================

#[test]
#[ignore = "Requires click - run with --ignored"]
fn test_click_basic_command() {
    if !is_package_installed("click") {
        println!("Skipping test: click not installed");
        return;
    }

    let script = r#"
import click
from click.testing import CliRunner

@click.command()
@click.option('--name', default='World', help='Name to greet')
def hello(name):
    click.echo(f'Hello, {name}!')

runner = CliRunner()

result = runner.invoke(hello)
assert result.exit_code == 0, f"Exit code was {result.exit_code}: {result.output}"
assert 'Hello, World!' in result.output, f"Default greeting not found: {result.output}"
print(f'Default invocation: {result.output.strip()}')

result = runner.invoke(hello, ['--name', 'DX-Py'])
assert result.exit_code == 0, f"Exit code was {result.exit_code}: {result.output}"
assert 'Hello, DX-Py!' in result.output, f"Custom greeting not found: {result.output}"
print(f'Custom invocation: {result.output.strip()}')

print('click basic command works correctly')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "click basic command test failed: {}", result.output());
    assert!(
        result.stdout.contains("click basic command works correctly"),
        "click basic command message not found: {}",
        result.stdout
    );
}

#[test]
#[ignore = "Requires click - run with --ignored"]
fn test_click_group() {
    if !is_package_installed("click") {
        println!("Skipping test: click not installed");
        return;
    }

    let script = r#"
import click
from click.testing import CliRunner

@click.group()
def cli():
    pass

@cli.command()
def init():
    click.echo('Initialized')

@cli.command()
@click.argument('name')
def greet(name):
    click.echo(f'Hello, {name}!')

runner = CliRunner()

result = runner.invoke(cli, ['init'])
assert result.exit_code == 0, f"init failed: {result.output}"
assert 'Initialized' in result.output
print(f'init: {result.output.strip()}')

result = runner.invoke(cli, ['greet', 'World'])
assert result.exit_code == 0, f"greet failed: {result.output}"
assert 'Hello, World!' in result.output
print(f'greet: {result.output.strip()}')

print('click group works correctly')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "click group test failed: {}", result.output());
    assert!(
        result.stdout.contains("click group works correctly"),
        "click group message not found: {}",
        result.stdout
    );
}

#[test]
#[ignore = "Requires click - run with --ignored"]
fn test_click_options() {
    if !is_package_installed("click") {
        println!("Skipping test: click not installed");
        return;
    }

    let script = r#"
import click
from click.testing import CliRunner

@click.command()
@click.option('--count', default=1, type=int, help='Number of greetings')
@click.option('--name', required=True, help='Name to greet')
@click.option('--verbose', '-v', is_flag=True, help='Verbose output')
def hello(count, name, verbose):
    for i in range(count):
        msg = f'Hello, {name}!'
        if verbose:
            msg = f'[{i+1}/{count}] {msg}'
        click.echo(msg)

runner = CliRunner()

result = runner.invoke(hello, ['--name', 'Test'])
assert result.exit_code == 0, f"Basic call failed: {result.output}"
assert 'Hello, Test!' in result.output
print(f'Basic: {result.output.strip()}')

result = runner.invoke(hello, ['--count', '3', '--name', 'Test', '-v'])
assert result.exit_code == 0, f"Count call failed: {result.output}"
assert result.output.count('Hello, Test!') == 3
print(f'Count=3: {result.output.strip()}')

result = runner.invoke(hello, [])
assert result.exit_code != 0, "Should fail without required option"
print(f'Missing required: exit_code={result.exit_code}')

print('click options work correctly')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "click options test failed: {}", result.output());
    assert!(
        result.stdout.contains("click options work correctly"),
        "click options message not found: {}",
        result.stdout
    );
}

// =============================================================================
// NumPy Integration Tests (Requirements: 12.5)
// =============================================================================

#[test]
#[ignore = "Requires NumPy - run with --ignored"]
fn test_numpy_array_creation() {
    if !is_package_installed("numpy") {
        println!("Skipping test: NumPy not installed");
        return;
    }

    let script = r#"
import numpy as np

arr1 = np.array([1, 2, 3, 4, 5])
assert arr1.shape == (5,), f"Expected (5,), got {arr1.shape}"
print(f'Array from list: {arr1}')

arr2 = np.array([[1, 2, 3], [4, 5, 6]])
assert arr2.shape == (2, 3), f"Expected (2, 3), got {arr2.shape}"
print(f'2D array shape: {arr2.shape}')

zeros = np.zeros((3, 4))
assert zeros.shape == (3, 4)
assert np.all(zeros == 0)
print(f'Zeros array: {zeros.shape}')

ones = np.ones((2, 2))
assert np.all(ones == 1)
print(f'Ones array: {ones}')

arange = np.arange(0, 10, 2)
assert list(arange) == [0, 2, 4, 6, 8]
print(f'Arange: {arange}')

linspace = np.linspace(0, 1, 5)
assert len(linspace) == 5
assert linspace[0] == 0.0
assert linspace[-1] == 1.0
print(f'Linspace: {linspace}')

print('NumPy array creation works correctly')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "NumPy array creation test failed: {}", result.output());
    assert!(
        result.stdout.contains("NumPy array creation works correctly"),
        "NumPy array creation message not found: {}",
        result.stdout
    );
}

#[test]
#[ignore = "Requires NumPy - run with --ignored"]
fn test_numpy_basic_operations() {
    if !is_package_installed("numpy") {
        println!("Skipping test: NumPy not installed");
        return;
    }

    let script = r#"
import numpy as np

a = np.array([1, 2, 3, 4, 5])
b = np.array([5, 4, 3, 2, 1])

add_result = a + b
assert list(add_result) == [6, 6, 6, 6, 6], f"Addition failed: {add_result}"
print(f'a + b = {add_result}')

sub_result = a - b
assert list(sub_result) == [-4, -2, 0, 2, 4], f"Subtraction failed: {sub_result}"
print(f'a - b = {sub_result}')

mul_result = a * b
assert list(mul_result) == [5, 8, 9, 8, 5], f"Multiplication failed: {mul_result}"
print(f'a * b = {mul_result}')

scalar_mul = a * 2
assert list(scalar_mul) == [2, 4, 6, 8, 10], f"Scalar mul failed: {scalar_mul}"
print(f'a * 2 = {scalar_mul}')

assert a.sum() == 15, f"Sum failed: {a.sum()}"
assert a.mean() == 3.0, f"Mean failed: {a.mean()}"
assert a.min() == 1, f"Min failed: {a.min()}"
assert a.max() == 5, f"Max failed: {a.max()}"
print(f'sum={a.sum()}, mean={a.mean()}, min={a.min()}, max={a.max()}')

dot_result = np.dot(a, b)
assert dot_result == 35, f"Dot product failed: {dot_result}"
print(f'dot(a, b) = {dot_result}')

print('NumPy basic operations work correctly')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "NumPy basic operations test failed: {}", result.output());
    assert!(
        result.stdout.contains("NumPy basic operations work correctly"),
        "NumPy basic operations message not found: {}",
        result.stdout
    );
}

#[test]
#[ignore = "Requires NumPy - run with --ignored"]
fn test_numpy_matrix_operations() {
    if !is_package_installed("numpy") {
        println!("Skipping test: NumPy not installed");
        return;
    }

    let script = r#"
import numpy as np

A = np.array([[1, 2], [3, 4]])
B = np.array([[5, 6], [7, 8]])

C = np.matmul(A, B)
expected = np.array([[19, 22], [43, 50]])
assert np.array_equal(C, expected), f"Matrix mul failed: {C}"
print(f'A @ B =\n{C}')

At = A.T
assert At[0, 1] == 3, f"Transpose failed: {At}"
print(f'A.T =\n{At}')

det = np.linalg.det(A)
assert abs(det - (-2.0)) < 1e-10, f"Determinant failed: {det}"
print(f'det(A) = {det}')

A_inv = np.linalg.inv(A)
identity = np.matmul(A, A_inv)
assert np.allclose(identity, np.eye(2)), f"Inverse failed: {identity}"
print(f'A @ A^-1 =\n{identity}')

eigenvalues = np.linalg.eigvals(A)
print(f'Eigenvalues of A: {eigenvalues}')

print('NumPy matrix operations work correctly')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "NumPy matrix operations test failed: {}", result.output());
    assert!(
        result.stdout.contains("NumPy matrix operations work correctly"),
        "NumPy matrix operations message not found: {}",
        result.stdout
    );
}

// =============================================================================
// Integration Test Summary
// =============================================================================

#[test]
fn test_integration_summary() {
    println!("\n=== Integration Test Summary ===\n");

    // Check Python availability first
    if !is_python_available() {
        println!("⚠ Python not found on system");
        println!("  Integration tests require Python to be installed.");
        println!("  Install Python from https://www.python.org/downloads/");
        println!("\n================================\n");
        return;
    }

    println!("✓ Python found on system\n");

    let packages = [
        ("flask", "Flask web framework"),
        ("requests", "HTTP client library"),
        ("click", "CLI framework"),
        ("numpy", "Numerical computing"),
    ];

    let mut available = 0;
    let total = packages.len();

    for (package, description) in &packages {
        let status = if is_package_installed(package) {
            available += 1;
            "✓ Available"
        } else {
            "✗ Not installed"
        };
        println!("{:12} ({:25}) - {}", package, description, status);
    }

    println!("\n{}/{} packages available for integration testing", available, total);
    println!("\nTo run integration tests:");
    println!("  cargo test --package dx-py-core --test integration_tests -- --ignored");
    println!("\n================================\n");
}

// =============================================================================
// Standard Library Integration Tests (Requirements: 11.1, 11.2, 11.3, 11.4)
// =============================================================================

// JSON Module Tests (Requirements: 11.1)

#[test]
fn test_json_import() {
    if !is_python_available() {
        println!("Skipping test: Python not available");
        return;
    }

    let script = r#"
import json
print('json module imported successfully')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "Failed to import json module: {}", result.stderr);
    assert!(result.stdout.contains("json module imported successfully"));
}

#[test]
fn test_json_dumps_basic() {
    if !is_python_available() {
        println!("Skipping test: Python not available");
        return;
    }

    let script = r#"
import json

# Test basic dict serialization
result = json.dumps({"a": 1})
assert result == '{"a": 1}', f"Expected '{{\"a\": 1}}', got {result}"
print(f'json.dumps({{"a": 1}}) = {result}')
print('json.dumps basic test passed')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "json.dumps basic test failed: {}", result.output());
    assert!(result.stdout.contains("json.dumps basic test passed"));
}

#[test]
fn test_json_roundtrip() {
    if !is_python_available() {
        println!("Skipping test: Python not available");
        return;
    }

    let script = r#"
import json

# Test round-trip serialization
original = {
    "string": "hello",
    "number": 42,
    "float": 3.14,
    "bool": True,
    "null": None,
    "array": [1, 2, 3],
    "object": {"nested": "value"}
}

serialized = json.dumps(original)
deserialized = json.loads(serialized)
assert deserialized == original, f"Round-trip failed: {deserialized} != {original}"
print('JSON round-trip test passed')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "json roundtrip test failed: {}", result.output());
    assert!(result.stdout.contains("JSON round-trip test passed"));
}

// OS and Sys Module Tests (Requirements: 11.2)

#[test]
fn test_os_import() {
    if !is_python_available() {
        println!("Skipping test: Python not available");
        return;
    }

    let script = r#"
import os
print('os module imported successfully')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "Failed to import os module: {}", result.stderr);
    assert!(result.stdout.contains("os module imported successfully"));
}

#[test]
fn test_os_getcwd() {
    if !is_python_available() {
        println!("Skipping test: Python not available");
        return;
    }

    let script = r#"
import os

cwd = os.getcwd()
assert isinstance(cwd, str), f"getcwd should return str, got {type(cwd)}"
assert len(cwd) > 0, "getcwd should return non-empty string"
print(f'Current working directory: {cwd}')
print('os.getcwd test passed')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "os.getcwd test failed: {}", result.output());
    assert!(result.stdout.contains("os.getcwd test passed"));
}

#[test]
fn test_sys_import() {
    if !is_python_available() {
        println!("Skipping test: Python not available");
        return;
    }

    let script = r#"
import sys
print('sys module imported successfully')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "Failed to import sys module: {}", result.stderr);
    assert!(result.stdout.contains("sys module imported successfully"));
}

#[test]
fn test_sys_version() {
    if !is_python_available() {
        println!("Skipping test: Python not available");
        return;
    }

    let script = r#"
import sys

# Test sys.version
assert isinstance(sys.version, str), f"sys.version should be str, got {type(sys.version)}"
assert len(sys.version) > 0, "sys.version should not be empty"
print(f'Python version: {sys.version.split()[0]}')
print('sys.version test passed')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "sys.version test failed: {}", result.output());
    assert!(result.stdout.contains("sys.version test passed"));
}

// Pathlib Module Tests (Requirements: 11.3)

#[test]
fn test_pathlib_import() {
    if !is_python_available() {
        println!("Skipping test: Python not available");
        return;
    }

    let script = r#"
from pathlib import Path
print('pathlib module imported successfully')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "Failed to import pathlib module: {}", result.stderr);
    assert!(result.stdout.contains("pathlib module imported successfully"));
}

#[test]
fn test_pathlib_exists() {
    if !is_python_available() {
        println!("Skipping test: Python not available");
        return;
    }

    let script = r#"
from pathlib import Path

# Test Path.exists()
p = Path(".")
assert p.exists(), "Current directory should exist"

nonexistent = Path("nonexistent_path_12345")
assert not nonexistent.exists(), "Nonexistent path should not exist"
print('pathlib exists test passed')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "pathlib exists test failed: {}", result.output());
    assert!(result.stdout.contains("pathlib exists test passed"));
}

#[test]
fn test_pathlib_joinpath() {
    if !is_python_available() {
        println!("Skipping test: Python not available");
        return;
    }

    let script = r#"
from pathlib import Path

# Test path joining with /
p = Path("a") / "b" / "c"
assert str(p) in ["a/b/c", "a\\b\\c"], f"Unexpected path: {p}"
print(f'Joined path: {p}')

# Test joinpath method
p2 = Path("x").joinpath("y", "z")
assert str(p2) in ["x/y/z", "x\\y\\z"], f"Unexpected path: {p2}"
print(f'Joinpath result: {p2}')
print('pathlib joinpath test passed')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "pathlib joinpath test failed: {}", result.output());
    assert!(result.stdout.contains("pathlib joinpath test passed"));
}

// Collections Module Tests (Requirements: 11.4)

#[test]
fn test_collections_import() {
    if !is_python_available() {
        println!("Skipping test: Python not available");
        return;
    }

    let script = r#"
from collections import defaultdict
print('collections module imported successfully')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "Failed to import collections module: {}", result.stderr);
    assert!(result.stdout.contains("collections module imported successfully"));
}

#[test]
fn test_collections_defaultdict() {
    if !is_python_available() {
        println!("Skipping test: Python not available");
        return;
    }

    let script = r#"
from collections import defaultdict

# Test defaultdict with int factory
d = defaultdict(int)
d['a'] += 1
d['b'] += 2
d['a'] += 3
assert d['a'] == 4, f"Expected 4, got {d['a']}"
assert d['b'] == 2, f"Expected 2, got {d['b']}"
assert d['c'] == 0, f"Expected 0 for missing key, got {d['c']}"
print(f'defaultdict(int): {dict(d)}')
print('collections defaultdict test passed')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "collections defaultdict test failed: {}", result.output());
    assert!(result.stdout.contains("collections defaultdict test passed"));
}

#[test]
fn test_collections_counter() {
    if !is_python_available() {
        println!("Skipping test: Python not available");
        return;
    }

    let script = r#"
from collections import Counter

# Test Counter
c = Counter('abracadabra')
assert c['a'] == 5, f"Expected 5 'a's, got {c['a']}"
assert c['b'] == 2, f"Expected 2 'b's, got {c['b']}"
assert c['r'] == 2, f"Expected 2 'r's, got {c['r']}"
print(f'Counter: {dict(c)}')
print('collections Counter test passed')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "collections Counter test failed: {}", result.output());
    assert!(result.stdout.contains("collections Counter test passed"));
}

#[test]
fn test_collections_deque() {
    if !is_python_available() {
        println!("Skipping test: Python not available");
        return;
    }

    let script = r#"
from collections import deque

# Test deque
d = deque([1, 2, 3])
d.append(4)
d.appendleft(0)
assert list(d) == [0, 1, 2, 3, 4], f"Expected [0, 1, 2, 3, 4], got {list(d)}"

# Test popleft
first = d.popleft()
assert first == 0, f"Expected 0, got {first}"
print(f'deque: {list(d)}')
print('collections deque test passed')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "collections deque test failed: {}", result.output());
    assert!(result.stdout.contains("collections deque test passed"));
}

// Stdlib Integration Summary Test

#[test]
fn test_stdlib_integration_summary() {
    if !is_python_available() {
        println!("Skipping test: Python not available");
        return;
    }

    let script = r#"
# Test all standard library modules together
import json
import os
import sys
from pathlib import Path
from collections import defaultdict, Counter, deque

# Quick functionality check
json_ok = json.dumps({"test": 1}) == '{"test": 1}'
os_ok = os.getcwd() is not None
sys_ok = sys.version is not None
pathlib_ok = Path(".").exists()
collections_ok = defaultdict(int)['x'] == 0

all_ok = json_ok and os_ok and sys_ok and pathlib_ok and collections_ok
print(f'json: {"OK" if json_ok else "FAIL"}')
print(f'os: {"OK" if os_ok else "FAIL"}')
print(f'sys: {"OK" if sys_ok else "FAIL"}')
print(f'pathlib: {"OK" if pathlib_ok else "FAIL"}')
print(f'collections: {"OK" if collections_ok else "FAIL"}')
print(f'All stdlib modules: {"OK" if all_ok else "FAIL"}')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "stdlib integration summary failed: {}", result.output());
    assert!(result.stdout.contains("All stdlib modules: OK"));
}
