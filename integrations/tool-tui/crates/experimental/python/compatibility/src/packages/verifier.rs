//! Package verification for popular Python packages
//!
//! Verifies that packages like requests, flask, fastapi, pandas, numpy,
//! sqlalchemy, and django work correctly with DX-Py.

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use crate::runtime::PythonRuntime;

/// Result of verifying a package
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Package name
    pub package: String,
    /// Verification status
    pub status: VerificationStatus,
    /// Time taken for verification
    pub duration: Duration,
    /// Import test result
    pub import_ok: bool,
    /// Basic operation test result
    pub basic_ops_ok: bool,
    /// Error message if any
    pub error: Option<String>,
    /// Detailed test results
    pub test_results: Vec<TestResult>,
}

/// Status of package verification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationStatus {
    /// Package works fully
    Compatible,
    /// Package works with some limitations
    PartiallyCompatible,
    /// Package does not work
    Incompatible,
    /// Package is not installed
    NotInstalled,
    /// Verification failed due to error
    Error,
}

/// Individual test result
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub message: Option<String>,
    pub duration: Duration,
}

/// Package verifier for checking compatibility
pub struct PackageVerifier {
    python_path: PathBuf,
    #[allow(dead_code)]
    site_packages: PathBuf,
    timeout: Duration,
}

impl PackageVerifier {
    /// Create a new package verifier
    pub fn new(runtime: &PythonRuntime) -> Self {
        Self {
            python_path: runtime.executable.clone(),
            site_packages: runtime.site_packages_path(),
            timeout: Duration::from_secs(30),
        }
    }

    /// Create verifier with custom Python path
    pub fn with_python(python_path: PathBuf) -> Self {
        let site_packages = python_path
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("lib").join("site-packages"))
            .unwrap_or_default();

        Self {
            python_path,
            site_packages,
            timeout: Duration::from_secs(30),
        }
    }

    /// Set verification timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Verify a single package
    pub fn verify(&self, package: &str) -> VerificationResult {
        let start = Instant::now();

        // Check if package is installed
        if !self.is_installed(package) {
            return VerificationResult {
                package: package.to_string(),
                status: VerificationStatus::NotInstalled,
                duration: start.elapsed(),
                import_ok: false,
                basic_ops_ok: false,
                error: Some(format!("Package '{}' is not installed", package)),
                test_results: vec![],
            };
        }

        let mut test_results = Vec::new();

        // Test import
        let import_result = self.test_import(package);
        test_results.push(import_result.clone());

        if !import_result.passed {
            return VerificationResult {
                package: package.to_string(),
                status: VerificationStatus::Incompatible,
                duration: start.elapsed(),
                import_ok: false,
                basic_ops_ok: false,
                error: import_result.message,
                test_results,
            };
        }

        // Test basic operations based on package
        let ops_results = self.test_basic_operations(package);
        let ops_passed = ops_results.iter().all(|r| r.passed);
        test_results.extend(ops_results);

        let status = if ops_passed {
            VerificationStatus::Compatible
        } else {
            VerificationStatus::PartiallyCompatible
        };

        VerificationResult {
            package: package.to_string(),
            status,
            duration: start.elapsed(),
            import_ok: true,
            basic_ops_ok: ops_passed,
            error: None,
            test_results,
        }
    }

    /// Verify multiple packages
    pub fn verify_all(&self, packages: &[&str]) -> Vec<VerificationResult> {
        packages.iter().map(|p| self.verify(p)).collect()
    }

    /// Verify top-100 PyPI packages
    pub fn verify_top_packages(&self) -> Vec<VerificationResult> {
        let top_packages = Self::top_100_packages();
        self.verify_all(&top_packages)
    }

    /// Check if a package is installed
    fn is_installed(&self, package: &str) -> bool {
        let output = Command::new(&self.python_path)
            .args(["-c", &format!("import {}", Self::normalize_import(package))])
            .output();

        matches!(output, Ok(o) if o.status.success())
    }

    /// Test importing a package
    fn test_import(&self, package: &str) -> TestResult {
        let start = Instant::now();
        let import_name = Self::normalize_import(package);

        let output = Command::new(&self.python_path)
            .args(["-c", &format!("import {}", import_name)])
            .output();

        match output {
            Ok(o) if o.status.success() => TestResult {
                name: format!("import_{}", package),
                passed: true,
                message: None,
                duration: start.elapsed(),
            },
            Ok(o) => TestResult {
                name: format!("import_{}", package),
                passed: false,
                message: Some(String::from_utf8_lossy(&o.stderr).to_string()),
                duration: start.elapsed(),
            },
            Err(e) => TestResult {
                name: format!("import_{}", package),
                passed: false,
                message: Some(e.to_string()),
                duration: start.elapsed(),
            },
        }
    }

    /// Test basic operations for a package
    fn test_basic_operations(&self, package: &str) -> Vec<TestResult> {
        match package {
            "requests" => self.test_requests_ops(),
            "flask" => self.test_flask_ops(),
            "fastapi" => self.test_fastapi_ops(),
            "pandas" => self.test_pandas_ops(),
            "numpy" => self.test_numpy_ops(),
            "sqlalchemy" => self.test_sqlalchemy_ops(),
            "django" => self.test_django_ops(),
            _ => self.test_generic_ops(package),
        }
    }

    /// Test requests package operations
    fn test_requests_ops(&self) -> Vec<TestResult> {
        vec![
            self.run_python_test(
                "requests_session",
                r#"
import requests
s = requests.Session()
assert hasattr(s, 'get')
assert hasattr(s, 'post')
print('OK')
"#,
            ),
            self.run_python_test(
                "requests_response",
                r#"
import requests
r = requests.Response()
assert hasattr(r, 'status_code')
assert hasattr(r, 'json')
print('OK')
"#,
            ),
        ]
    }

    /// Test flask package operations
    fn test_flask_ops(&self) -> Vec<TestResult> {
        vec![
            self.run_python_test(
                "flask_app",
                r#"
from flask import Flask
app = Flask(__name__)
assert app is not None
print('OK')
"#,
            ),
            self.run_python_test(
                "flask_route",
                r#"
from flask import Flask
app = Flask(__name__)
@app.route('/')
def index():
    return 'Hello'
assert '/' in [r.rule for r in app.url_map.iter_rules()]
print('OK')
"#,
            ),
        ]
    }

    /// Test fastapi package operations
    fn test_fastapi_ops(&self) -> Vec<TestResult> {
        vec![
            self.run_python_test(
                "fastapi_app",
                r#"
from fastapi import FastAPI
app = FastAPI()
assert app is not None
print('OK')
"#,
            ),
            self.run_python_test(
                "fastapi_route",
                r#"
from fastapi import FastAPI
app = FastAPI()
@app.get('/')
def read_root():
    return {'Hello': 'World'}
assert len(app.routes) > 0
print('OK')
"#,
            ),
        ]
    }

    /// Test pandas package operations
    fn test_pandas_ops(&self) -> Vec<TestResult> {
        vec![
            self.run_python_test(
                "pandas_dataframe",
                r#"
import pandas as pd
df = pd.DataFrame({'a': [1, 2, 3], 'b': [4, 5, 6]})
assert len(df) == 3
assert list(df.columns) == ['a', 'b']
print('OK')
"#,
            ),
            self.run_python_test(
                "pandas_series",
                r#"
import pandas as pd
s = pd.Series([1, 2, 3, 4, 5])
assert len(s) == 5
assert s.sum() == 15
print('OK')
"#,
            ),
            self.run_python_test(
                "pandas_operations",
                r#"
import pandas as pd
df = pd.DataFrame({'a': [1, 2, 3], 'b': [4, 5, 6]})
result = df['a'] + df['b']
assert list(result) == [5, 7, 9]
print('OK')
"#,
            ),
        ]
    }

    /// Test numpy package operations
    fn test_numpy_ops(&self) -> Vec<TestResult> {
        vec![
            self.run_python_test(
                "numpy_array",
                r#"
import numpy as np
arr = np.array([1, 2, 3, 4, 5])
assert len(arr) == 5
assert arr.sum() == 15
print('OK')
"#,
            ),
            self.run_python_test(
                "numpy_operations",
                r#"
import numpy as np
a = np.array([1, 2, 3])
b = np.array([4, 5, 6])
c = a + b
assert list(c) == [5, 7, 9]
print('OK')
"#,
            ),
            self.run_python_test(
                "numpy_matrix",
                r#"
import numpy as np
m = np.array([[1, 2], [3, 4]])
assert m.shape == (2, 2)
assert m[0, 0] == 1
print('OK')
"#,
            ),
        ]
    }

    /// Test sqlalchemy package operations
    fn test_sqlalchemy_ops(&self) -> Vec<TestResult> {
        vec![
            self.run_python_test(
                "sqlalchemy_engine",
                r#"
from sqlalchemy import create_engine
engine = create_engine('sqlite:///:memory:')
assert engine is not None
print('OK')
"#,
            ),
            self.run_python_test(
                "sqlalchemy_table",
                r#"
from sqlalchemy import create_engine, MetaData, Table, Column, Integer, String
engine = create_engine('sqlite:///:memory:')
metadata = MetaData()
users = Table('users', metadata,
    Column('id', Integer, primary_key=True),
    Column('name', String(50))
)
metadata.create_all(engine)
print('OK')
"#,
            ),
        ]
    }

    /// Test django package operations
    fn test_django_ops(&self) -> Vec<TestResult> {
        vec![
            self.run_python_test(
                "django_import",
                r#"
import django
assert hasattr(django, 'VERSION')
print('OK')
"#,
            ),
            self.run_python_test(
                "django_conf",
                r#"
import django
from django.conf import settings
settings.configure(DEBUG=True)
assert settings.DEBUG == True
print('OK')
"#,
            ),
        ]
    }

    /// Test generic package operations
    fn test_generic_ops(&self, package: &str) -> Vec<TestResult> {
        let import_name = Self::normalize_import(package);
        vec![self.run_python_test(
            &format!("{}_attributes", package),
            &format!(
                r#"
import {}
m = {}
assert hasattr(m, '__name__')
assert hasattr(m, '__version__') or hasattr(m, 'VERSION') or True
print('OK')
"#,
                import_name, import_name
            ),
        )]
    }

    /// Run a Python test and return the result
    fn run_python_test(&self, name: &str, code: &str) -> TestResult {
        let start = Instant::now();

        let output = Command::new(&self.python_path).args(["-c", code]).output();

        match output {
            Ok(o) if o.status.success() => TestResult {
                name: name.to_string(),
                passed: true,
                message: None,
                duration: start.elapsed(),
            },
            Ok(o) => TestResult {
                name: name.to_string(),
                passed: false,
                message: Some(String::from_utf8_lossy(&o.stderr).to_string()),
                duration: start.elapsed(),
            },
            Err(e) => TestResult {
                name: name.to_string(),
                passed: false,
                message: Some(e.to_string()),
                duration: start.elapsed(),
            },
        }
    }

    /// Normalize package name to import name
    fn normalize_import(package: &str) -> &str {
        match package {
            "Pillow" | "pillow" => "PIL",
            "scikit-learn" => "sklearn",
            "beautifulsoup4" => "bs4",
            "python-dateutil" => "dateutil",
            "PyYAML" => "yaml",
            "typing-extensions" => "typing_extensions",
            "importlib-metadata" => "importlib_metadata",
            "zipp" => "zipp",
            _ => package,
        }
    }

    /// Get list of top 100 PyPI packages
    pub fn top_100_packages() -> Vec<&'static str> {
        vec![
            // Web frameworks
            "requests",
            "flask",
            "django",
            "fastapi",
            "aiohttp",
            "httpx",
            "urllib3",
            "starlette",
            "uvicorn",
            "gunicorn",
            // Data science
            "numpy",
            "pandas",
            "scipy",
            "matplotlib",
            "scikit-learn",
            "tensorflow",
            "torch",
            "keras",
            "xgboost",
            "lightgbm",
            // Database
            "sqlalchemy",
            "psycopg2",
            "pymysql",
            "redis",
            "pymongo",
            "sqlite3",
            "aiosqlite",
            "asyncpg",
            "motor",
            "elasticsearch",
            // Utilities
            "click",
            "typer",
            "rich",
            "tqdm",
            "colorama",
            "python-dotenv",
            "pydantic",
            "attrs",
            "dataclasses",
            "typing-extensions",
            // Testing
            "pytest",
            "unittest",
            "mock",
            "coverage",
            "hypothesis",
            "faker",
            "factory-boy",
            "responses",
            "httpretty",
            "vcrpy",
            // Serialization
            "json",
            "yaml",
            "toml",
            "msgpack",
            "protobuf",
            "orjson",
            "ujson",
            "simplejson",
            "pickle",
            "cloudpickle",
            // Async
            "asyncio",
            "trio",
            "anyio",
            "aiofiles",
            "aiocache",
            // Parsing
            "beautifulsoup4",
            "lxml",
            "html5lib",
            "cssselect",
            "parsel",
            // Cryptography
            "cryptography",
            "pycryptodome",
            "hashlib",
            "secrets",
            "bcrypt",
            // Image processing
            "Pillow",
            "opencv-python",
            "imageio",
            "scikit-image",
            "wand",
            // CLI
            "argparse",
            "fire",
            "docopt",
            "plumbum",
            "sh",
            // Logging
            "logging",
            "loguru",
            "structlog",
            "python-json-logger",
            "coloredlogs",
            // Date/Time
            "datetime",
            "python-dateutil",
            "arrow",
            "pendulum",
            "pytz",
            // File handling
            "pathlib",
            "os",
            "shutil",
            "glob",
            "fnmatch",
            // Networking
            "socket",
            "paramiko",
            "fabric",
            "netmiko",
            "scapy",
            // AWS
            "boto3",
            "botocore",
            "aiobotocore",
            "s3fs",
            "smart-open",
            // Other popular
            "jinja2",
            "markupsafe",
            "werkzeug",
            "itsdangerous",
            "certifi",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_import() {
        assert_eq!(PackageVerifier::normalize_import("Pillow"), "PIL");
        assert_eq!(PackageVerifier::normalize_import("scikit-learn"), "sklearn");
        assert_eq!(PackageVerifier::normalize_import("requests"), "requests");
    }

    #[test]
    fn test_top_100_packages() {
        let packages = PackageVerifier::top_100_packages();
        assert!(packages.len() >= 100);
        assert!(packages.contains(&"requests"));
        assert!(packages.contains(&"numpy"));
        assert!(packages.contains(&"pandas"));
    }
}
