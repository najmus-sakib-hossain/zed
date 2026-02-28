//! Real-World Benchmark Infrastructure
//!
//! Provides infrastructure for running identical workloads on DX-Py and CPython
//! to measure timing, memory, and throughput for real-world frameworks.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use std::time::{Duration, Instant};
use thiserror::Error;

/// Errors that can occur during benchmark execution
#[derive(Debug, Error)]
pub enum BenchmarkError {
    #[error("Failed to execute benchmark: {0}")]
    ExecutionFailed(String),

    #[error("Benchmark timed out after {0:?}")]
    Timeout(Duration),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Python interpreter not found: {0}")]
    InterpreterNotFound(String),

    #[error("Invalid benchmark configuration: {0}")]
    InvalidConfig(String),
}

/// Configuration for benchmark execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    /// Number of warmup iterations
    pub warmup_iterations: u32,
    /// Number of measurement iterations
    pub measurement_iterations: u32,
    /// Timeout for each benchmark run
    pub timeout: Duration,
    /// DX-Py interpreter path
    pub dxpy_interpreter: String,
    /// CPython interpreter path
    pub cpython_interpreter: String,
    /// Whether to measure memory usage
    pub measure_memory: bool,
    /// Whether to measure throughput (for web frameworks)
    pub measure_throughput: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            warmup_iterations: 3,
            measurement_iterations: 10,
            timeout: Duration::from_secs(300),
            dxpy_interpreter: "dx-py".to_string(),
            cpython_interpreter: "python3".to_string(),
            measure_memory: true,
            measure_throughput: true,
        }
    }
}

impl BenchmarkConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_warmup(mut self, iterations: u32) -> Self {
        self.warmup_iterations = iterations;
        self
    }

    pub fn with_measurements(mut self, iterations: u32) -> Self {
        self.measurement_iterations = iterations;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_dxpy_interpreter(mut self, path: impl Into<String>) -> Self {
        self.dxpy_interpreter = path.into();
        self
    }

    pub fn with_cpython_interpreter(mut self, path: impl Into<String>) -> Self {
        self.cpython_interpreter = path.into();
        self
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), BenchmarkError> {
        if self.measurement_iterations == 0 {
            return Err(BenchmarkError::InvalidConfig(
                "measurement_iterations must be > 0".to_string(),
            ));
        }
        if self.timeout.is_zero() {
            return Err(BenchmarkError::InvalidConfig("timeout must be > 0".to_string()));
        }
        Ok(())
    }
}

/// Metrics from a single benchmark run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkMetrics {
    /// Duration of the benchmark in milliseconds
    pub duration_ms: f64,
    /// Memory usage in megabytes (if measured)
    pub memory_mb: Option<f64>,
    /// Throughput in operations per second (if applicable)
    pub throughput: Option<f64>,
    /// Additional custom metrics
    pub custom_metrics: HashMap<String, f64>,
}

impl BenchmarkMetrics {
    pub fn new(duration_ms: f64) -> Self {
        Self {
            duration_ms,
            memory_mb: None,
            throughput: None,
            custom_metrics: HashMap::new(),
        }
    }

    pub fn with_memory(mut self, memory_mb: f64) -> Self {
        self.memory_mb = Some(memory_mb);
        self
    }

    pub fn with_throughput(mut self, throughput: f64) -> Self {
        self.throughput = Some(throughput);
        self
    }

    pub fn with_custom(mut self, name: impl Into<String>, value: f64) -> Self {
        self.custom_metrics.insert(name.into(), value);
        self
    }
}

/// Result of a real-world benchmark comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealWorldBenchmark {
    /// Framework being benchmarked
    pub framework: String,
    /// Workload description
    pub workload: String,
    /// DX-Py benchmark results
    pub dxpy_result: BenchmarkMetrics,
    /// CPython benchmark results
    pub cpython_result: BenchmarkMetrics,
    /// Speedup factor (dxpy_time / cpython_time, >1 means DX-Py is slower)
    pub speedup: f64,
    /// Memory ratio (dxpy_memory / cpython_memory)
    pub memory_ratio: Option<f64>,
    /// Throughput ratio (dxpy_throughput / cpython_throughput)
    pub throughput_ratio: Option<f64>,
    /// Timestamp of the benchmark
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl RealWorldBenchmark {
    pub fn new(
        framework: impl Into<String>,
        workload: impl Into<String>,
        dxpy_result: BenchmarkMetrics,
        cpython_result: BenchmarkMetrics,
    ) -> Self {
        let speedup = cpython_result.duration_ms / dxpy_result.duration_ms;
        let memory_ratio = match (dxpy_result.memory_mb, cpython_result.memory_mb) {
            (Some(dx), Some(cp)) if cp > 0.0 => Some(dx / cp),
            _ => None,
        };
        let throughput_ratio = match (dxpy_result.throughput, cpython_result.throughput) {
            (Some(dx), Some(cp)) if cp > 0.0 => Some(dx / cp),
            _ => None,
        };

        Self {
            framework: framework.into(),
            workload: workload.into(),
            dxpy_result,
            cpython_result,
            speedup,
            memory_ratio,
            throughput_ratio,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Check if DX-Py is faster than CPython
    pub fn is_dxpy_faster(&self) -> bool {
        self.speedup > 1.0
    }

    /// Get speedup as a percentage (positive = faster, negative = slower)
    pub fn speedup_percentage(&self) -> f64 {
        (self.speedup - 1.0) * 100.0
    }
}

/// Benchmark runner for real-world framework benchmarks
pub struct BenchmarkRunner {
    config: BenchmarkConfig,
}

impl BenchmarkRunner {
    pub fn new(config: BenchmarkConfig) -> Self {
        Self { config }
    }

    /// Run a benchmark on both DX-Py and CPython
    pub fn run_comparison(
        &self,
        framework: &str,
        workload: &str,
        script: &str,
    ) -> Result<RealWorldBenchmark, BenchmarkError> {
        self.config.validate()?;

        // Run on DX-Py
        let dxpy_result = self.run_benchmark(&self.config.dxpy_interpreter, script)?;

        // Run on CPython
        let cpython_result = self.run_benchmark(&self.config.cpython_interpreter, script)?;

        Ok(RealWorldBenchmark::new(framework, workload, dxpy_result, cpython_result))
    }

    /// Run a benchmark on a specific interpreter
    fn run_benchmark(
        &self,
        interpreter: &str,
        script: &str,
    ) -> Result<BenchmarkMetrics, BenchmarkError> {
        let mut timings = Vec::with_capacity(self.config.measurement_iterations as usize);
        let start_time = Instant::now();

        // Warmup phase
        for _ in 0..self.config.warmup_iterations {
            if start_time.elapsed() > self.config.timeout {
                return Err(BenchmarkError::Timeout(self.config.timeout));
            }
            let _output = self.execute_script(interpreter, script)?;
        }

        // Measurement phase
        for _ in 0..self.config.measurement_iterations {
            if start_time.elapsed() > self.config.timeout {
                return Err(BenchmarkError::Timeout(self.config.timeout));
            }

            let iter_start = Instant::now();
            let _output = self.execute_script(interpreter, script)?;
            let elapsed = iter_start.elapsed();
            timings.push(elapsed.as_secs_f64() * 1000.0); // Convert to ms

            // Parse memory from output if available
            if self.config.measure_memory {
                // Memory measurement would be parsed from script output
                // For now, we'll rely on the script to report memory
            }
        }

        // Calculate mean duration
        let mean_duration = timings.iter().sum::<f64>() / timings.len() as f64;

        Ok(BenchmarkMetrics::new(mean_duration))
    }

    /// Execute a Python script and return output
    fn execute_script(&self, interpreter: &str, script: &str) -> Result<String, BenchmarkError> {
        let output = std::process::Command::new(interpreter)
            .args(["-c", script])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    BenchmarkError::InterpreterNotFound(interpreter.to_string())
                } else {
                    BenchmarkError::IoError(e)
                }
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BenchmarkError::ExecutionFailed(stderr.to_string()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Run a throughput benchmark (for web frameworks)
    pub fn run_throughput_benchmark(
        &self,
        framework: &str,
        workload: &str,
        setup_script: &str,
        request_script: &str,
        num_requests: u32,
    ) -> Result<RealWorldBenchmark, BenchmarkError> {
        self.config.validate()?;

        // Run on DX-Py
        let dxpy_result = self.run_throughput_test(
            &self.config.dxpy_interpreter,
            setup_script,
            request_script,
            num_requests,
        )?;

        // Run on CPython
        let cpython_result = self.run_throughput_test(
            &self.config.cpython_interpreter,
            setup_script,
            request_script,
            num_requests,
        )?;

        Ok(RealWorldBenchmark::new(framework, workload, dxpy_result, cpython_result))
    }

    /// Run a throughput test
    fn run_throughput_test(
        &self,
        interpreter: &str,
        _setup_script: &str,
        request_script: &str,
        num_requests: u32,
    ) -> Result<BenchmarkMetrics, BenchmarkError> {
        let start = Instant::now();

        // Execute the request script multiple times
        for _ in 0..num_requests {
            self.execute_script(interpreter, request_script)?;
        }

        let duration = start.elapsed();
        let duration_ms = duration.as_secs_f64() * 1000.0;
        let throughput = num_requests as f64 / duration.as_secs_f64();

        Ok(BenchmarkMetrics::new(duration_ms).with_throughput(throughput))
    }
}

/// Django-specific benchmark suite
pub struct DjangoBenchmarks {
    runner: BenchmarkRunner,
}

impl DjangoBenchmarks {
    pub fn new(config: BenchmarkConfig) -> Self {
        Self {
            runner: BenchmarkRunner::new(config),
        }
    }

    /// Benchmark Django request latency
    pub fn benchmark_request_latency(&self) -> Result<RealWorldBenchmark, BenchmarkError> {
        let script = r#"
import django
from django.conf import settings
from django.test import RequestFactory
from django.http import HttpResponse

if not settings.configured:
    settings.configure(
        DEBUG=False,
        ROOT_URLCONF='',
        MIDDLEWARE=[],
        TEMPLATES=[],
    )
    django.setup()

def view(request):
    return HttpResponse("Hello, World!")

factory = RequestFactory()
for _ in range(100):
    request = factory.get('/')
    response = view(request)
    assert response.status_code == 200
"#;

        self.runner.run_comparison("Django", "request_latency", script)
    }

    /// Benchmark Django ORM operations
    pub fn benchmark_orm_operations(&self) -> Result<RealWorldBenchmark, BenchmarkError> {
        let script = r#"
import django
from django.conf import settings

if not settings.configured:
    settings.configure(
        DATABASES={'default': {'ENGINE': 'django.db.backends.sqlite3', 'NAME': ':memory:'}},
        INSTALLED_APPS=['django.contrib.contenttypes', 'django.contrib.auth'],
        DEFAULT_AUTO_FIELD='django.db.models.BigAutoField',
    )
    django.setup()

from django.contrib.auth.models import User
from django.db import connection

# Create tables
with connection.schema_editor() as schema_editor:
    schema_editor.create_model(User)

# Benchmark ORM operations
for i in range(100):
    user = User.objects.create_user(f'user{i}', f'user{i}@test.com', 'password')
    User.objects.filter(username=f'user{i}').first()
    user.delete()
"#;

        self.runner.run_comparison("Django", "orm_operations", script)
    }

    /// Benchmark Django template rendering
    pub fn benchmark_template_rendering(&self) -> Result<RealWorldBenchmark, BenchmarkError> {
        let script = r#"
import django
from django.conf import settings

if not settings.configured:
    settings.configure(
        TEMPLATES=[{
            'BACKEND': 'django.template.backends.django.DjangoTemplates',
            'DIRS': [],
            'APP_DIRS': False,
            'OPTIONS': {'context_processors': []},
        }],
    )
    django.setup()

from django.template import Template, Context

template = Template('''
{% for item in items %}
<div class="item">
    <h2>{{ item.title }}</h2>
    <p>{{ item.description }}</p>
    {% if item.active %}<span class="active">Active</span>{% endif %}
</div>
{% endfor %}
''')

items = [{'title': f'Item {i}', 'description': f'Description {i}', 'active': i % 2 == 0} for i in range(100)]
context = Context({'items': items})

for _ in range(100):
    result = template.render(context)
"#;

        self.runner.run_comparison("Django", "template_rendering", script)
    }

    /// Run all Django benchmarks
    pub fn run_all(&self) -> Vec<Result<RealWorldBenchmark, BenchmarkError>> {
        vec![
            self.benchmark_request_latency(),
            self.benchmark_orm_operations(),
            self.benchmark_template_rendering(),
        ]
    }
}

/// NumPy-specific benchmark suite
pub struct NumpyBenchmarks {
    runner: BenchmarkRunner,
}

impl NumpyBenchmarks {
    pub fn new(config: BenchmarkConfig) -> Self {
        Self {
            runner: BenchmarkRunner::new(config),
        }
    }

    /// Benchmark NumPy array creation
    pub fn benchmark_array_creation(&self) -> Result<RealWorldBenchmark, BenchmarkError> {
        let script = r#"
import numpy as np

for _ in range(1000):
    a = np.zeros((100, 100))
    b = np.ones((100, 100))
    c = np.arange(10000).reshape(100, 100)
    d = np.random.rand(100, 100)
"#;

        self.runner.run_comparison("NumPy", "array_creation", script)
    }

    /// Benchmark NumPy arithmetic operations
    pub fn benchmark_arithmetic(&self) -> Result<RealWorldBenchmark, BenchmarkError> {
        let script = r#"
import numpy as np

a = np.random.rand(1000, 1000)
b = np.random.rand(1000, 1000)

for _ in range(100):
    c = a + b
    d = a * b
    e = a - b
    f = a / (b + 0.001)
"#;

        self.runner.run_comparison("NumPy", "arithmetic_operations", script)
    }

    /// Benchmark NumPy linear algebra
    pub fn benchmark_linear_algebra(&self) -> Result<RealWorldBenchmark, BenchmarkError> {
        let script = r#"
import numpy as np

a = np.random.rand(100, 100)
b = np.random.rand(100, 100)

for _ in range(100):
    c = np.dot(a, b)
    d = np.matmul(a, b)
    e = a @ b
"#;

        self.runner.run_comparison("NumPy", "linear_algebra", script)
    }

    /// Benchmark NumPy slicing and indexing
    pub fn benchmark_slicing(&self) -> Result<RealWorldBenchmark, BenchmarkError> {
        let script = r#"
import numpy as np

a = np.random.rand(1000, 1000)

for _ in range(1000):
    b = a[100:200, 300:400]
    c = a[::2, ::2]
    d = a[a > 0.5]
    e = a[:, 0]
"#;

        self.runner.run_comparison("NumPy", "slicing_indexing", script)
    }

    /// Benchmark NumPy broadcasting
    pub fn benchmark_broadcasting(&self) -> Result<RealWorldBenchmark, BenchmarkError> {
        let script = r#"
import numpy as np

a = np.random.rand(1000, 1000)
b = np.random.rand(1000)
c = np.random.rand(1000, 1)

for _ in range(100):
    d = a + b
    e = a * c
    f = a + b.reshape(1, -1)
"#;

        self.runner.run_comparison("NumPy", "broadcasting", script)
    }

    /// Run all NumPy benchmarks
    pub fn run_all(&self) -> Vec<Result<RealWorldBenchmark, BenchmarkError>> {
        vec![
            self.benchmark_array_creation(),
            self.benchmark_arithmetic(),
            self.benchmark_linear_algebra(),
            self.benchmark_slicing(),
            self.benchmark_broadcasting(),
        ]
    }
}

/// Pandas-specific benchmark suite
pub struct PandasBenchmarks {
    runner: BenchmarkRunner,
}

impl PandasBenchmarks {
    pub fn new(config: BenchmarkConfig) -> Self {
        Self {
            runner: BenchmarkRunner::new(config),
        }
    }

    /// Benchmark Pandas DataFrame creation
    pub fn benchmark_dataframe_creation(&self) -> Result<RealWorldBenchmark, BenchmarkError> {
        let script = r#"
import pandas as pd
import numpy as np

for _ in range(100):
    df = pd.DataFrame({
        'A': np.random.rand(10000),
        'B': np.random.rand(10000),
        'C': np.random.randint(0, 100, 10000),
        'D': ['cat' if x < 0.5 else 'dog' for x in np.random.rand(10000)],
    })
"#;

        self.runner.run_comparison("Pandas", "dataframe_creation", script)
    }

    /// Benchmark Pandas groupby operations
    pub fn benchmark_groupby(&self) -> Result<RealWorldBenchmark, BenchmarkError> {
        let script = r#"
import pandas as pd
import numpy as np

df = pd.DataFrame({
    'A': np.random.rand(100000),
    'B': np.random.rand(100000),
    'C': np.random.randint(0, 100, 100000),
    'D': np.random.choice(['cat', 'dog', 'bird'], 100000),
})

for _ in range(100):
    result = df.groupby('D').agg({'A': 'mean', 'B': 'sum', 'C': 'count'})
    result2 = df.groupby(['D', 'C']).mean()
"#;

        self.runner.run_comparison("Pandas", "groupby_operations", script)
    }

    /// Benchmark Pandas merge operations
    pub fn benchmark_merge(&self) -> Result<RealWorldBenchmark, BenchmarkError> {
        let script = r#"
import pandas as pd
import numpy as np

df1 = pd.DataFrame({
    'key': np.arange(10000),
    'A': np.random.rand(10000),
})

df2 = pd.DataFrame({
    'key': np.arange(10000),
    'B': np.random.rand(10000),
})

for _ in range(100):
    result = pd.merge(df1, df2, on='key')
    result2 = df1.merge(df2, on='key', how='left')
"#;

        self.runner.run_comparison("Pandas", "merge_operations", script)
    }

    /// Benchmark Pandas I/O operations
    pub fn benchmark_io(&self) -> Result<RealWorldBenchmark, BenchmarkError> {
        let script = r#"
import pandas as pd
import numpy as np
import tempfile
import os

df = pd.DataFrame({
    'A': np.random.rand(10000),
    'B': np.random.rand(10000),
    'C': np.random.randint(0, 100, 10000),
})

with tempfile.TemporaryDirectory() as tmpdir:
    csv_path = os.path.join(tmpdir, 'test.csv')
    json_path = os.path.join(tmpdir, 'test.json')
    
    for _ in range(10):
        df.to_csv(csv_path, index=False)
        df_read = pd.read_csv(csv_path)
        
        df.to_json(json_path)
        df_read = pd.read_json(json_path)
"#;

        self.runner.run_comparison("Pandas", "io_operations", script)
    }

    /// Benchmark Pandas pivot operations
    pub fn benchmark_pivot(&self) -> Result<RealWorldBenchmark, BenchmarkError> {
        let script = r#"
import pandas as pd
import numpy as np

df = pd.DataFrame({
    'date': pd.date_range('2020-01-01', periods=1000),
    'category': np.random.choice(['A', 'B', 'C'], 1000),
    'value': np.random.rand(1000),
})

for _ in range(100):
    pivot = df.pivot_table(values='value', index='date', columns='category', aggfunc='mean')
"#;

        self.runner.run_comparison("Pandas", "pivot_operations", script)
    }

    /// Run all Pandas benchmarks
    pub fn run_all(&self) -> Vec<Result<RealWorldBenchmark, BenchmarkError>> {
        vec![
            self.benchmark_dataframe_creation(),
            self.benchmark_groupby(),
            self.benchmark_merge(),
            self.benchmark_io(),
            self.benchmark_pivot(),
        ]
    }
}

/// Benchmark report generator
pub struct BenchmarkReportGenerator {
    results: Vec<RealWorldBenchmark>,
}

impl BenchmarkReportGenerator {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    pub fn add_result(&mut self, result: RealWorldBenchmark) {
        self.results.push(result);
    }

    pub fn add_results(&mut self, results: Vec<RealWorldBenchmark>) {
        self.results.extend(results);
    }

    /// Generate a markdown report
    pub fn generate_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str("# Real-World Benchmark Report\n\n");
        md.push_str(&format!(
            "Generated: {}\n\n",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));

        // Group by framework
        let mut by_framework: HashMap<&str, Vec<&RealWorldBenchmark>> = HashMap::new();
        for result in &self.results {
            by_framework.entry(&result.framework).or_default().push(result);
        }

        for (framework, results) in by_framework {
            md.push_str(&format!("## {}\n\n", framework));
            md.push_str("| Workload | DX-Py (ms) | CPython (ms) | Speedup | Status |\n");
            md.push_str("|----------|------------|--------------|---------|--------|\n");

            for result in results {
                let status = if result.is_dxpy_faster() {
                    "✅ Faster"
                } else if result.speedup > 0.9 {
                    "⚠️ Similar"
                } else {
                    "❌ Slower"
                };

                md.push_str(&format!(
                    "| {} | {:.2} | {:.2} | {:.2}x | {} |\n",
                    result.workload,
                    result.dxpy_result.duration_ms,
                    result.cpython_result.duration_ms,
                    result.speedup,
                    status
                ));
            }
            md.push('\n');
        }

        // Summary
        md.push_str("## Summary\n\n");
        let faster_count = self.results.iter().filter(|r| r.is_dxpy_faster()).count();
        let total = self.results.len();
        md.push_str(&format!(
            "- DX-Py faster in {} of {} benchmarks ({:.1}%)\n",
            faster_count,
            total,
            (faster_count as f64 / total as f64) * 100.0
        ));

        let avg_speedup: f64 = self.results.iter().map(|r| r.speedup).sum::<f64>() / total as f64;
        md.push_str(&format!("- Average speedup: {:.2}x\n", avg_speedup));

        md
    }

    /// Generate a JSON report
    pub fn generate_json(&self) -> String {
        serde_json::to_string_pretty(&self.results).unwrap_or_else(|_| "[]".to_string())
    }

    /// Get all results
    pub fn results(&self) -> &[RealWorldBenchmark] {
        &self.results
    }
}

impl Default for BenchmarkReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Validate benchmark metrics
pub fn validate_benchmark_metrics(benchmark: &RealWorldBenchmark) -> BenchmarkValidation {
    let mut validation = BenchmarkValidation {
        is_valid: true,
        has_timing: true,
        has_cpython_comparison: true,
        has_appropriate_metrics: true,
        warnings: Vec::new(),
        errors: Vec::new(),
    };

    // Check timing measurements
    if benchmark.dxpy_result.duration_ms <= 0.0 {
        validation.is_valid = false;
        validation.has_timing = false;
        validation.errors.push("DX-Py duration is invalid".to_string());
    }

    if benchmark.cpython_result.duration_ms <= 0.0 {
        validation.is_valid = false;
        validation.has_timing = false;
        validation.errors.push("CPython duration is invalid".to_string());
    }

    // Check for appropriate metrics based on framework type
    let is_web_framework = matches!(benchmark.framework.as_str(), "Django" | "Flask" | "FastAPI");

    if is_web_framework {
        // Web frameworks should have throughput for request benchmarks
        if (benchmark.workload.contains("request") || benchmark.workload.contains("latency"))
            && benchmark.dxpy_result.throughput.is_none()
        {
            validation
                .warnings
                .push("Web framework benchmark missing throughput metric".to_string());
        }
    }

    // Check for reasonable speedup values
    if benchmark.speedup < 0.01 || benchmark.speedup > 100.0 {
        validation.warnings.push(format!(
            "Unusual speedup value: {:.2}x - may indicate measurement error",
            benchmark.speedup
        ));
    }

    validation
}

/// Validation result for benchmark metrics
#[derive(Debug, Clone)]
pub struct BenchmarkValidation {
    pub is_valid: bool,
    pub has_timing: bool,
    pub has_cpython_comparison: bool,
    pub has_appropriate_metrics: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_config_defaults() {
        let config = BenchmarkConfig::default();
        assert_eq!(config.warmup_iterations, 3);
        assert_eq!(config.measurement_iterations, 10);
        assert!(config.measure_memory);
        assert!(config.measure_throughput);
    }

    #[test]
    fn test_benchmark_config_validation() {
        let config = BenchmarkConfig::default();
        assert!(config.validate().is_ok());

        let invalid_config = BenchmarkConfig {
            measurement_iterations: 0,
            ..Default::default()
        };
        assert!(invalid_config.validate().is_err());
    }

    #[test]
    fn test_benchmark_metrics() {
        let metrics = BenchmarkMetrics::new(100.0)
            .with_memory(50.0)
            .with_throughput(1000.0)
            .with_custom("custom_metric", 42.0);

        assert_eq!(metrics.duration_ms, 100.0);
        assert_eq!(metrics.memory_mb, Some(50.0));
        assert_eq!(metrics.throughput, Some(1000.0));
        assert_eq!(metrics.custom_metrics.get("custom_metric"), Some(&42.0));
    }

    #[test]
    fn test_real_world_benchmark() {
        let dxpy = BenchmarkMetrics::new(80.0);
        let cpython = BenchmarkMetrics::new(100.0);

        let benchmark = RealWorldBenchmark::new("Django", "request_latency", dxpy, cpython);

        assert!(benchmark.is_dxpy_faster());
        assert_eq!(benchmark.speedup, 1.25); // 100/80 = 1.25
        assert_eq!(benchmark.speedup_percentage(), 25.0);
    }

    #[test]
    fn test_real_world_benchmark_slower() {
        let dxpy = BenchmarkMetrics::new(150.0);
        let cpython = BenchmarkMetrics::new(100.0);

        let benchmark = RealWorldBenchmark::new("NumPy", "array_ops", dxpy, cpython);

        assert!(!benchmark.is_dxpy_faster());
        assert!(benchmark.speedup < 1.0);
    }

    #[test]
    fn test_benchmark_validation() {
        let dxpy = BenchmarkMetrics::new(100.0);
        let cpython = BenchmarkMetrics::new(100.0);
        let benchmark = RealWorldBenchmark::new("Django", "test", dxpy, cpython);

        let validation = validate_benchmark_metrics(&benchmark);
        assert!(validation.is_valid);
        assert!(validation.has_timing);
        assert!(validation.has_cpython_comparison);
    }

    #[test]
    fn test_benchmark_validation_invalid() {
        let dxpy = BenchmarkMetrics::new(-1.0);
        let cpython = BenchmarkMetrics::new(100.0);
        let benchmark = RealWorldBenchmark::new("Django", "test", dxpy, cpython);

        let validation = validate_benchmark_metrics(&benchmark);
        assert!(!validation.is_valid);
        assert!(!validation.has_timing);
    }

    #[test]
    fn test_report_generator() {
        let mut generator = BenchmarkReportGenerator::new();

        let dxpy = BenchmarkMetrics::new(80.0);
        let cpython = BenchmarkMetrics::new(100.0);
        let benchmark = RealWorldBenchmark::new("Django", "request_latency", dxpy, cpython);

        generator.add_result(benchmark);

        let markdown = generator.generate_markdown();
        assert!(markdown.contains("Django"));
        assert!(markdown.contains("request_latency"));
        assert!(markdown.contains("Faster"));

        let json = generator.generate_json();
        assert!(json.contains("Django"));
    }
}
