//! Flask Framework Validation
//!
//! Provides validation infrastructure for testing Flask compatibility
//! with DX-Py runtime. This module implements:
//! - Flask app creation and route registration
//! - HTTP request handling validation
//! - Jinja2 template rendering
//! - Werkzeug C extension compatibility
//! - Flask test suite execution

use crate::{
    FailureCategorizer, FailureCategory, FrameworkInfo, FrameworkTestResult, TestFailure,
    TestFormat, TestResultParser, TestRunConfig, TestRunner,
};
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use thiserror::Error;

/// Errors that can occur during Flask validation
#[derive(Debug, Error)]
pub enum FlaskValidationError {
    #[error("Failed to create Flask app: {0}")]
    AppCreationFailed(String),

    #[error("Route registration failed: {0}")]
    RouteRegistrationFailed(String),

    #[error("HTTP request handling failed: {0}")]
    RequestHandlingFailed(String),

    #[error("Template rendering failed: {0}")]
    TemplateRenderingFailed(String),

    #[error("Flask test suite failed: {0}")]
    TestSuiteFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Timeout waiting for operation")]
    Timeout,

    #[error("Flask not installed or not found")]
    FlaskNotFound,
}

/// Flask validation configuration
#[derive(Debug, Clone)]
pub struct FlaskValidationConfig {
    /// Flask version to test
    pub flask_version: String,
    /// Path to Flask project or test directory
    pub project_path: Option<String>,
    /// Temporary directory for test projects
    pub temp_dir: Option<PathBuf>,
    /// Whether to run Flask's own test suite
    pub run_flask_tests: bool,
    /// Whether to test app creation
    pub test_app_creation: bool,
    /// Whether to test route registration
    pub test_routes: bool,
    /// Whether to test HTTP request handling
    pub test_requests: bool,
    /// Whether to test Jinja2 templates
    pub test_templates: bool,
    /// Whether to test Werkzeug C extensions
    pub test_werkzeug: bool,
    /// Timeout for test execution
    pub timeout: Duration,
    /// Python interpreter to use
    pub interpreter: String,
    /// Port for test server
    pub test_server_port: u16,
    /// Flask test modules to run (empty = all)
    pub test_modules: Vec<String>,
}

impl Default for FlaskValidationConfig {
    fn default() -> Self {
        Self {
            flask_version: "3.0".to_string(),
            project_path: None,
            temp_dir: None,
            run_flask_tests: true,
            test_app_creation: true,
            test_routes: true,
            test_requests: true,
            test_templates: true,
            test_werkzeug: true,
            timeout: Duration::from_secs(300),
            interpreter: "dx-py".to_string(),
            test_server_port: 5555,
            test_modules: Vec::new(),
        }
    }
}

impl FlaskValidationConfig {
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            flask_version: version.into(),
            ..Default::default()
        }
    }

    pub fn with_project_path(mut self, path: impl Into<String>) -> Self {
        self.project_path = Some(path.into());
        self
    }

    pub fn with_temp_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.temp_dir = Some(path.into());
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_interpreter(mut self, interpreter: impl Into<String>) -> Self {
        self.interpreter = interpreter.into();
        self
    }

    pub fn with_test_server_port(mut self, port: u16) -> Self {
        self.test_server_port = port;
        self
    }

    pub fn with_test_modules(mut self, modules: Vec<String>) -> Self {
        self.test_modules = modules;
        self
    }
}

/// Flask validator
pub struct FlaskValidator {
    config: FlaskValidationConfig,
    runner: TestRunner,
    parser: TestResultParser,
    categorizer: FailureCategorizer,
}

impl FlaskValidator {
    pub fn new(config: FlaskValidationConfig) -> Self {
        let run_config = TestRunConfig::default()
            .with_timeout(config.timeout)
            .with_format(TestFormat::Pytest);
        Self {
            config,
            runner: TestRunner::new(run_config),
            parser: TestResultParser::new(TestFormat::Pytest),
            categorizer: FailureCategorizer::new(),
        }
    }

    /// Create framework info for Flask
    pub fn framework_info(&self) -> FrameworkInfo {
        FrameworkInfo::new("Flask", &self.config.flask_version)
            .with_test_command("python -m pytest --tb=short -q")
            .with_min_pass_rate(0.95)
    }

    /// Validate Flask compatibility - runs all configured tests
    pub fn validate(&self) -> FrameworkTestResult {
        let framework = self.framework_info();
        let mut total_tests = 0;
        let mut passed = 0;
        let mut failed = 0;
        let skipped = 0;
        let errors = 0;
        let mut failure_categories: HashMap<FailureCategory, Vec<TestFailure>> = HashMap::new();
        let start = std::time::Instant::now();
        let mut raw_output = String::new();

        // Run subsystem tests
        if self.config.test_app_creation {
            let result = self.test_app_creation();
            total_tests += 1;
            match result {
                Ok(output) => {
                    passed += 1;
                    raw_output.push_str(&format!("[PASS] App creation\n{}\n", output));
                }
                Err(e) => {
                    failed += 1;
                    let msg = e.to_string();
                    raw_output.push_str(&format!("[FAIL] App creation: {}\n", msg));
                    let failure = TestFailure::new("flask_app_creation", msg);
                    let category = self.categorizer.categorize(&failure);
                    failure_categories.entry(category).or_default().push(failure);
                }
            }
        }

        if self.config.test_routes {
            let result = self.test_route_registration();
            total_tests += 1;
            match result {
                Ok(output) => {
                    passed += 1;
                    raw_output.push_str(&format!("[PASS] Route registration\n{}\n", output));
                }
                Err(e) => {
                    failed += 1;
                    let msg = e.to_string();
                    raw_output.push_str(&format!("[FAIL] Route registration: {}\n", msg));
                    let failure = TestFailure::new("flask_route_registration", msg);
                    let category = self.categorizer.categorize(&failure);
                    failure_categories.entry(category).or_default().push(failure);
                }
            }
        }

        if self.config.test_requests {
            let result = self.test_request_handling();
            total_tests += 1;
            match result {
                Ok(output) => {
                    passed += 1;
                    raw_output.push_str(&format!("[PASS] Request handling\n{}\n", output));
                }
                Err(e) => {
                    failed += 1;
                    let msg = e.to_string();
                    raw_output.push_str(&format!("[FAIL] Request handling: {}\n", msg));
                    let failure = TestFailure::new("flask_request_handling", msg);
                    let category = self.categorizer.categorize(&failure);
                    failure_categories.entry(category).or_default().push(failure);
                }
            }
        }

        if self.config.test_templates {
            let result = self.test_template_rendering();
            total_tests += 1;
            match result {
                Ok(output) => {
                    passed += 1;
                    raw_output.push_str(&format!("[PASS] Template rendering\n{}\n", output));
                }
                Err(e) => {
                    failed += 1;
                    let msg = e.to_string();
                    raw_output.push_str(&format!("[FAIL] Template rendering: {}\n", msg));
                    let failure = TestFailure::new("flask_template_rendering", msg);
                    let category = self.categorizer.categorize(&failure);
                    failure_categories.entry(category).or_default().push(failure);
                }
            }
        }

        if self.config.test_werkzeug {
            let result = self.test_werkzeug_compatibility();
            total_tests += 1;
            match result {
                Ok(output) => {
                    passed += 1;
                    raw_output.push_str(&format!("[PASS] Werkzeug compatibility\n{}\n", output));
                }
                Err(e) => {
                    failed += 1;
                    let msg = e.to_string();
                    raw_output.push_str(&format!("[FAIL] Werkzeug compatibility: {}\n", msg));
                    let failure = TestFailure::new("flask_werkzeug_compatibility", msg);
                    let category = self.categorizer.categorize(&failure);
                    failure_categories.entry(category).or_default().push(failure);
                }
            }
        }

        if self.config.run_flask_tests {
            let result = self.run_flask_test_suite();
            match result {
                Ok(test_result) => {
                    total_tests += test_result.total;
                    passed += test_result.passed;
                    failed += test_result.failed;
                    raw_output.push_str(&format!(
                        "[INFO] Flask test suite: {} passed, {} failed\n{}",
                        test_result.passed, test_result.failed, test_result.output
                    ));
                    for (cat, failures) in test_result.failures_by_category {
                        failure_categories.entry(cat).or_default().extend(failures);
                    }
                }
                Err(e) => {
                    total_tests += 1;
                    failed += 1;
                    let msg = e.to_string();
                    raw_output.push_str(&format!("[FAIL] Flask test suite: {}\n", msg));
                    let failure = TestFailure::new("flask_test_suite", msg);
                    let category = self.categorizer.categorize(&failure);
                    failure_categories.entry(category).or_default().push(failure);
                }
            }
        }

        FrameworkTestResult {
            framework,
            total_tests,
            passed,
            failed,
            skipped,
            errors,
            failure_categories,
            duration: start.elapsed(),
            timestamp: Utc::now(),
            raw_output: Some(raw_output),
        }
    }
}

impl FlaskValidator {
    /// Test Flask app creation
    fn test_app_creation(&self) -> Result<String, FlaskValidationError> {
        let test_script = r#"
from flask import Flask

# Create a basic Flask app
app = Flask(__name__)

# Verify app was created correctly
assert app is not None, "App creation failed"
assert app.name == '__main__', f"App name incorrect: {app.name}"
assert app.debug == False, "Debug should be False by default"

# Test app configuration
app.config['TESTING'] = True
assert app.config['TESTING'] == True, "Config setting failed"

# Test secret key setting
app.secret_key = 'test-secret-key'
assert app.secret_key == 'test-secret-key', "Secret key setting failed"

print("Flask app creation passed")
"#;

        let output = self.run_python_script(test_script)?;

        if output.contains("Flask app creation passed") {
            Ok("Flask app creation validated successfully".to_string())
        } else {
            Err(FlaskValidationError::AppCreationFailed(output))
        }
    }

    /// Test Flask route registration
    fn test_route_registration(&self) -> Result<String, FlaskValidationError> {
        let test_script = r#"
from flask import Flask

app = Flask(__name__)

# Test basic route registration
@app.route('/')
def index():
    return 'Hello, World!'

@app.route('/users')
def users():
    return 'Users list'

@app.route('/users/<int:user_id>')
def user_detail(user_id):
    return f'User {user_id}'

@app.route('/api/data', methods=['GET', 'POST'])
def api_data():
    return 'API data'

# Verify routes were registered
rules = list(app.url_map.iter_rules())
rule_endpoints = [r.endpoint for r in rules]

assert 'index' in rule_endpoints, "Index route not registered"
assert 'users' in rule_endpoints, "Users route not registered"
assert 'user_detail' in rule_endpoints, "User detail route not registered"
assert 'api_data' in rule_endpoints, "API data route not registered"

# Test route matching
with app.test_request_context('/'):
    assert app.url_map.bind('localhost').match('/') == ('index', {})

with app.test_request_context('/users/42'):
    endpoint, values = app.url_map.bind('localhost').match('/users/42')
    assert endpoint == 'user_detail', f"Wrong endpoint: {endpoint}"
    assert values == {'user_id': 42}, f"Wrong values: {values}"

print("Flask route registration passed")
"#;

        let output = self.run_python_script(test_script)?;

        if output.contains("Flask route registration passed") {
            Ok("Flask route registration validated successfully".to_string())
        } else {
            Err(FlaskValidationError::RouteRegistrationFailed(output))
        }
    }

    /// Test Flask HTTP request handling
    fn test_request_handling(&self) -> Result<String, FlaskValidationError> {
        let test_script = r#"
from flask import Flask, request, jsonify

app = Flask(__name__)
app.config['TESTING'] = True

@app.route('/')
def index():
    return 'Hello, World!'

@app.route('/echo', methods=['POST'])
def echo():
    data = request.get_json()
    return jsonify(data)

@app.route('/headers')
def headers():
    user_agent = request.headers.get('User-Agent', 'Unknown')
    return f'User-Agent: {user_agent}'

@app.route('/query')
def query():
    name = request.args.get('name', 'Guest')
    return f'Hello, {name}!'

# Test with test client
client = app.test_client()

# Test GET request
response = client.get('/')
assert response.status_code == 200, f"GET / failed: {response.status_code}"
assert response.data == b'Hello, World!', f"GET / wrong data: {response.data}"

# Test POST with JSON
response = client.post('/echo', 
    json={'message': 'test'},
    content_type='application/json')
assert response.status_code == 200, f"POST /echo failed: {response.status_code}"
data = response.get_json()
assert data == {'message': 'test'}, f"POST /echo wrong data: {data}"

# Test headers
response = client.get('/headers', headers={'User-Agent': 'TestClient/1.0'})
assert response.status_code == 200, f"GET /headers failed: {response.status_code}"
assert b'TestClient/1.0' in response.data, f"Headers not passed: {response.data}"

# Test query parameters
response = client.get('/query?name=Alice')
assert response.status_code == 200, f"GET /query failed: {response.status_code}"
assert b'Hello, Alice!' in response.data, f"Query params not parsed: {response.data}"

print("Flask request handling passed")
"#;

        let output = self.run_python_script(test_script)?;

        if output.contains("Flask request handling passed") {
            Ok("Flask request handling validated successfully".to_string())
        } else {
            Err(FlaskValidationError::RequestHandlingFailed(output))
        }
    }
}

impl FlaskValidator {
    /// Test Jinja2 template rendering
    fn test_template_rendering(&self) -> Result<String, FlaskValidationError> {
        let test_script = r#"
from flask import Flask, render_template_string

app = Flask(__name__)
app.config['TESTING'] = True

# Test basic template rendering
with app.app_context():
    # Simple variable substitution
    result = render_template_string('Hello, {{ name }}!', name='World')
    assert result == 'Hello, World!', f"Basic render failed: {result}"
    
    # If statement
    result = render_template_string('{% if show %}Visible{% endif %}', show=True)
    assert result == 'Visible', f"If statement failed: {result}"
    
    result = render_template_string('{% if show %}Visible{% endif %}', show=False)
    assert result == '', f"If statement (false) failed: {result}"
    
    # For loop
    result = render_template_string('{% for item in items %}{{ item }}{% endfor %}', items=['a', 'b', 'c'])
    assert result == 'abc', f"For loop failed: {result}"
    
    # Filters
    result = render_template_string('{{ name|upper }}', name='test')
    assert result == 'TEST', f"Upper filter failed: {result}"
    
    result = render_template_string('{{ name|lower }}', name='TEST')
    assert result == 'test', f"Lower filter failed: {result}"
    
    result = render_template_string('{{ items|length }}', items=[1, 2, 3])
    assert result == '3', f"Length filter failed: {result}"
    
    # Auto-escaping
    result = render_template_string('{{ html }}', html='<script>alert("xss")</script>')
    assert '&lt;' in result, f"Auto-escape failed: {result}"
    assert '<script>' not in result, f"XSS not escaped: {result}"
    
    # Safe filter (disable escaping)
    from markupsafe import Markup
    result = render_template_string('{{ html|safe }}', html=Markup('<b>bold</b>'))
    assert '<b>bold</b>' in result, f"Safe filter failed: {result}"

print("Flask template rendering passed")
"#;

        let output = self.run_python_script(test_script)?;

        if output.contains("Flask template rendering passed") {
            Ok("Flask template rendering validated successfully".to_string())
        } else {
            Err(FlaskValidationError::TemplateRenderingFailed(output))
        }
    }

    /// Test Werkzeug C extension compatibility
    fn test_werkzeug_compatibility(&self) -> Result<String, FlaskValidationError> {
        let test_script = r#"
from werkzeug.routing import Map, Rule
from werkzeug.wrappers import Request, Response
from werkzeug.test import Client
from werkzeug.serving import WSGIRequestHandler

# Test URL routing
url_map = Map([
    Rule('/', endpoint='index'),
    Rule('/users', endpoint='users'),
    Rule('/users/<int:id>', endpoint='user'),
    Rule('/files/<path:filename>', endpoint='file'),
])

# Test URL matching
adapter = url_map.bind('localhost')

endpoint, values = adapter.match('/')
assert endpoint == 'index', f"Index match failed: {endpoint}"

endpoint, values = adapter.match('/users')
assert endpoint == 'users', f"Users match failed: {endpoint}"

endpoint, values = adapter.match('/users/42')
assert endpoint == 'user', f"User match failed: {endpoint}"
assert values == {'id': 42}, f"User values failed: {values}"

endpoint, values = adapter.match('/files/path/to/file.txt')
assert endpoint == 'file', f"File match failed: {endpoint}"
assert values == {'filename': 'path/to/file.txt'}, f"File values failed: {values}"

# Test URL building
url = adapter.build('user', {'id': 123})
assert url == '/users/123', f"URL build failed: {url}"

# Test Request/Response
def application(environ, start_response):
    request = Request(environ)
    response = Response(f'Hello, {request.args.get("name", "World")}!')
    return response(environ, start_response)

client = Client(application, Response)
response = client.get('/?name=Test')
assert response.status_code == 200, f"Response status failed: {response.status_code}"
assert b'Hello, Test!' in response.data, f"Response data failed: {response.data}"

# Test multidict
from werkzeug.datastructures import MultiDict
md = MultiDict([('key', 'value1'), ('key', 'value2')])
assert md.get('key') == 'value1', f"MultiDict get failed"
assert md.getlist('key') == ['value1', 'value2'], f"MultiDict getlist failed"

print("Werkzeug compatibility passed")
"#;

        let output = self.run_python_script(test_script)?;

        if output.contains("Werkzeug compatibility passed") {
            Ok("Werkzeug compatibility validated successfully".to_string())
        } else {
            Err(FlaskValidationError::TestSuiteFailed(output))
        }
    }
}

impl FlaskValidator {
    /// Run Flask's test suite
    fn run_flask_test_suite(&self) -> Result<FlaskTestResult, FlaskValidationError> {
        let test_modules = if self.config.test_modules.is_empty() {
            // Default to a subset of core tests for faster validation
            vec![
                "tests.test_basic",
                "tests.test_reqctx",
                "tests.test_templating",
            ]
        } else {
            self.config.test_modules.iter().map(|s| s.as_str()).collect()
        };

        let mut args = vec!["-m", "pytest", "-v", "--tb=short"];
        for module in &test_modules {
            args.push(module);
        }

        let output = std::process::Command::new(&self.config.interpreter)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let combined = format!("{}\n{}", stdout, stderr);

        // Parse test results from output
        let (total, passed, failed, failures) = self.parse_pytest_output(&combined);

        // Categorize failures
        let mut failures_by_category: HashMap<FailureCategory, Vec<TestFailure>> = HashMap::new();
        for failure in failures {
            let category = self.categorizer.categorize(&failure);
            failures_by_category.entry(category).or_default().push(failure);
        }

        Ok(FlaskTestResult {
            total,
            passed,
            failed,
            output: combined,
            failures_by_category,
        })
    }

    /// Parse pytest output to extract results
    fn parse_pytest_output(&self, output: &str) -> (usize, usize, usize, Vec<TestFailure>) {
        let mut passed: usize = 0;
        let mut failed: usize = 0;
        let mut failures = Vec::new();

        for line in output.lines() {
            // Look for pytest summary line: "X passed, Y failed in Z.ZZs"
            if line.contains(" passed") || line.contains(" failed") {
                // Parse "X passed"
                if let Some(idx) = line.find(" passed") {
                    let before = &line[..idx];
                    if let Some(num_str) = before.split_whitespace().last() {
                        passed = num_str.parse().unwrap_or(0);
                    }
                }
                // Parse "X failed"
                if let Some(idx) = line.find(" failed") {
                    let before = &line[..idx];
                    if let Some(num_str) = before.split(',').next_back() {
                        if let Some(num) = num_str.split_whitespace().next() {
                            failed = num.parse().unwrap_or(0);
                        }
                    }
                }
            }

            // Capture individual test failures
            if line.starts_with("FAILED") {
                let test_name = line.split_whitespace().nth(1).unwrap_or("unknown");
                failures.push(TestFailure::new(test_name, line.to_string()));
            }
        }

        let total = passed + failed;
        (total, passed, failed, failures)
    }

    /// Run a Python script and capture output
    fn run_python_script(&self, script: &str) -> Result<String, FlaskValidationError> {
        let output = std::process::Command::new(&self.config.interpreter)
            .args(["-c", script])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            return Err(FlaskValidationError::TestSuiteFailed(format!("{}\n{}", stdout, stderr)));
        }

        Ok(format!("{}\n{}", stdout, stderr))
    }

    /// Get the test runner
    pub fn runner(&self) -> &TestRunner {
        &self.runner
    }

    /// Get the parser
    pub fn parser(&self) -> &TestResultParser {
        &self.parser
    }
}

/// Result from running Flask tests
#[derive(Debug, Clone)]
pub struct FlaskTestResult {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub output: String,
    pub failures_by_category: HashMap<FailureCategory, Vec<TestFailure>>,
}

/// Flask test categories for detailed reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlaskTestCategory {
    Core,
    Routing,
    Requests,
    Responses,
    Templates,
    Sessions,
    Blueprints,
    ErrorHandling,
    Extensions,
    Werkzeug,
}

impl FlaskTestCategory {
    pub fn from_test_name(name: &str) -> Self {
        let lower = name.to_lowercase();
        // Check more specific patterns first to avoid substring conflicts
        if lower.contains("blueprint") {
            Self::Blueprints
        } else if lower.contains("werkzeug") {
            Self::Werkzeug
        } else if lower.contains("template") || lower.contains("jinja") {
            Self::Templates
        } else if lower.contains("session") || lower.contains("cookie") {
            Self::Sessions
        } else if lower.contains("error") || lower.contains("exception") {
            Self::ErrorHandling
        } else if lower.contains("extension") {
            Self::Extensions
        } else if lower.contains("route") || lower.contains("url") {
            Self::Routing
        } else if lower.contains("request") {
            Self::Requests
        } else if lower.contains("response") {
            Self::Responses
        } else {
            Self::Core
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Core => "Core",
            Self::Routing => "URL Routing",
            Self::Requests => "Request Handling",
            Self::Responses => "Response Handling",
            Self::Templates => "Templates/Jinja2",
            Self::Sessions => "Sessions/Cookies",
            Self::Blueprints => "Blueprints",
            Self::ErrorHandling => "Error Handling",
            Self::Extensions => "Extensions",
            Self::Werkzeug => "Werkzeug",
        }
    }
}

/// Flask-specific compatibility report
#[derive(Debug, Clone)]
pub struct FlaskCompatibilityReport {
    pub result: FrameworkTestResult,
    pub by_category: HashMap<FlaskTestCategory, SubsystemResult>,
    pub werkzeug_status: WerkzeugStatus,
}

#[derive(Debug, Clone)]
pub struct SubsystemResult {
    pub category: FlaskTestCategory,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
}

impl SubsystemResult {
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.passed as f64 / self.total as f64
        }
    }
}

#[derive(Debug, Clone)]
pub struct WerkzeugStatus {
    pub url_routing: bool,
    pub request_handling: bool,
    pub response_handling: bool,
    pub multidict: bool,
    pub local_stack: bool,
}

impl Default for WerkzeugStatus {
    fn default() -> Self {
        Self {
            url_routing: true,
            request_handling: true,
            response_handling: true,
            multidict: true,
            local_stack: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flask_validation_config() {
        let config = FlaskValidationConfig::new("3.0")
            .with_project_path("/tmp/test")
            .with_timeout(Duration::from_secs(60))
            .with_interpreter("python3")
            .with_test_server_port(9000);

        assert_eq!(config.flask_version, "3.0");
        assert_eq!(config.project_path, Some("/tmp/test".to_string()));
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.interpreter, "python3");
        assert_eq!(config.test_server_port, 9000);
    }

    #[test]
    fn test_flask_validation_config_defaults() {
        let config = FlaskValidationConfig::default();
        assert_eq!(config.flask_version, "3.0");
        assert_eq!(config.interpreter, "dx-py");
        assert_eq!(config.test_server_port, 5555);
        assert!(config.test_app_creation);
        assert!(config.test_routes);
        assert!(config.test_requests);
        assert!(config.test_templates);
        assert!(config.test_werkzeug);
    }

    #[test]
    fn test_flask_validator_framework_info() {
        let config = FlaskValidationConfig::new("3.0");
        let validator = FlaskValidator::new(config);
        let info = validator.framework_info();

        assert_eq!(info.name, "Flask");
        assert_eq!(info.version, "3.0");
        assert_eq!(info.min_pass_rate, 0.95);
    }

    #[test]
    fn test_flask_test_category() {
        assert_eq!(
            FlaskTestCategory::from_test_name("test_route_matching"),
            FlaskTestCategory::Routing
        );
        assert_eq!(
            FlaskTestCategory::from_test_name("test_url_building"),
            FlaskTestCategory::Routing
        );
        assert_eq!(
            FlaskTestCategory::from_test_name("test_request_parsing"),
            FlaskTestCategory::Requests
        );
        assert_eq!(
            FlaskTestCategory::from_test_name("test_response_headers"),
            FlaskTestCategory::Responses
        );
        assert_eq!(
            FlaskTestCategory::from_test_name("test_template_render"),
            FlaskTestCategory::Templates
        );
        assert_eq!(
            FlaskTestCategory::from_test_name("test_jinja_filters"),
            FlaskTestCategory::Templates
        );
        assert_eq!(
            FlaskTestCategory::from_test_name("test_session_data"),
            FlaskTestCategory::Sessions
        );
        // Note: "blueprint_routes" contains "route" so it matches Routing first
        assert_eq!(
            FlaskTestCategory::from_test_name("test_blueprints"),
            FlaskTestCategory::Blueprints
        );
        assert_eq!(
            FlaskTestCategory::from_test_name("test_error_handler"),
            FlaskTestCategory::ErrorHandling
        );
        assert_eq!(
            FlaskTestCategory::from_test_name("test_werkzeug_local"),
            FlaskTestCategory::Werkzeug
        );
        assert_eq!(FlaskTestCategory::from_test_name("test_something"), FlaskTestCategory::Core);
    }

    #[test]
    fn test_subsystem_result() {
        let result = SubsystemResult {
            category: FlaskTestCategory::Routing,
            total: 100,
            passed: 95,
            failed: 5,
        };
        assert_eq!(result.pass_rate(), 0.95);
    }

    #[test]
    fn test_subsystem_result_empty() {
        let result = SubsystemResult {
            category: FlaskTestCategory::Core,
            total: 0,
            passed: 0,
            failed: 0,
        };
        assert_eq!(result.pass_rate(), 0.0);
    }

    #[test]
    fn test_werkzeug_status() {
        let status = WerkzeugStatus::default();
        assert!(status.url_routing);
        assert!(status.request_handling);
        assert!(status.response_handling);
        assert!(status.multidict);
        assert!(status.local_stack);
    }

    #[test]
    fn test_parse_pytest_output() {
        let config = FlaskValidationConfig::default();
        let validator = FlaskValidator::new(config);

        let output = r#"
test_basic.py::test_index PASSED
test_basic.py::test_users PASSED
test_basic.py::test_error FAILED
========================= 2 passed, 1 failed in 0.5s =========================
"#;

        let (total, passed, failed, _failures) = validator.parse_pytest_output(output);
        assert_eq!(total, 3);
        assert_eq!(passed, 2);
        assert_eq!(failed, 1);
    }

    #[test]
    fn test_parse_pytest_output_all_passed() {
        let config = FlaskValidationConfig::default();
        let validator = FlaskValidator::new(config);

        let output = r#"
test_basic.py::test_index PASSED
test_basic.py::test_users PASSED
========================= 2 passed in 0.3s =========================
"#;

        let (total, passed, failed, _failures) = validator.parse_pytest_output(output);
        assert_eq!(total, 2);
        assert_eq!(passed, 2);
        assert_eq!(failed, 0);
    }
}
