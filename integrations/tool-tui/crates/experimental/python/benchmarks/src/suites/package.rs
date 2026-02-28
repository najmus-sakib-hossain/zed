//! Package manager benchmark suite (DX-Py vs UV)

use crate::data::{TestDataGenerator, TestProject};
use crate::suites::BenchmarkSpec;

/// Package manager benchmark suite comparing DX-Py against UV
pub struct PackageSuite {
    pub test_projects: Vec<TestProject>,
    #[allow(dead_code)]
    data_generator: TestDataGenerator,
}

impl PackageSuite {
    pub fn new(seed: u64) -> Self {
        let mut generator = TestDataGenerator::new(seed);

        // Generate test projects of various sizes
        let test_projects = vec![
            generator.generate_project(5),   // Small
            generator.generate_project(20),  // Medium
            generator.generate_project(100), // Large
        ];

        Self {
            test_projects,
            data_generator: generator,
        }
    }

    /// Get all package manager benchmarks
    pub fn all_benchmarks(&self) -> Vec<BenchmarkSpec> {
        vec![
            self.bench_resolution_small(),
            self.bench_resolution_medium(),
            self.bench_resolution_large(),
            self.bench_install_cold_cache(),
            self.bench_install_warm_cache(),
            self.bench_lock_generation(),
            self.bench_venv_creation(),
        ]
    }

    /// Get real-world project benchmarks
    pub fn real_world_benchmarks(&self) -> Vec<BenchmarkSpec> {
        vec![
            self.bench_real_world_flask(),
            self.bench_real_world_django(),
            self.bench_real_world_requests(),
            self.bench_real_world_numpy(),
        ]
    }

    // Resolution benchmarks

    /// Small project dependency resolution (5 deps)
    pub fn bench_resolution_small(&self) -> BenchmarkSpec {
        self.create_resolution_benchmark("resolution_small", 5)
    }

    /// Medium project dependency resolution (20 deps)
    pub fn bench_resolution_medium(&self) -> BenchmarkSpec {
        self.create_resolution_benchmark("resolution_medium", 20)
    }

    /// Large project dependency resolution (100+ deps)
    pub fn bench_resolution_large(&self) -> BenchmarkSpec {
        self.create_resolution_benchmark("resolution_large", 100)
    }

    fn create_resolution_benchmark(&self, name: &str, dep_count: usize) -> BenchmarkSpec {
        let _project = self
            .test_projects
            .iter()
            .find(|p| p.dependency_count >= dep_count)
            .unwrap_or(&self.test_projects[0]);

        BenchmarkSpec::new(
            name,
            format!(
                r#"
# Simulate dependency resolution
# In real benchmark, this would call the package manager
deps = {}
resolved = {{}}
for dep in deps:
    resolved[dep] = "1.0.0"  # Simulated resolution
"#,
                dep_count
            ),
        )
    }

    // Installation benchmarks

    /// Cold cache installation benchmark
    pub fn bench_install_cold_cache(&self) -> BenchmarkSpec {
        BenchmarkSpec::new(
            "install_cold_cache",
            r#"
# Simulate cold cache installation
# In real benchmark, this would clear cache and install
import time
packages = ["requests", "flask", "click"]
for pkg in packages:
    # Simulated download and install
    pass
"#,
        )
    }

    /// Warm cache installation benchmark
    pub fn bench_install_warm_cache(&self) -> BenchmarkSpec {
        BenchmarkSpec::new(
            "install_warm_cache",
            r#"
# Simulate warm cache installation
# In real benchmark, packages would be cached
import time
packages = ["requests", "flask", "click"]
for pkg in packages:
    # Simulated cached install
    pass
"#,
        )
    }

    /// Lock file generation benchmark
    pub fn bench_lock_generation(&self) -> BenchmarkSpec {
        BenchmarkSpec::new(
            "lock_generation",
            r#"
# Simulate lock file generation
import json
dependencies = {
    "requests": "2.31.0",
    "flask": "3.0.0",
    "click": "8.1.7",
}
lock_content = {
    "version": 1,
    "packages": dependencies,
    "hashes": {k: f"sha256:{hash(v)}" for k, v in dependencies.items()}
}
lock_json = json.dumps(lock_content, indent=2)
"#,
        )
    }

    /// Virtual environment creation benchmark
    pub fn bench_venv_creation(&self) -> BenchmarkSpec {
        BenchmarkSpec::new(
            "venv_creation",
            r#"
# Simulate venv creation
# In real benchmark, this would create actual venv
import os
import tempfile
venv_path = tempfile.mkdtemp(prefix="bench_venv_")
# Simulated venv structure
dirs = ["bin", "lib", "include"]
for d in dirs:
    os.makedirs(os.path.join(venv_path, d), exist_ok=True)
# Cleanup
import shutil
shutil.rmtree(venv_path)
"#,
        )
    }

    // Real-world project benchmarks

    /// Flask project benchmark
    pub fn bench_real_world_flask(&self) -> BenchmarkSpec {
        BenchmarkSpec::new(
            "real_world_flask",
            r#"
# Simulate Flask project setup
flask_deps = [
    "flask>=3.0.0",
    "werkzeug>=3.0.0",
    "jinja2>=3.1.0",
    "itsdangerous>=2.1.0",
    "click>=8.1.0",
    "blinker>=1.7.0",
]
resolved = {dep.split(">=")[0]: dep.split(">=")[1] for dep in flask_deps}
"#,
        )
    }

    /// Django project benchmark
    pub fn bench_real_world_django(&self) -> BenchmarkSpec {
        BenchmarkSpec::new(
            "real_world_django",
            r#"
# Simulate Django project setup
django_deps = [
    "django>=5.0",
    "asgiref>=3.7.0",
    "sqlparse>=0.4.4",
    "tzdata",
]
resolved = {}
for dep in django_deps:
    if ">=" in dep:
        name, version = dep.split(">=")
        resolved[name] = version
    else:
        resolved[dep] = "latest"
"#,
        )
    }

    /// Requests project benchmark
    pub fn bench_real_world_requests(&self) -> BenchmarkSpec {
        BenchmarkSpec::new(
            "real_world_requests",
            r#"
# Simulate requests project setup
requests_deps = [
    "requests>=2.31.0",
    "charset-normalizer>=2.0.0",
    "idna>=2.5",
    "urllib3>=1.21.1",
    "certifi>=2017.4.17",
]
resolved = {dep.split(">=")[0]: dep.split(">=")[1] for dep in requests_deps}
"#,
        )
    }

    /// NumPy project benchmark
    pub fn bench_real_world_numpy(&self) -> BenchmarkSpec {
        BenchmarkSpec::new(
            "real_world_numpy",
            r#"
# Simulate numpy project setup (heavy computation)
# NumPy has complex build requirements
numpy_deps = ["numpy>=1.26.0"]
# Simulate wheel selection
platforms = ["manylinux_2_17_x86_64", "win_amd64", "macosx_10_9_x86_64"]
selected_wheel = platforms[0]  # Simulated platform detection
"#,
        )
    }
}

impl Default for PackageSuite {
    fn default() -> Self {
        Self::new(42)
    }
}
