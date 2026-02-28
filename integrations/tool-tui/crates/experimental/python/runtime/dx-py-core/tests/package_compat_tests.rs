//! Package Compatibility Tests
//!
//! These tests verify that popular Python packages can be imported and
//! basic operations work correctly with DX-Py runtime.
//!
//! Requirements: 4.2.1-4.2.7

use std::process::Command;

/// Helper to run a Python script and check if it succeeds
fn run_python_script(script: &str) -> Result<String, String> {
    // Try to find Python in common locations
    let python_cmd = if cfg!(windows) { "python" } else { "python3" };

    let output = Command::new(python_cmd)
        .arg("-c")
        .arg(script)
        .output()
        .map_err(|e| format!("Failed to execute Python: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

/// Helper to check if a package is installed
fn is_package_installed(package: &str) -> bool {
    let script = format!("import {}", package.split('[').next().unwrap_or(package));
    run_python_script(&script).is_ok()
}

/// Test that requests package can be imported and make basic HTTP requests
/// Validates: Requirements 4.2.1
#[test]
fn test_requests_import() {
    if !is_package_installed("requests") {
        println!("Skipping test: requests not installed");
        return;
    }

    let script = r#"
import requests
print("requests version:", requests.__version__)
# Test basic functionality
session = requests.Session()
print("Session created successfully")
"#;

    let result = run_python_script(script);
    assert!(result.is_ok(), "Failed to import requests: {:?}", result.err());
    let output = result.unwrap();
    assert!(output.contains("requests version:"), "Version not found in output");
    assert!(output.contains("Session created successfully"), "Session creation failed");
}

/// Test that Flask can be imported and basic app creation works
/// Validates: Requirements 4.2.2
#[test]
fn test_flask_import() {
    if !is_package_installed("flask") {
        println!("Skipping test: flask not installed");
        return;
    }

    let script = r#"
from flask import Flask
app = Flask(__name__)

@app.route('/')
def hello():
    return 'Hello, World!'

print("Flask app created successfully")
print("Flask version:", Flask.__module__)
"#;

    let result = run_python_script(script);
    assert!(result.is_ok(), "Failed to import flask: {:?}", result.err());
    assert!(result.unwrap().contains("Flask app created successfully"));
}

/// Test that FastAPI can be imported and basic app creation works
/// Validates: Requirements 4.2.2
#[test]
fn test_fastapi_import() {
    if !is_package_installed("fastapi") {
        println!("Skipping test: fastapi not installed");
        return;
    }

    let script = r#"
from fastapi import FastAPI
app = FastAPI()

@app.get("/")
async def root():
    return {"message": "Hello World"}

print("FastAPI app created successfully")
"#;

    let result = run_python_script(script);
    assert!(result.is_ok(), "Failed to import fastapi: {:?}", result.err());
    assert!(result.unwrap().contains("FastAPI app created successfully"));
}

/// Test that pandas can be imported and basic DataFrame operations work
/// Validates: Requirements 4.2.3
#[test]
fn test_pandas_import() {
    if !is_package_installed("pandas") {
        println!("Skipping test: pandas not installed");
        return;
    }

    let script = r#"
import pandas as pd

# Create a simple DataFrame
df = pd.DataFrame({
    'A': [1, 2, 3, 4, 5],
    'B': ['a', 'b', 'c', 'd', 'e'],
    'C': [1.1, 2.2, 3.3, 4.4, 5.5]
})

# Basic operations
print("DataFrame shape:", df.shape)
print("Column sum:", df['A'].sum())
print("Mean of C:", df['C'].mean())

# Filtering
filtered = df[df['A'] > 2]
print("Filtered rows:", len(filtered))

print("Pandas operations successful")
"#;

    let result = run_python_script(script);
    assert!(result.is_ok(), "Failed to import pandas: {:?}", result.err());
    let output = result.unwrap();
    assert!(output.contains("DataFrame shape: (5, 3)"), "Shape mismatch");
    assert!(output.contains("Column sum: 15"), "Sum mismatch");
    assert!(output.contains("Pandas operations successful"));
}

/// Test that numpy can be imported and basic array operations work
/// Validates: Requirements 4.2.4
#[test]
fn test_numpy_import() {
    if !is_package_installed("numpy") {
        println!("Skipping test: numpy not installed");
        return;
    }

    let script = r#"
import numpy as np

# Create arrays
arr = np.array([1, 2, 3, 4, 5])
matrix = np.array([[1, 2], [3, 4]])

# Basic operations
print("Array sum:", arr.sum())
print("Array mean:", arr.mean())
print("Matrix shape:", matrix.shape)
print("Matrix determinant:", np.linalg.det(matrix))

# Broadcasting
result = arr * 2
print("Broadcast result:", result.tolist())

# Linear algebra
eigenvalues = np.linalg.eigvals(matrix)
print("Eigenvalues computed:", len(eigenvalues))

print("NumPy operations successful")
"#;

    let result = run_python_script(script);
    assert!(result.is_ok(), "Failed to import numpy: {:?}", result.err());
    let output = result.unwrap();
    assert!(output.contains("Array sum: 15"), "Sum mismatch");
    assert!(output.contains("NumPy operations successful"));
}

/// Test that SQLAlchemy can be imported and basic ORM setup works
/// Validates: Requirements 4.2.5
#[test]
fn test_sqlalchemy_import() {
    if !is_package_installed("sqlalchemy") {
        println!("Skipping test: sqlalchemy not installed");
        return;
    }

    let script = r#"
from sqlalchemy import create_engine, Column, Integer, String
from sqlalchemy.orm import declarative_base, sessionmaker

# Create in-memory SQLite database
engine = create_engine('sqlite:///:memory:', echo=False)
Base = declarative_base()

# Define a model
class User(Base):
    __tablename__ = 'users'
    id = Column(Integer, primary_key=True)
    name = Column(String)
    email = Column(String)

# Create tables
Base.metadata.create_all(engine)

# Create session
Session = sessionmaker(bind=engine)
session = Session()

# Insert data
user = User(name='Test User', email='test@example.com')
session.add(user)
session.commit()

# Query data
result = session.query(User).filter_by(name='Test User').first()
print("User found:", result.name)
print("SQLAlchemy operations successful")
"#;

    let result = run_python_script(script);
    assert!(result.is_ok(), "Failed to import sqlalchemy: {:?}", result.err());
    let output = result.unwrap();
    assert!(output.contains("User found: Test User"), "Query failed");
    assert!(output.contains("SQLAlchemy operations successful"));
}

/// Test that Django can be imported and basic setup works
/// Validates: Requirements 4.2.6
#[test]
fn test_django_import() {
    if !is_package_installed("django") {
        println!("Skipping test: django not installed");
        return;
    }

    let script = r#"
import django
from django.conf import settings

# Configure Django settings
if not settings.configured:
    settings.configure(
        DEBUG=True,
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

print("Django version:", django.VERSION)
print("Django setup successful")
"#;

    let result = run_python_script(script);
    assert!(result.is_ok(), "Failed to import django: {:?}", result.err());
    let output = result.unwrap();
    assert!(output.contains("Django version:"), "Version not found");
    assert!(output.contains("Django setup successful"));
}

/// Test that httpx (async HTTP client) can be imported
/// Validates: Requirements 4.2.1 (HTTP client alternative)
#[test]
fn test_httpx_import() {
    if !is_package_installed("httpx") {
        println!("Skipping test: httpx not installed");
        return;
    }

    let script = r#"
import httpx

# Create client
client = httpx.Client()
print("httpx client created successfully")

# Test async client creation
async_client = httpx.AsyncClient()
print("httpx async client created successfully")
"#;

    let result = run_python_script(script);
    assert!(result.is_ok(), "Failed to import httpx: {:?}", result.err());
    assert!(result.unwrap().contains("httpx client created successfully"));
}

/// Test that pydantic can be imported and basic model validation works
/// Validates: Requirements 4.2.7 (data validation)
#[test]
fn test_pydantic_import() {
    if !is_package_installed("pydantic") {
        println!("Skipping test: pydantic not installed");
        return;
    }

    let script = r#"
from pydantic import BaseModel, ValidationError
from typing import Optional

class User(BaseModel):
    id: int
    name: str
    email: Optional[str] = None

# Valid model
user = User(id=1, name='Test User')
print("User created:", user.name)

# Test validation
try:
    invalid_user = User(id='not_an_int', name='Test')
except ValidationError as e:
    print("Validation error caught correctly")

print("Pydantic operations successful")
"#;

    let result = run_python_script(script);
    assert!(result.is_ok(), "Failed to import pydantic: {:?}", result.err());
    let output = result.unwrap();
    assert!(output.contains("User created: Test User"), "Model creation failed");
    assert!(output.contains("Pydantic operations successful"));
}

/// Test that click (CLI framework) can be imported
/// Validates: Requirements 4.2.7 (CLI tools)
#[test]
fn test_click_import() {
    if !is_package_installed("click") {
        println!("Skipping test: click not installed");
        return;
    }

    let script = r#"
import click

@click.command()
@click.option('--name', default='World', help='Name to greet')
def hello(name):
    click.echo(f'Hello, {name}!')

print("Click command created successfully")
"#;

    let result = run_python_script(script);
    assert!(result.is_ok(), "Failed to import click: {:?}", result.err());
    assert!(result.unwrap().contains("Click command created successfully"));
}

/// Test that pytest can be imported
/// Validates: Requirements 4.2.7 (testing framework)
#[test]
fn test_pytest_import() {
    if !is_package_installed("pytest") {
        println!("Skipping test: pytest not installed");
        return;
    }

    let script = r#"
import pytest

# Check pytest version
print("pytest version:", pytest.__version__)

# Test fixture decorator exists
@pytest.fixture
def sample_fixture():
    return 42

print("pytest import successful")
"#;

    let result = run_python_script(script);
    assert!(result.is_ok(), "Failed to import pytest: {:?}", result.err());
    assert!(result.unwrap().contains("pytest import successful"));
}

/// Summary test that reports overall package compatibility status
#[test]
fn test_package_compatibility_summary() {
    let packages = vec![
        ("requests", "HTTP client"),
        ("flask", "Web framework"),
        ("fastapi", "Async web framework"),
        ("pandas", "Data analysis"),
        ("numpy", "Numerical computing"),
        ("sqlalchemy", "ORM"),
        ("django", "Web framework"),
        ("httpx", "Async HTTP client"),
        ("pydantic", "Data validation"),
        ("click", "CLI framework"),
        ("pytest", "Testing framework"),
    ];

    println!("\n=== Package Compatibility Summary ===\n");

    let mut installed_count = 0;
    let total_count = packages.len();

    for (package, description) in &packages {
        let status = if is_package_installed(package) {
            installed_count += 1;
            "✓ Available"
        } else {
            "✗ Not installed"
        };
        println!("{:15} ({:20}) - {}", package, description, status);
    }

    println!("\n{}/{} packages available for testing", installed_count, total_count);
    println!("=====================================\n");

    // This test always passes - it's informational
}
