//! Django Framework Validation
//!
//! Provides validation infrastructure for testing Django compatibility
//! with DX-Py runtime. This module implements:
//! - Project creation validation (django-admin startproject)
//! - Development server testing
//! - Django core test suite execution

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

/// Errors that can occur during Django validation
#[derive(Debug, Error)]
pub enum DjangoValidationError {
    #[error("Failed to create Django project: {0}")]
    ProjectCreationFailed(String),

    #[error("Development server failed to start: {0}")]
    DevServerFailed(String),

    #[error("Django test suite failed: {0}")]
    TestSuiteFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Timeout waiting for operation")]
    Timeout,

    #[error("Django not installed or not found")]
    DjangoNotFound,
}

/// Django validation configuration
#[derive(Debug, Clone)]
pub struct DjangoValidationConfig {
    /// Django version to test
    pub django_version: String,
    /// Path to Django project or test directory
    pub project_path: Option<String>,
    /// Temporary directory for test projects
    pub temp_dir: Option<PathBuf>,
    /// Whether to run Django's own test suite
    pub run_django_tests: bool,
    /// Whether to test project creation
    pub test_project_creation: bool,
    /// Whether to test development server
    pub test_dev_server: bool,
    /// Whether to test ORM operations
    pub test_orm: bool,
    /// Whether to test template rendering
    pub test_templates: bool,
    /// Timeout for test execution
    pub timeout: Duration,
    /// Python interpreter to use
    pub interpreter: String,
    /// Port for development server testing
    pub dev_server_port: u16,
    /// Django test modules to run (empty = all)
    pub test_modules: Vec<String>,
}

impl Default for DjangoValidationConfig {
    fn default() -> Self {
        Self {
            django_version: "4.2".to_string(),
            project_path: None,
            temp_dir: None,
            run_django_tests: true,
            test_project_creation: true,
            test_dev_server: false, // Requires network
            test_orm: true,
            test_templates: true,
            timeout: Duration::from_secs(300),
            interpreter: "dx-py".to_string(),
            dev_server_port: 8765,
            test_modules: Vec::new(),
        }
    }
}

impl DjangoValidationConfig {
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            django_version: version.into(),
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

    pub fn with_dev_server_port(mut self, port: u16) -> Self {
        self.dev_server_port = port;
        self
    }

    pub fn with_test_modules(mut self, modules: Vec<String>) -> Self {
        self.test_modules = modules;
        self
    }

    pub fn enable_dev_server_test(mut self) -> Self {
        self.test_dev_server = true;
        self
    }
}

/// Django validator
pub struct DjangoValidator {
    config: DjangoValidationConfig,
    runner: TestRunner,
    parser: TestResultParser,
    categorizer: FailureCategorizer,
}

impl DjangoValidator {
    pub fn new(config: DjangoValidationConfig) -> Self {
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

    /// Create framework info for Django
    pub fn framework_info(&self) -> FrameworkInfo {
        FrameworkInfo::new("Django", &self.config.django_version)
            .with_test_command("python -m pytest --tb=short -q")
            .with_min_pass_rate(0.90)
            .with_env("DJANGO_SETTINGS_MODULE", "test_settings")
    }

    /// Validate Django compatibility - runs all configured tests
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
        if self.config.test_project_creation {
            let result = self.test_project_creation();
            total_tests += 1;
            match result {
                Ok(output) => {
                    passed += 1;
                    raw_output.push_str(&format!("[PASS] Project creation\n{}\n", output));
                }
                Err(e) => {
                    failed += 1;
                    let msg = e.to_string();
                    raw_output.push_str(&format!("[FAIL] Project creation: {}\n", msg));
                    let failure = TestFailure::new("django_project_creation", msg);
                    let category = self.categorizer.categorize(&failure);
                    failure_categories.entry(category).or_default().push(failure);
                }
            }
        }

        if self.config.test_dev_server {
            let result = self.test_dev_server();
            total_tests += 1;
            match result {
                Ok(output) => {
                    passed += 1;
                    raw_output.push_str(&format!("[PASS] Development server\n{}\n", output));
                }
                Err(e) => {
                    failed += 1;
                    let msg = e.to_string();
                    raw_output.push_str(&format!("[FAIL] Development server: {}\n", msg));
                    let failure = TestFailure::new("django_dev_server", msg);
                    let category = self.categorizer.categorize(&failure);
                    failure_categories.entry(category).or_default().push(failure);
                }
            }
        }

        if self.config.test_orm {
            let result = self.test_orm_operations();
            total_tests += 1;
            match result {
                Ok(output) => {
                    passed += 1;
                    raw_output.push_str(&format!("[PASS] ORM operations\n{}\n", output));
                }
                Err(e) => {
                    failed += 1;
                    let msg = e.to_string();
                    raw_output.push_str(&format!("[FAIL] ORM operations: {}\n", msg));
                    let failure = TestFailure::new("django_orm_operations", msg);
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
                    let failure = TestFailure::new("django_template_rendering", msg);
                    let category = self.categorizer.categorize(&failure);
                    failure_categories.entry(category).or_default().push(failure);
                }
            }
        }

        if self.config.run_django_tests {
            let result = self.run_django_core_tests();
            match result {
                Ok(test_result) => {
                    total_tests += test_result.total;
                    passed += test_result.passed;
                    failed += test_result.failed;
                    raw_output.push_str(&format!(
                        "[INFO] Django core tests: {} passed, {} failed\n{}",
                        test_result.passed, test_result.failed, test_result.output
                    ));
                    // Merge failure categories
                    for (cat, failures) in test_result.failures_by_category {
                        failure_categories.entry(cat).or_default().extend(failures);
                    }
                }
                Err(e) => {
                    total_tests += 1;
                    failed += 1;
                    let msg = e.to_string();
                    raw_output.push_str(&format!("[FAIL] Django core tests: {}\n", msg));
                    let failure = TestFailure::new("django_core_tests", msg);
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

    /// Test Django project creation (django-admin startproject)
    fn test_project_creation(&self) -> Result<String, DjangoValidationError> {
        let temp_dir = self.get_temp_dir()?;
        let project_name = "test_project";
        let project_path = temp_dir.join(project_name);

        // Clean up any existing project
        if project_path.exists() {
            std::fs::remove_dir_all(&project_path)?;
        }

        // Run django-admin startproject
        let output = std::process::Command::new(&self.config.interpreter)
            .args(["-m", "django", "startproject", project_name])
            .current_dir(&temp_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DjangoValidationError::ProjectCreationFailed(stderr.to_string()));
        }

        // Verify project structure
        let expected_files = [
            project_path.join("manage.py"),
            project_path.join(project_name).join("settings.py"),
            project_path.join(project_name).join("urls.py"),
            project_path.join(project_name).join("wsgi.py"),
        ];

        for file in &expected_files {
            if !file.exists() {
                return Err(DjangoValidationError::ProjectCreationFailed(format!(
                    "Missing expected file: {}",
                    file.display()
                )));
            }
        }

        Ok(format!("Successfully created Django project at {}", project_path.display()))
    }

    /// Test Django development server startup
    fn test_dev_server(&self) -> Result<String, DjangoValidationError> {
        let temp_dir = self.get_temp_dir()?;
        let project_name = "test_project";
        let project_path = temp_dir.join(project_name);

        // Ensure project exists
        if !project_path.exists() {
            self.test_project_creation()?;
        }

        let port = self.config.dev_server_port;

        // Start the development server
        let mut child = std::process::Command::new(&self.config.interpreter)
            .args([
                "manage.py",
                "runserver",
                &format!("127.0.0.1:{}", port),
                "--noreload",
            ])
            .current_dir(&project_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Wait a bit for server to start
        std::thread::sleep(Duration::from_secs(2));

        // Try to connect to the server
        let client_result = std::net::TcpStream::connect_timeout(
            &format!("127.0.0.1:{}", port).parse().unwrap(),
            Duration::from_secs(5),
        );

        // Kill the server
        let _ = child.kill();

        match client_result {
            Ok(_) => Ok(format!("Development server started successfully on port {}", port)),
            Err(e) => Err(DjangoValidationError::DevServerFailed(format!(
                "Could not connect to server: {}",
                e
            ))),
        }
    }

    /// Test Django ORM operations
    fn test_orm_operations(&self) -> Result<String, DjangoValidationError> {
        // Create a test script that exercises ORM operations
        let test_script = r#"
import django
from django.conf import settings

# Configure minimal Django settings
if not settings.configured:
    settings.configure(
        DATABASES={
            'default': {
                'ENGINE': 'django.db.backends.sqlite3',
                'NAME': ':memory:',
            }
        },
        INSTALLED_APPS=[
            'django.contrib.contenttypes',
            'django.contrib.auth',
        ],
        DEFAULT_AUTO_FIELD='django.db.models.BigAutoField',
    )

django.setup()

from django.db import connection
from django.contrib.auth.models import User

# Create tables
with connection.schema_editor() as schema_editor:
    schema_editor.create_model(User)

# Test basic ORM operations
user = User.objects.create_user('testuser', 'test@example.com', 'password123')
assert user.pk is not None, "User creation failed"

# Test query
found = User.objects.get(username='testuser')
assert found.email == 'test@example.com', "Query failed"

# Test filter
users = User.objects.filter(username__startswith='test')
assert users.count() == 1, "Filter failed"

# Test update
user.email = 'updated@example.com'
user.save()
user.refresh_from_db()
assert user.email == 'updated@example.com', "Update failed"

# Test delete
user.delete()
assert User.objects.count() == 0, "Delete failed"

print("All ORM operations passed")
"#;

        let output = self.run_python_script(test_script)?;

        if output.contains("All ORM operations passed") {
            Ok("ORM operations validated successfully".to_string())
        } else {
            Err(DjangoValidationError::TestSuiteFailed(output))
        }
    }

    /// Test Django template rendering
    fn test_template_rendering(&self) -> Result<String, DjangoValidationError> {
        let test_script = r#"
import django
from django.conf import settings

if not settings.configured:
    settings.configure(
        TEMPLATES=[{
            'BACKEND': 'django.template.backends.django.DjangoTemplates',
            'DIRS': [],
            'APP_DIRS': False,
            'OPTIONS': {
                'context_processors': [],
            },
        }],
    )

django.setup()

from django.template import Template, Context

# Test basic template rendering
template = Template("Hello, {{ name }}!")
context = Context({'name': 'World'})
result = template.render(context)
assert result == "Hello, World!", f"Basic render failed: {result}"

# Test template tags
template = Template("{% if show %}Visible{% endif %}")
context = Context({'show': True})
result = template.render(context)
assert result == "Visible", f"If tag failed: {result}"

# Test for loop
template = Template("{% for item in items %}{{ item }}{% endfor %}")
context = Context({'items': ['a', 'b', 'c']})
result = template.render(context)
assert result == "abc", f"For loop failed: {result}"

# Test filters
template = Template("{{ name|upper }}")
context = Context({'name': 'test'})
result = template.render(context)
assert result == "TEST", f"Filter failed: {result}"

# Test auto-escaping
template = Template("{{ html }}")
context = Context({'html': '<script>alert("xss")</script>'})
result = template.render(context)
assert '&lt;' in result, f"Auto-escape failed: {result}"

print("All template operations passed")
"#;

        let output = self.run_python_script(test_script)?;

        if output.contains("All template operations passed") {
            Ok("Template rendering validated successfully".to_string())
        } else {
            Err(DjangoValidationError::TestSuiteFailed(output))
        }
    }

    /// Run Django's core test suite
    fn run_django_core_tests(&self) -> Result<DjangoCoreTestResult, DjangoValidationError> {
        let test_modules = if self.config.test_modules.is_empty() {
            // Default to a subset of core tests for faster validation
            vec![
                "template_tests.filter_tests",
                "template_tests.syntax_tests",
                "model_fields",
                "queries",
                "urlpatterns",
            ]
        } else {
            self.config.test_modules.iter().map(|s| s.as_str()).collect()
        };

        let mut args = vec!["-m", "django", "test", "--verbosity=2"];
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
        let (total, passed, failed, failures) = self.parse_django_test_output(&combined);

        // Categorize failures
        let mut failures_by_category: HashMap<FailureCategory, Vec<TestFailure>> = HashMap::new();
        for failure in failures {
            let category = self.categorizer.categorize(&failure);
            failures_by_category.entry(category).or_default().push(failure);
        }

        Ok(DjangoCoreTestResult {
            total,
            passed,
            failed,
            output: combined,
            failures_by_category,
        })
    }

    /// Parse Django test output to extract results
    fn parse_django_test_output(&self, output: &str) -> (usize, usize, usize, Vec<TestFailure>) {
        let mut total: usize = 0;
        let mut failed: usize = 0;
        let mut failures = Vec::new();

        // Look for Django test summary line: "Ran X tests in Y.YYYs"
        for line in output.lines() {
            if line.starts_with("Ran ") && line.contains(" tests in ") {
                if let Some(num_str) = line.strip_prefix("Ran ") {
                    if let Some(num) = num_str.split_whitespace().next() {
                        total = num.parse().unwrap_or(0);
                    }
                }
            }

            // Count failures from "FAILED (failures=X)" or "FAILED (errors=X)"
            if line.contains("FAILED") {
                if let Some(start) = line.find("failures=") {
                    let rest = &line[start + 9..];
                    if let Some(end) = rest.find(|c: char| !c.is_ascii_digit()) {
                        failed += rest[..end].parse::<usize>().unwrap_or(0);
                    }
                }
                if let Some(start) = line.find("errors=") {
                    let rest = &line[start + 7..];
                    if let Some(end) = rest.find(|c: char| !c.is_ascii_digit()) {
                        failed += rest[..end].parse::<usize>().unwrap_or(0);
                    }
                }
            }

            // Capture individual test failures
            if line.starts_with("FAIL:") || line.starts_with("ERROR:") {
                let test_name = line.split_whitespace().nth(1).unwrap_or("unknown");
                failures.push(TestFailure::new(test_name, line.to_string()));
            }
        }

        let passed = total.saturating_sub(failed);
        (total, passed, failed, failures)
    }

    /// Run a Python script and capture output
    fn run_python_script(&self, script: &str) -> Result<String, DjangoValidationError> {
        let output = std::process::Command::new(&self.config.interpreter)
            .args(["-c", script])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            return Err(DjangoValidationError::TestSuiteFailed(format!("{}\n{}", stdout, stderr)));
        }

        Ok(format!("{}\n{}", stdout, stderr))
    }

    /// Get or create temporary directory for tests
    fn get_temp_dir(&self) -> Result<PathBuf, DjangoValidationError> {
        if let Some(ref dir) = self.config.temp_dir {
            if !dir.exists() {
                std::fs::create_dir_all(dir)?;
            }
            Ok(dir.clone())
        } else {
            Ok(std::env::temp_dir().join("dx-py-django-validation"))
        }
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

/// Result from running Django core tests
#[derive(Debug, Clone)]
pub struct DjangoCoreTestResult {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub output: String,
    pub failures_by_category: HashMap<FailureCategory, Vec<TestFailure>>,
}

/// Django test categories for detailed reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DjangoTestCategory {
    Core,
    Orm,
    Templates,
    Urls,
    Forms,
    Admin,
    Auth,
    Middleware,
    StaticFiles,
    ManagementCommands,
}

impl DjangoTestCategory {
    pub fn from_test_name(name: &str) -> Self {
        let lower = name.to_lowercase();
        // Check more specific patterns first to avoid substring conflicts
        // e.g., "form" contains "orm" as substring
        if lower.contains("template") {
            Self::Templates
        } else if lower.contains("form") && !lower.contains("transform") {
            Self::Forms
        } else if lower.contains("orm") || lower.contains("model") || lower.contains("query") {
            Self::Orm
        } else if lower.contains("url") || lower.contains("route") {
            Self::Urls
        } else if lower.contains("admin") {
            Self::Admin
        } else if lower.contains("auth") || lower.contains("login") || lower.contains("user") {
            Self::Auth
        } else if lower.contains("middleware") {
            Self::Middleware
        } else if lower.contains("static") {
            Self::StaticFiles
        } else if lower.contains("command") || lower.contains("manage") {
            Self::ManagementCommands
        } else {
            Self::Core
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Core => "Core",
            Self::Orm => "ORM/Database",
            Self::Templates => "Templates",
            Self::Urls => "URL Routing",
            Self::Forms => "Forms",
            Self::Admin => "Admin",
            Self::Auth => "Authentication",
            Self::Middleware => "Middleware",
            Self::StaticFiles => "Static Files",
            Self::ManagementCommands => "Management Commands",
        }
    }
}

/// Django-specific compatibility report
#[derive(Debug, Clone)]
pub struct DjangoCompatibilityReport {
    pub result: FrameworkTestResult,
    pub by_category: HashMap<DjangoTestCategory, SubsystemResult>,
    pub c_extension_status: CExtensionStatus,
}

#[derive(Debug, Clone)]
pub struct SubsystemResult {
    pub category: DjangoTestCategory,
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
pub struct CExtensionStatus {
    pub json_parsing: bool,
    pub password_hashing: bool,
    pub database_adapters: bool,
    pub template_extensions: bool,
}

impl Default for CExtensionStatus {
    fn default() -> Self {
        Self {
            json_parsing: true,
            password_hashing: true,
            database_adapters: true,
            template_extensions: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_django_validation_config() {
        let config = DjangoValidationConfig::new("4.2")
            .with_project_path("/tmp/test")
            .with_timeout(Duration::from_secs(60))
            .with_interpreter("python3")
            .with_dev_server_port(9000);

        assert_eq!(config.django_version, "4.2");
        assert_eq!(config.project_path, Some("/tmp/test".to_string()));
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.interpreter, "python3");
        assert_eq!(config.dev_server_port, 9000);
    }

    #[test]
    fn test_django_validation_config_defaults() {
        let config = DjangoValidationConfig::default();
        assert_eq!(config.django_version, "4.2");
        assert_eq!(config.interpreter, "dx-py");
        assert_eq!(config.dev_server_port, 8765);
        assert!(config.test_project_creation);
        assert!(config.test_orm);
        assert!(config.test_templates);
        assert!(!config.test_dev_server);
    }

    #[test]
    fn test_django_validator_framework_info() {
        let config = DjangoValidationConfig::new("4.2");
        let validator = DjangoValidator::new(config);
        let info = validator.framework_info();

        assert_eq!(info.name, "Django");
        assert_eq!(info.version, "4.2");
        assert_eq!(info.min_pass_rate, 0.90);
    }

    #[test]
    fn test_django_test_category() {
        assert_eq!(
            DjangoTestCategory::from_test_name("test_model_creation"),
            DjangoTestCategory::Orm
        );
        assert_eq!(
            DjangoTestCategory::from_test_name("test_template_render"),
            DjangoTestCategory::Templates
        );
        assert_eq!(
            DjangoTestCategory::from_test_name("test_url_routing"),
            DjangoTestCategory::Urls
        );
        assert_eq!(
            DjangoTestCategory::from_test_name("test_form_validation"),
            DjangoTestCategory::Forms
        );
        assert_eq!(
            DjangoTestCategory::from_test_name("test_admin_site"),
            DjangoTestCategory::Admin
        );
        assert_eq!(DjangoTestCategory::from_test_name("test_user_login"), DjangoTestCategory::Auth);
        assert_eq!(
            DjangoTestCategory::from_test_name("test_middleware_process"),
            DjangoTestCategory::Middleware
        );
        assert_eq!(
            DjangoTestCategory::from_test_name("test_static_files"),
            DjangoTestCategory::StaticFiles
        );
        assert_eq!(
            DjangoTestCategory::from_test_name("test_management_command"),
            DjangoTestCategory::ManagementCommands
        );
        assert_eq!(DjangoTestCategory::from_test_name("test_something"), DjangoTestCategory::Core);
    }

    #[test]
    fn test_subsystem_result() {
        let result = SubsystemResult {
            category: DjangoTestCategory::Orm,
            total: 100,
            passed: 90,
            failed: 10,
        };
        assert_eq!(result.pass_rate(), 0.9);
    }

    #[test]
    fn test_subsystem_result_empty() {
        let result = SubsystemResult {
            category: DjangoTestCategory::Core,
            total: 0,
            passed: 0,
            failed: 0,
        };
        assert_eq!(result.pass_rate(), 0.0);
    }

    #[test]
    fn test_c_extension_status() {
        let status = CExtensionStatus::default();
        assert!(status.json_parsing);
        assert!(status.password_hashing);
        assert!(status.database_adapters);
        assert!(status.template_extensions);
    }

    #[test]
    fn test_parse_django_test_output() {
        let config = DjangoValidationConfig::default();
        let validator = DjangoValidator::new(config);

        let output = r#"
test_basic (tests.TestCase) ... ok
test_advanced (tests.TestCase) ... FAIL
Ran 10 tests in 1.234s
FAILED (failures=2, errors=1)
"#;

        let (total, passed, failed, _failures) = validator.parse_django_test_output(output);
        assert_eq!(total, 10);
        assert_eq!(failed, 3); // 2 failures + 1 error
        assert_eq!(passed, 7);
    }

    #[test]
    fn test_parse_django_test_output_success() {
        let config = DjangoValidationConfig::default();
        let validator = DjangoValidator::new(config);

        let output = r#"
test_basic (tests.TestCase) ... ok
test_advanced (tests.TestCase) ... ok
Ran 5 tests in 0.500s
OK
"#;

        let (total, passed, failed, failures) = validator.parse_django_test_output(output);
        assert_eq!(total, 5);
        assert_eq!(passed, 5);
        assert_eq!(failed, 0);
        assert!(failures.is_empty());
    }

    #[test]
    fn test_django_validation_error_display() {
        let err = DjangoValidationError::ProjectCreationFailed("test error".to_string());
        assert!(err.to_string().contains("test error"));

        let err = DjangoValidationError::DevServerFailed("server error".to_string());
        assert!(err.to_string().contains("server error"));

        let err = DjangoValidationError::Timeout;
        assert!(err.to_string().contains("Timeout"));
    }

    #[test]
    fn test_django_core_test_result() {
        let result = DjangoCoreTestResult {
            total: 100,
            passed: 95,
            failed: 5,
            output: "test output".to_string(),
            failures_by_category: HashMap::new(),
        };

        assert_eq!(result.total, 100);
        assert_eq!(result.passed, 95);
        assert_eq!(result.failed, 5);
    }
}
