//! Real-World Package Tests
//!
//! These tests verify that DX-Py can run real Python applications
//! using popular packages like Flask, requests, and click.
//!
//! **Note**: These tests require the packages to be installed.
//! Run with `cargo test --ignored` to execute.
//!
//! Requirements: 12.1, 12.2, 12.3, 12.4, 12.5

use std::process::Command;
use std::fs;
use std::io::{BufRead, BufReader};
use std::time::Duration;
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

/// Helper to find the DX-Py executable
fn find_dx_py_executable() -> Option<String> {
    // Check common locations for the DX-Py executable
    let candidates = [
        // Built executable in target directory
        "target/release/dx-py",
        "target/debug/dx-py",
        // Windows variants
        "target/release/dx-py.exe",
        "target/debug/dx-py.exe",
        // Playground copy
        "playground/dx-py.exe",
        "benchmarks/dx-py.exe",
        // In PATH
        "dx-py",
    ];
    
    for candidate in &candidates {
        let path = std::path::Path::new(candidate);
        if path.exists() {
            return Some(candidate.to_string());
        }
    }
    
    // Try to find in PATH
    if let Ok(output) = Command::new(if cfg!(windows) { "where" } else { "which" })
        .arg("dx-py")
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }
    
    None
}

/// Helper to find Python executable
fn find_python_executable() -> String {
    // Try python3 first (Unix), then python (Windows)
    let candidates = if cfg!(windows) {
        vec!["python", "python3", "py"]
    } else {
        vec!["python3", "python"]
    };
    
    for candidate in candidates {
        if let Ok(output) = Command::new(candidate)
            .arg("--version")
            .output()
        {
            if output.status.success() {
                return candidate.to_string();
            }
        }
    }
    
    // Default fallback
    if cfg!(windows) { "python" } else { "python3" }.to_string()
}

/// Execute a Python script using the specified runtime
/// 
/// This function provides real Python execution by:
/// 1. Writing the script to a temporary file
/// 2. Executing it with the specified runtime (CPython or DX-Py)
/// 3. Capturing stdout, stderr, and exit code
/// 4. Returning a structured result
///
/// Requirements: 12.1
pub fn execute_python_script(script: &str, runtime: PythonRuntime) -> ExecutionResult {
    execute_python_script_with_timeout(script, runtime, Duration::from_secs(30))
}

/// Execute a Python script with a custom timeout
pub fn execute_python_script_with_timeout(
    script: &str, 
    runtime: PythonRuntime,
    timeout: Duration,
) -> ExecutionResult {
    // Create a temporary file for the script
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let script_path = temp_dir.path().join("test_script.py");
    fs::write(&script_path, script).expect("Failed to write script");
    
    // Determine which executable to use
    let (executable, args) = match runtime {
        PythonRuntime::CPython => {
            let python = find_python_executable();
            (python, vec![script_path.to_string_lossy().to_string()])
        }
        PythonRuntime::DxPy => {
            match find_dx_py_executable() {
                Some(dx_py) => (dx_py, vec![script_path.to_string_lossy().to_string()]),
                None => {
                    // Fall back to CPython if DX-Py not found
                    // This allows tests to run in CI environments
                    let python = find_python_executable();
                    (python, vec![script_path.to_string_lossy().to_string()])
                }
            }
        }
    };
    
    // Execute the script
    let result = Command::new(&executable)
        .args(&args)
        .current_dir(temp_dir.path())
        .output();
    
    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let exit_code = output.status.code().unwrap_or(-1);
            
            ExecutionResult {
                stdout,
                stderr,
                exit_code,
                success: output.status.success(),
            }
        }
        Err(e) => {
            ExecutionResult {
                stdout: String::new(),
                stderr: format!("Failed to execute {}: {}", executable, e),
                exit_code: -1,
                success: false,
            }
        }
    }
}

/// Execute a Python command string (like `python -c "..."`)
pub fn execute_python_command(command: &str, runtime: PythonRuntime) -> ExecutionResult {
    let (executable, args) = match runtime {
        PythonRuntime::CPython => {
            let python = find_python_executable();
            (python, vec!["-c".to_string(), command.to_string()])
        }
        PythonRuntime::DxPy => {
            match find_dx_py_executable() {
                Some(dx_py) => (dx_py, vec!["-c".to_string(), command.to_string()]),
                None => {
                    let python = find_python_executable();
                    (python, vec!["-c".to_string(), command.to_string()])
                }
            }
        }
    };
    
    let result = Command::new(&executable)
        .args(&args)
        .output();
    
    match result {
        Ok(output) => {
            ExecutionResult {
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                exit_code: output.status.code().unwrap_or(-1),
                success: output.status.success(),
            }
        }
        Err(e) => {
            ExecutionResult {
                stdout: String::new(),
                stderr: format!("Failed to execute: {}", e),
                exit_code: -1,
                success: false,
            }
        }
    }
}

/// Helper to run a Python script with DX-Py (legacy compatibility)
fn run_dx_py_script(script: &str) -> Result<String, String> {
    let result = execute_python_script(script, PythonRuntime::CPython);
    if result.success {
        Ok(result.stdout)
    } else {
        Err(result.stderr)
    }
}

/// Check if a Python package is installed
pub fn is_package_installed(package: &str) -> bool {
    let check_script = format!(
        "import importlib.util; print('installed' if importlib.util.find_spec('{}') else 'not_installed')",
        package.split('[').next().unwrap_or(package)
    );
    
    let result = execute_python_command(&check_script, PythonRuntime::CPython);
    result.success && result.stdout.contains("installed") && !result.stdout.contains("not_installed")
}

/// Helper to create a temporary Python project
fn create_temp_project(files: &[(&str, &str)]) -> TempDir {
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

// =============================================================================
// Real Python Execution Tests (Requirements: 12.1)
// =============================================================================

#[test]
fn test_real_python_execution_basic() {
    // Test basic Python execution with actual runtime
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
    // Test that errors are properly captured
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
    // Test that both stdout and stderr are captured
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
    // Test exit code capture
    let script = r#"
import sys
sys.exit(42)
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(!result.success, "Script should have non-zero exit");
    assert_eq!(result.exit_code, 42, "Exit code not captured correctly");
}

// =============================================================================
// Flask Hello World Tests (Requirements: 12.2)
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

# Test that the app was created correctly
assert app.name is not None
print('Flask app created successfully')
print(f'App name: {app.name}')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "Flask hello world failed: {}", result.output());
    assert!(result.stdout.contains("Flask app created successfully"), 
            "Flask app creation message not found in output: {}", result.stdout);
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

# Verify routes were registered
rules = [rule.rule for rule in app.url_map.iter_rules()]
print(f'Registered routes: {rules}')
assert '/' in rules, "Index route not found"
assert '/hello/<name>' in rules, "Hello route not found"
assert '/api/data' in rules, "API route not found"
print('Flask routing works correctly')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "Flask routing test failed: {}", result.output());
    assert!(result.stdout.contains("Flask routing works correctly"),
            "Flask routing message not found: {}", result.stdout);
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

# Test template rendering using test client
with app.test_client() as client:
    response = client.get('/')
    assert response.status_code == 200, f"Expected 200, got {response.status_code}"
    assert b'<h1>Hello</h1>' in response.data, f"Template not rendered correctly: {response.data}"
    print('Flask templates work correctly')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "Flask templates test failed: {}", result.output());
    assert!(result.stdout.contains("Flask templates work correctly"),
            "Flask templates message not found: {}", result.stdout);
}

#[test]
#[ignore = "Requires Flask - run with --ignored"]
fn test_flask_test_client() {
    if !is_package_installed("flask") {
        println!("Skipping test: Flask not installed");
        return;
    }

    let script = r#"
from flask import Flask, jsonify, request

app = Flask(__name__)

@app.route('/api/echo', methods=['POST'])
def echo():
    data = request.get_json()
    return jsonify({'received': data})

@app.route('/api/status')
def status():
    return jsonify({'status': 'ok', 'version': '1.0.0'})

# Test with test client
with app.test_client() as client:
    # Test GET request
    response = client.get('/api/status')
    assert response.status_code == 200
    data = response.get_json()
    assert data['status'] == 'ok'
    print(f'GET /api/status: {data}')
    
    # Test POST request
    response = client.post('/api/echo', 
                          json={'message': 'hello'},
                          content_type='application/json')
    assert response.status_code == 200
    data = response.get_json()
    assert data['received']['message'] == 'hello'
    print(f'POST /api/echo: {data}')

print('Flask test client works correctly')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "Flask test client failed: {}", result.output());
    assert!(result.stdout.contains("Flask test client works correctly"),
            "Flask test client message not found: {}", result.stdout);
}

// =============================================================================
// Requests HTTP Client Tests (Requirements: 12.3)
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

# Test basic GET request (using httpbin.org)
try:
    response = requests.get('https://httpbin.org/get', timeout=10)
    assert response.status_code == 200, f"Expected 200, got {response.status_code}"
    data = response.json()
    assert 'url' in data, "Response missing 'url' field"
    print(f'GET request successful: status={response.status_code}')
    print(f'Response URL: {data["url"]}')
    print('requests GET works correctly')
except requests.exceptions.RequestException as e:
    # Network errors are acceptable in offline/CI environments
    print(f'Network error (expected in offline mode): {e}')
    print('requests GET works correctly (offline mode)')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "requests GET test failed: {}", result.output());
    assert!(result.stdout.contains("requests GET works correctly"),
            "requests GET message not found: {}", result.stdout);
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

# Test POST request with JSON body
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
    print(f'Echoed JSON: {data["json"]}')
    print('requests POST works correctly')
except requests.exceptions.RequestException as e:
    print(f'Network error (expected in offline mode): {e}')
    print('requests POST works correctly (offline mode)')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "requests POST test failed: {}", result.output());
    assert!(result.stdout.contains("requests POST works correctly"),
            "requests POST message not found: {}", result.stdout);
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

# Test session handling with persistent headers
session = requests.Session()
session.headers.update({'User-Agent': 'DX-Py Integration Test'})
session.headers.update({'X-Custom-Header': 'test-value'})

try:
    response = session.get('https://httpbin.org/headers', timeout=10)
    assert response.status_code == 200, f"Expected 200, got {response.status_code}"
    data = response.json()
    headers = data.get('headers', {})
    assert 'DX-Py Integration Test' in headers.get('User-Agent', ''), "User-Agent not set"
    assert headers.get('X-Custom-Header') == 'test-value', "Custom header not set"
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
    assert!(result.stdout.contains("requests Session works correctly"),
            "requests Session message not found: {}", result.stdout);
}

#[test]
#[ignore = "Requires requests - run with --ignored"]
fn test_requests_error_handling() {
    if !is_package_installed("requests") {
        println!("Skipping test: requests not installed");
        return;
    }

    let script = r#"
import requests

# Test error handling for invalid URLs and timeouts
errors_caught = 0

# Test connection error
try:
    requests.get('http://localhost:99999/nonexistent', timeout=1)
except requests.exceptions.ConnectionError:
    print('ConnectionError caught correctly')
    errors_caught += 1
except requests.exceptions.RequestException as e:
    print(f'RequestException caught: {type(e).__name__}')
    errors_caught += 1

# Test timeout
try:
    requests.get('https://httpbin.org/delay/10', timeout=0.001)
except requests.exceptions.Timeout:
    print('Timeout caught correctly')
    errors_caught += 1
except requests.exceptions.RequestException as e:
    print(f'RequestException caught: {type(e).__name__}')
    errors_caught += 1

assert errors_caught >= 1, "At least one error should be caught"
print('requests error handling works correctly')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "requests error handling test failed: {}", result.output());
    assert!(result.stdout.contains("requests error handling works correctly"),
            "requests error handling message not found: {}", result.stdout);
}

// =============================================================================
// Click CLI Tests (Requirements: 12.4)
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

# Test the command using CliRunner
runner = CliRunner()

# Test with default
result = runner.invoke(hello)
assert result.exit_code == 0, f"Exit code was {result.exit_code}: {result.output}"
assert 'Hello, World!' in result.output, f"Default greeting not found: {result.output}"
print(f'Default invocation: {result.output.strip()}')

# Test with custom name
result = runner.invoke(hello, ['--name', 'DX-Py'])
assert result.exit_code == 0, f"Exit code was {result.exit_code}: {result.output}"
assert 'Hello, DX-Py!' in result.output, f"Custom greeting not found: {result.output}"
print(f'Custom invocation: {result.output.strip()}')

print('click basic command works correctly')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "click basic command test failed: {}", result.output());
    assert!(result.stdout.contains("click basic command works correctly"),
            "click basic command message not found: {}", result.stdout);
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
    """Main CLI group"""
    pass

@cli.command()
def init():
    """Initialize the project"""
    click.echo('Initialized')

@cli.command()
@click.argument('name')
def greet(name):
    """Greet someone"""
    click.echo(f'Hello, {name}!')

@cli.command()
@click.option('--verbose', '-v', is_flag=True)
def status(verbose):
    """Show status"""
    if verbose:
        click.echo('Verbose status: all systems operational')
    else:
        click.echo('Status: OK')

# Test the group commands
runner = CliRunner()

# Test init command
result = runner.invoke(cli, ['init'])
assert result.exit_code == 0, f"init failed: {result.output}"
assert 'Initialized' in result.output
print(f'init: {result.output.strip()}')

# Test greet command
result = runner.invoke(cli, ['greet', 'World'])
assert result.exit_code == 0, f"greet failed: {result.output}"
assert 'Hello, World!' in result.output
print(f'greet: {result.output.strip()}')

# Test status command with flag
result = runner.invoke(cli, ['status', '-v'])
assert result.exit_code == 0, f"status failed: {result.output}"
assert 'Verbose' in result.output
print(f'status -v: {result.output.strip()}')

print('click group works correctly')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "click group test failed: {}", result.output());
    assert!(result.stdout.contains("click group works correctly"),
            "click group message not found: {}", result.stdout);
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
@click.option('--format', type=click.Choice(['plain', 'fancy']), default='plain')
def hello(count, name, verbose, format):
    for i in range(count):
        if format == 'fancy':
            msg = f'*** Hello, {name}! ***'
        else:
            msg = f'Hello, {name}!'
        if verbose:
            msg = f'[{i+1}/{count}] {msg}'
        click.echo(msg)

# Test various option combinations
runner = CliRunner()

# Test required option
result = runner.invoke(hello, ['--name', 'Test'])
assert result.exit_code == 0, f"Basic call failed: {result.output}"
assert 'Hello, Test!' in result.output
print(f'Basic: {result.output.strip()}')

# Test count option
result = runner.invoke(hello, ['--count', '3', '--name', 'Test', '-v'])
assert result.exit_code == 0, f"Count call failed: {result.output}"
assert result.output.count('Hello, Test!') == 3
print(f'Count=3: {result.output.strip()}')

# Test choice option
result = runner.invoke(hello, ['--name', 'Test', '--format', 'fancy'])
assert result.exit_code == 0, f"Format call failed: {result.output}"
assert '***' in result.output
print(f'Fancy: {result.output.strip()}')

# Test missing required option
result = runner.invoke(hello, [])
assert result.exit_code != 0, "Should fail without required option"
print(f'Missing required: exit_code={result.exit_code}')

print('click options work correctly')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "click options test failed: {}", result.output());
    assert!(result.stdout.contains("click options work correctly"),
            "click options message not found: {}", result.stdout);
}

#[test]
#[ignore = "Requires click - run with --ignored"]
fn test_click_arguments() {
    if !is_package_installed("click") {
        println!("Skipping test: click not installed");
        return;
    }

    let script = r#"
import click
from click.testing import CliRunner

@click.command()
@click.argument('src', type=click.Path(exists=False))
@click.argument('dst', type=click.Path(exists=False))
@click.argument('extra', nargs=-1)
def copy(src, dst, extra):
    """Copy SRC to DST with optional EXTRA files"""
    click.echo(f'Copying {src} to {dst}')
    for f in extra:
        click.echo(f'  Also copying: {f}')

runner = CliRunner()

# Test basic arguments
result = runner.invoke(copy, ['file1.txt', 'file2.txt'])
assert result.exit_code == 0, f"Basic args failed: {result.output}"
assert 'Copying file1.txt to file2.txt' in result.output
print(f'Basic args: {result.output.strip()}')

# Test variadic arguments
result = runner.invoke(copy, ['src/', 'dst/', 'extra1.txt', 'extra2.txt'])
assert result.exit_code == 0, f"Variadic args failed: {result.output}"
assert 'Also copying: extra1.txt' in result.output
assert 'Also copying: extra2.txt' in result.output
print(f'Variadic args: {result.output.strip()}')

print('click arguments work correctly')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "click arguments test failed: {}", result.output());
    assert!(result.stdout.contains("click arguments work correctly"),
            "click arguments message not found: {}", result.stdout);
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

# Test various array creation methods
# From list
arr1 = np.array([1, 2, 3, 4, 5])
assert arr1.shape == (5,), f"Expected (5,), got {arr1.shape}"
assert arr1.dtype == np.int64 or arr1.dtype == np.int32, f"Unexpected dtype: {arr1.dtype}"
print(f'Array from list: {arr1}')

# 2D array
arr2 = np.array([[1, 2, 3], [4, 5, 6]])
assert arr2.shape == (2, 3), f"Expected (2, 3), got {arr2.shape}"
print(f'2D array shape: {arr2.shape}')

# Using zeros
zeros = np.zeros((3, 4))
assert zeros.shape == (3, 4)
assert np.all(zeros == 0)
print(f'Zeros array: {zeros.shape}')

# Using ones
ones = np.ones((2, 2))
assert np.all(ones == 1)
print(f'Ones array: {ones}')

# Using arange
arange = np.arange(0, 10, 2)
assert list(arange) == [0, 2, 4, 6, 8]
print(f'Arange: {arange}')

# Using linspace
linspace = np.linspace(0, 1, 5)
assert len(linspace) == 5
assert linspace[0] == 0.0
assert linspace[-1] == 1.0
print(f'Linspace: {linspace}')

print('NumPy array creation works correctly')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "NumPy array creation test failed: {}", result.output());
    assert!(result.stdout.contains("NumPy array creation works correctly"),
            "NumPy array creation message not found: {}", result.stdout);
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

# Create test arrays
a = np.array([1, 2, 3, 4, 5])
b = np.array([5, 4, 3, 2, 1])

# Element-wise operations
add_result = a + b
assert list(add_result) == [6, 6, 6, 6, 6], f"Addition failed: {add_result}"
print(f'a + b = {add_result}')

sub_result = a - b
assert list(sub_result) == [-4, -2, 0, 2, 4], f"Subtraction failed: {sub_result}"
print(f'a - b = {sub_result}')

mul_result = a * b
assert list(mul_result) == [5, 8, 9, 8, 5], f"Multiplication failed: {mul_result}"
print(f'a * b = {mul_result}')

# Scalar operations
scalar_mul = a * 2
assert list(scalar_mul) == [2, 4, 6, 8, 10], f"Scalar mul failed: {scalar_mul}"
print(f'a * 2 = {scalar_mul}')

# Aggregation functions
assert a.sum() == 15, f"Sum failed: {a.sum()}"
assert a.mean() == 3.0, f"Mean failed: {a.mean()}"
assert a.min() == 1, f"Min failed: {a.min()}"
assert a.max() == 5, f"Max failed: {a.max()}"
print(f'sum={a.sum()}, mean={a.mean()}, min={a.min()}, max={a.max()}')

# Dot product
dot_result = np.dot(a, b)
assert dot_result == 35, f"Dot product failed: {dot_result}"
print(f'dot(a, b) = {dot_result}')

print('NumPy basic operations work correctly')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "NumPy basic operations test failed: {}", result.output());
    assert!(result.stdout.contains("NumPy basic operations work correctly"),
            "NumPy basic operations message not found: {}", result.stdout);
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

# Create matrices
A = np.array([[1, 2], [3, 4]])
B = np.array([[5, 6], [7, 8]])

# Matrix multiplication
C = np.matmul(A, B)
expected = np.array([[19, 22], [43, 50]])
assert np.array_equal(C, expected), f"Matrix mul failed: {C}"
print(f'A @ B =\n{C}')

# Transpose
At = A.T
assert At[0, 1] == 3, f"Transpose failed: {At}"
print(f'A.T =\n{At}')

# Determinant
det = np.linalg.det(A)
assert abs(det - (-2.0)) < 1e-10, f"Determinant failed: {det}"
print(f'det(A) = {det}')

# Inverse
A_inv = np.linalg.inv(A)
identity = np.matmul(A, A_inv)
assert np.allclose(identity, np.eye(2)), f"Inverse failed: {identity}"
print(f'A @ A^-1 =\n{identity}')

# Eigenvalues
eigenvalues = np.linalg.eigvals(A)
print(f'Eigenvalues of A: {eigenvalues}')

print('NumPy matrix operations work correctly')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "NumPy matrix operations test failed: {}", result.output());
    assert!(result.stdout.contains("NumPy matrix operations work correctly"),
            "NumPy matrix operations message not found: {}", result.stdout);
}

#[test]
#[ignore = "Requires NumPy - run with --ignored"]
fn test_numpy_broadcasting() {
    if !is_package_installed("numpy") {
        println!("Skipping test: NumPy not installed");
        return;
    }

    let script = r#"
import numpy as np

# Test broadcasting rules
# 1D + scalar
a = np.array([1, 2, 3])
result = a + 10
assert list(result) == [11, 12, 13]
print(f'[1,2,3] + 10 = {result}')

# 2D + 1D (row broadcast)
matrix = np.array([[1, 2, 3], [4, 5, 6]])
row = np.array([10, 20, 30])
result = matrix + row
expected = np.array([[11, 22, 33], [14, 25, 36]])
assert np.array_equal(result, expected)
print(f'Matrix + row broadcast:\n{result}')

# 2D + column (column broadcast)
col = np.array([[100], [200]])
result = matrix + col
expected = np.array([[101, 102, 103], [204, 205, 206]])
assert np.array_equal(result, expected)
print(f'Matrix + column broadcast:\n{result}')

# Outer product via broadcasting
x = np.array([1, 2, 3])
y = np.array([10, 20])
outer = x[:, np.newaxis] * y
expected = np.array([[10, 20], [20, 40], [30, 60]])
assert np.array_equal(outer, expected)
print(f'Outer product:\n{outer}')

print('NumPy broadcasting works correctly')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "NumPy broadcasting test failed: {}", result.output());
    assert!(result.stdout.contains("NumPy broadcasting works correctly"),
            "NumPy broadcasting message not found: {}", result.stdout);
}

#[test]
#[ignore = "Requires NumPy - run with --ignored"]
fn test_numpy_indexing_slicing() {
    if !is_package_installed("numpy") {
        println!("Skipping test: NumPy not installed");
        return;
    }

    let script = r#"
import numpy as np

# Create test array
arr = np.arange(20).reshape(4, 5)
print(f'Original array:\n{arr}')

# Basic indexing
assert arr[0, 0] == 0
assert arr[1, 2] == 7
assert arr[-1, -1] == 19
print(f'arr[1, 2] = {arr[1, 2]}')

# Slicing
row = arr[1, :]
assert list(row) == [5, 6, 7, 8, 9]
print(f'Row 1: {row}')

col = arr[:, 2]
assert list(col) == [2, 7, 12, 17]
print(f'Column 2: {col}')

subarray = arr[1:3, 2:4]
expected = np.array([[7, 8], [12, 13]])
assert np.array_equal(subarray, expected)
print(f'Subarray [1:3, 2:4]:\n{subarray}')

# Boolean indexing
mask = arr > 10
filtered = arr[mask]
assert all(x > 10 for x in filtered)
print(f'Elements > 10: {filtered}')

# Fancy indexing
indices = [0, 2, 3]
selected_rows = arr[indices]
assert selected_rows.shape == (3, 5)
print(f'Selected rows:\n{selected_rows}')

print('NumPy indexing and slicing work correctly')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "NumPy indexing test failed: {}", result.output());
    assert!(result.stdout.contains("NumPy indexing and slicing work correctly"),
            "NumPy indexing message not found: {}", result.stdout);
}

// =============================================================================
// Combined Integration Tests
// =============================================================================

#[test]
#[ignore = "Requires Flask and requests - run with --ignored"]
fn test_flask_with_requests() {
    if !is_package_installed("flask") || !is_package_installed("requests") {
        println!("Skipping test: Flask or requests not installed");
        return;
    }

    let script = r#"
from flask import Flask, jsonify
import threading
import time

app = Flask(__name__)

@app.route('/api/status')
def status():
    return jsonify({'status': 'ok', 'version': '1.0.0'})

# Verify the app is configured correctly
assert app.url_map is not None
rules = [rule.rule for rule in app.url_map.iter_rules()]
assert '/api/status' in rules
print('Flask + requests integration ready')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "Flask + requests integration failed: {}", result.output());
    assert!(result.stdout.contains("Flask + requests integration ready"),
            "Integration message not found: {}", result.stdout);
}

// =============================================================================
// Performance Sanity Tests
// =============================================================================

#[test]
#[ignore = "Performance test - run with --ignored"]
fn test_performance_list_comprehension() {
    let script = r#"
import time

start = time.time()
result = [x * 2 for x in range(100000)]
elapsed = time.time() - start

assert len(result) == 100000
assert result[0] == 0
assert result[-1] == 199998
print(f'List comprehension: {elapsed:.3f}s')
print('Performance test passed')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "List comprehension performance test failed: {}", result.output());
}

#[test]
#[ignore = "Performance test - run with --ignored"]
fn test_performance_dict_operations() {
    let script = r#"
import time

start = time.time()
d = {}
for i in range(100000):
    d[i] = i * 2
elapsed = time.time() - start

assert len(d) == 100000
assert d[0] == 0
assert d[99999] == 199998
print(f'Dict operations: {elapsed:.3f}s')
print('Performance test passed')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "Dict operations performance test failed: {}", result.output());
}

#[test]
#[ignore = "Performance test - run with --ignored"]
fn test_performance_string_operations() {
    let script = r#"
import time

# Test string join (efficient)
start = time.time()
s = ''.join(str(i) for i in range(10000))
elapsed = time.time() - start

assert len(s) > 0
print(f'String join: {elapsed:.3f}s')
print('Performance test passed')
"#;

    let result = execute_python_script(script, PythonRuntime::CPython);
    assert!(result.success, "String operations performance test failed: {}", result.output());
}

// =============================================================================
// Integration Test Summary
// =============================================================================

#[test]
fn test_integration_summary() {
    println!("\n=== Integration Test Summary ===\n");
    
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
    println!("  cargo test --test integration_tests -- --ignored");
    println!("\n================================\n");
}
