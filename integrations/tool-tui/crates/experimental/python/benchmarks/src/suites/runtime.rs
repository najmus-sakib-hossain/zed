//! Runtime benchmark suite (DX-Py vs CPython)

use crate::data::{DataSize, TestDataGenerator};
use serde::{Deserialize, Serialize};

/// Benchmark specification for runtime comparisons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSpec {
    pub name: String,
    pub cpython_code: String,
    pub dxpy_code: String,
    pub setup_code: Option<String>,
    pub teardown_code: Option<String>,
}

impl BenchmarkSpec {
    /// Create a new benchmark spec with identical code for both runtimes
    pub fn new(name: impl Into<String>, code: impl Into<String>) -> Self {
        let code = code.into();
        Self {
            name: name.into(),
            cpython_code: code.clone(),
            dxpy_code: code,
            setup_code: None,
            teardown_code: None,
        }
    }

    /// Create a benchmark spec with setup code
    pub fn with_setup(mut self, setup: impl Into<String>) -> Self {
        self.setup_code = Some(setup.into());
        self
    }

    /// Create a benchmark spec with teardown code
    pub fn with_teardown(mut self, teardown: impl Into<String>) -> Self {
        self.teardown_code = Some(teardown.into());
        self
    }
}

/// Runtime benchmark suite comparing DX-Py against CPython
pub struct RuntimeSuite {
    pub data_generator: TestDataGenerator,
}

impl RuntimeSuite {
    pub fn new(seed: u64) -> Self {
        Self {
            data_generator: TestDataGenerator::new(seed),
        }
    }

    /// Get all micro-benchmarks
    pub fn micro_benchmarks(&self) -> Vec<BenchmarkSpec> {
        vec![
            self.bench_int_arithmetic(),
            self.bench_string_operations(),
            self.bench_list_operations(),
            self.bench_dict_operations(),
        ]
    }

    /// Get all macro-benchmarks
    pub fn macro_benchmarks(&mut self) -> Vec<BenchmarkSpec> {
        vec![
            self.bench_json_parsing(),
            self.bench_file_io(),
            self.bench_http_handling(),
        ]
    }

    /// Get startup and memory benchmarks
    pub fn startup_memory_benchmarks(&self) -> Vec<BenchmarkSpec> {
        vec![self.bench_cold_startup(), self.bench_memory_usage()]
    }

    // Micro-benchmarks

    /// Integer arithmetic benchmark
    pub fn bench_int_arithmetic(&self) -> BenchmarkSpec {
        BenchmarkSpec::new(
            "int_arithmetic",
            r#"
result = 0
for i in range(10000):
    result += i * 2 - i // 3 + i % 7
    result = result ^ (i << 2)
    result = result & 0xFFFFFFFF
"#,
        )
    }

    /// String operations benchmark
    pub fn bench_string_operations(&self) -> BenchmarkSpec {
        BenchmarkSpec::new(
            "string_operations",
            r#"
s = "hello world " * 100
result = s.upper()
result = result.lower()
result = result.replace("world", "python")
parts = result.split()
result = "-".join(parts)
result = result.strip()
"#,
        )
    }

    /// List operations benchmark
    pub fn bench_list_operations(&self) -> BenchmarkSpec {
        BenchmarkSpec::new(
            "list_operations",
            r#"
lst = list(range(1000))
lst.reverse()
lst.sort()
lst.append(1001)
lst.insert(500, 999)
lst.pop()
lst.remove(500)
result = [x * 2 for x in lst if x % 2 == 0]
"#,
        )
    }

    /// Dictionary operations benchmark
    pub fn bench_dict_operations(&self) -> BenchmarkSpec {
        BenchmarkSpec::new(
            "dict_operations",
            r#"
d = {str(i): i * 2 for i in range(1000)}
for i in range(100):
    d[f"new_{i}"] = i * 3
keys = list(d.keys())
values = list(d.values())
items = list(d.items())
result = {k: v for k, v in d.items() if v % 2 == 0}
"#,
        )
    }

    // Macro-benchmarks

    /// JSON parsing benchmark
    pub fn bench_json_parsing(&mut self) -> BenchmarkSpec {
        let json_data = self.data_generator.generate_json_data(DataSize::Medium);
        let escaped_json =
            json_data.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");

        BenchmarkSpec::new(
            "json_parsing",
            format!(
                r#"
import json
data = "{}"
for _ in range(100):
    parsed = json.loads(data)
    serialized = json.dumps(parsed)
"#,
                escaped_json
            ),
        )
    }

    /// File I/O benchmark
    pub fn bench_file_io(&self) -> BenchmarkSpec {
        BenchmarkSpec::new(
            "file_io",
            r#"
import tempfile
import os

with tempfile.NamedTemporaryFile(mode='w', delete=False) as f:
    fname = f.name
    for i in range(1000):
        f.write(f"Line {i}: " + "x" * 100 + "\n")

with open(fname, 'r') as f:
    lines = f.readlines()

os.unlink(fname)
"#,
        )
    }

    /// HTTP handling benchmark (simulated)
    pub fn bench_http_handling(&self) -> BenchmarkSpec {
        BenchmarkSpec::new(
            "http_handling",
            r#"
# Simulate HTTP request/response handling without actual network
headers = {
    "Content-Type": "application/json",
    "Accept": "application/json",
    "User-Agent": "BenchmarkClient/1.0",
}

for _ in range(1000):
    # Parse headers
    parsed = {k.lower(): v for k, v in headers.items()}
    # Build response
    response = {
        "status": 200,
        "headers": parsed,
        "body": {"message": "ok"}
    }
"#,
        )
    }

    // Startup and memory benchmarks

    /// Cold startup time benchmark
    pub fn bench_cold_startup(&self) -> BenchmarkSpec {
        BenchmarkSpec::new(
            "cold_startup",
            r#"
# Minimal startup - just import and exit
import sys
"#,
        )
    }

    /// Memory usage benchmark
    pub fn bench_memory_usage(&self) -> BenchmarkSpec {
        BenchmarkSpec::new(
            "memory_usage",
            r#"
# Allocate and use memory
data = []
for i in range(10000):
    data.append([j * i for j in range(100)])
# Force some computation
total = sum(sum(row) for row in data)
"#,
        )
    }
}
