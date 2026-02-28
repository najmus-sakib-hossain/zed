//! DX-Py vs UV Comparison Benchmarks
//!
//! Comprehensive benchmark suite comparing dx-py against uv across:
//! - Package resolution
//! - Package installation
//! - Virtual environment creation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::hash::Hash;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

// ============================================================================
// System Information
// ============================================================================

/// System specs for reproducibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub cpu: String,
    pub cpu_cores: usize,
    pub memory_gb: f64,
    pub dx_py_version: String,
    pub uv_version: Option<String>,
}

impl SystemInfo {
    /// Detect system information
    pub fn detect() -> Self {
        let os = Self::detect_os();
        let arch = env::consts::ARCH.to_string();
        let cpu = Self::detect_cpu();
        let cpu_cores = num_cpus();
        let memory_gb = Self::detect_memory_gb();
        let dx_py_version = env!("CARGO_PKG_VERSION").to_string();
        let uv_version = Self::detect_uv_version();

        SystemInfo {
            os,
            arch,
            cpu,
            cpu_cores,
            memory_gb,
            dx_py_version,
            uv_version,
        }
    }

    fn detect_os() -> String {
        let os = env::consts::OS;
        match os {
            "windows" => {
                // Try to get Windows version
                if let Ok(output) = Command::new("cmd").args(["/C", "ver"]).output() {
                    let version = String::from_utf8_lossy(&output.stdout);
                    if version.contains("10") || version.contains("11") {
                        return format!(
                            "Windows {}",
                            if version.contains("11") { "11" } else { "10" }
                        );
                    }
                }
                "Windows".to_string()
            }
            "macos" => {
                if let Ok(output) = Command::new("sw_vers").args(["-productVersion"]).output() {
                    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    return format!("macOS {}", version);
                }
                "macOS".to_string()
            }
            "linux" => {
                // Try to read /etc/os-release
                if let Ok(content) = fs::read_to_string("/etc/os-release") {
                    for line in content.lines() {
                        if line.starts_with("PRETTY_NAME=") {
                            let name = line.trim_start_matches("PRETTY_NAME=").trim_matches('"');
                            return name.to_string();
                        }
                    }
                }
                "Linux".to_string()
            }
            _ => os.to_string(),
        }
    }

    fn detect_cpu() -> String {
        #[cfg(target_os = "windows")]
        {
            if let Ok(output) = Command::new("wmic").args(["cpu", "get", "name"]).output() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines().skip(1) {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        return trimmed.to_string();
                    }
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            if let Ok(output) =
                Command::new("sysctl").args(["-n", "machdep.cpu.brand_string"]).output()
            {
                return String::from_utf8_lossy(&output.stdout).trim().to_string();
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
                for line in content.lines() {
                    if line.starts_with("model name") {
                        if let Some(name) = line.split(':').nth(1) {
                            return name.trim().to_string();
                        }
                    }
                }
            }
        }

        "Unknown CPU".to_string()
    }

    fn detect_memory_gb() -> f64 {
        #[cfg(target_os = "windows")]
        {
            if let Ok(output) = Command::new("wmic")
                .args(["computersystem", "get", "totalphysicalmemory"])
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                for line in output_str.lines().skip(1) {
                    if let Ok(bytes) = line.trim().parse::<u64>() {
                        return bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                    }
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = Command::new("sysctl").args(["-n", "hw.memsize"]).output() {
                if let Ok(bytes) = String::from_utf8_lossy(&output.stdout).trim().parse::<u64>() {
                    return bytes as f64 / (1024.0 * 1024.0 * 1024.0);
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = fs::read_to_string("/proc/meminfo") {
                for line in content.lines() {
                    if line.starts_with("MemTotal:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            if let Ok(kb) = parts[1].parse::<u64>() {
                                return kb as f64 / (1024.0 * 1024.0);
                            }
                        }
                    }
                }
            }
        }

        0.0
    }

    fn detect_uv_version() -> Option<String> {
        let uv_cmd = if cfg!(windows) { "uv.exe" } else { "uv" };

        if let Ok(output) = Command::new(uv_cmd).args(["--version"]).output() {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout);
                // Parse "uv 0.1.0" format
                return version.trim().strip_prefix("uv ").map(|s| s.to_string());
            }
        }
        None
    }
}

/// Get number of CPU cores
fn num_cpus() -> usize {
    std::thread::available_parallelism().map(|p| p.get()).unwrap_or(1)
}

// ============================================================================
// Cache Management
// ============================================================================

/// Cache clearing for cold start benchmarks
#[derive(Debug, Clone)]
pub struct CacheManager {
    pub dx_py_cache: PathBuf,
    pub uv_cache: PathBuf,
}

impl CacheManager {
    /// Create a new cache manager with default paths
    pub fn new() -> Self {
        let dx_py_cache = Self::get_dx_py_cache_path();
        let uv_cache = Self::get_uv_cache_path();

        CacheManager {
            dx_py_cache,
            uv_cache,
        }
    }

    fn get_dx_py_cache_path() -> PathBuf {
        if cfg!(windows) {
            dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".")).join("dx-py")
        } else {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".cache")
                .join("dx-py")
        }
    }

    fn get_uv_cache_path() -> PathBuf {
        if cfg!(windows) {
            dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".")).join("uv")
        } else {
            dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".cache").join("uv")
        }
    }

    /// Clear dx-py cache
    pub fn clear_dx_py_cache(&self) -> std::io::Result<()> {
        if self.dx_py_cache.exists() {
            fs::remove_dir_all(&self.dx_py_cache)?;
        }
        Ok(())
    }

    /// Clear uv cache
    pub fn clear_uv_cache(&self) -> std::io::Result<()> {
        if self.uv_cache.exists() {
            fs::remove_dir_all(&self.uv_cache)?;
        }
        Ok(())
    }

    /// Clear all caches
    pub fn clear_all(&self) -> std::io::Result<()> {
        let _ = self.clear_dx_py_cache();
        let _ = self.clear_uv_cache();
        Ok(())
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Test Project Definitions
// ============================================================================

/// Project complexity category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectCategory {
    Simple,  // 5-10 deps
    Medium,  // 20-50 deps
    Complex, // 100+ deps
}

impl std::fmt::Display for ProjectCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectCategory::Simple => write!(f, "simple"),
            ProjectCategory::Medium => write!(f, "medium"),
            ProjectCategory::Complex => write!(f, "complex"),
        }
    }
}

/// Test project configuration for benchmarks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestProject {
    pub name: String,
    pub dependencies: Vec<String>,
    pub category: ProjectCategory,
}

impl TestProject {
    /// Create a simple project (5-10 dependencies)
    pub fn simple() -> Self {
        TestProject {
            name: "simple-project".to_string(),
            dependencies: vec![
                "requests".to_string(),
                "click".to_string(),
                "rich".to_string(),
                "httpx".to_string(),
                "pydantic".to_string(),
            ],
            category: ProjectCategory::Simple,
        }
    }

    /// Create a medium project (20-50 dependencies)
    pub fn medium() -> Self {
        TestProject {
            name: "medium-project".to_string(),
            dependencies: vec![
                // Web framework
                "flask".to_string(),
                "flask-cors".to_string(),
                "flask-login".to_string(),
                // Database
                "sqlalchemy".to_string(),
                "alembic".to_string(),
                // Task queue
                "celery".to_string(),
                "redis".to_string(),
                // AWS
                "boto3".to_string(),
                "botocore".to_string(),
                // HTTP
                "requests".to_string(),
                "httpx".to_string(),
                "aiohttp".to_string(),
                // Data validation
                "pydantic".to_string(),
                "marshmallow".to_string(),
                // CLI
                "click".to_string(),
                "rich".to_string(),
                "typer".to_string(),
                // Testing
                "pytest".to_string(),
                "pytest-cov".to_string(),
                "pytest-asyncio".to_string(),
                // Utilities
                "python-dotenv".to_string(),
                "pyyaml".to_string(),
                "toml".to_string(),
                "jinja2".to_string(),
                "pillow".to_string(),
            ],
            category: ProjectCategory::Medium,
        }
    }

    /// Create a complex project (100+ dependencies)
    pub fn complex() -> Self {
        TestProject {
            name: "complex-project".to_string(),
            dependencies: vec![
                // Data science core
                "pandas".to_string(),
                "numpy".to_string(),
                "scipy".to_string(),
                "matplotlib".to_string(),
                "seaborn".to_string(),
                // Machine learning
                "scikit-learn".to_string(),
                "xgboost".to_string(),
                "lightgbm".to_string(),
                // Deep learning (lighter alternatives)
                "torch".to_string(),
                // Data processing
                "polars".to_string(),
                "pyarrow".to_string(),
                "fastparquet".to_string(),
                // Visualization
                "plotly".to_string(),
                "bokeh".to_string(),
                "altair".to_string(),
                // Web framework
                "fastapi".to_string(),
                "uvicorn".to_string(),
                "starlette".to_string(),
                "django".to_string(),
                "flask".to_string(),
                // Database
                "sqlalchemy".to_string(),
                "asyncpg".to_string(),
                "psycopg2-binary".to_string(),
                "pymongo".to_string(),
                "redis".to_string(),
                // HTTP clients
                "requests".to_string(),
                "httpx".to_string(),
                "aiohttp".to_string(),
                "urllib3".to_string(),
                // AWS
                "boto3".to_string(),
                "aiobotocore".to_string(),
                // Data validation
                "pydantic".to_string(),
                "marshmallow".to_string(),
                "attrs".to_string(),
                // CLI
                "click".to_string(),
                "rich".to_string(),
                "typer".to_string(),
                "tqdm".to_string(),
                // Testing
                "pytest".to_string(),
                "pytest-cov".to_string(),
                "pytest-asyncio".to_string(),
                "hypothesis".to_string(),
                "faker".to_string(),
                // Utilities
                "python-dotenv".to_string(),
                "pyyaml".to_string(),
                "toml".to_string(),
                "orjson".to_string(),
                "ujson".to_string(),
                "msgpack".to_string(),
                "cryptography".to_string(),
                "bcrypt".to_string(),
                "pyjwt".to_string(),
                "python-dateutil".to_string(),
                "pytz".to_string(),
                "pendulum".to_string(),
                "arrow".to_string(),
                // Logging
                "loguru".to_string(),
                "structlog".to_string(),
                // Async
                "anyio".to_string(),
                "trio".to_string(),
                // Image processing
                "pillow".to_string(),
                "opencv-python-headless".to_string(),
                // NLP
                "nltk".to_string(),
                "spacy".to_string(),
                // Jupyter
                "jupyter".to_string(),
                "ipython".to_string(),
                "notebook".to_string(),
            ],
            category: ProjectCategory::Complex,
        }
    }

    /// Generate pyproject.toml content for this project
    pub fn to_pyproject_toml(&self) -> String {
        let deps: Vec<String> =
            self.dependencies.iter().map(|d| format!("    \"{}\"", d)).collect();

        format!(
            r#"[project]
name = "{}"
version = "0.1.0"
requires-python = ">=3.10"
dependencies = [
{}
]
"#,
            self.name,
            deps.join(",\n")
        )
    }
}

// ============================================================================
// Benchmark Results
// ============================================================================

/// Tool being benchmarked
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tool {
    DxPy,
    Uv,
}

impl std::fmt::Display for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Tool::DxPy => write!(f, "dx-py"),
            Tool::Uv => write!(f, "uv"),
        }
    }
}

/// Operation being benchmarked
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Operation {
    Resolution,
    Installation,
    VenvCreation,
    Download,
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::Resolution => write!(f, "resolution"),
            Operation::Installation => write!(f, "installation"),
            Operation::VenvCreation => write!(f, "venv_creation"),
            Operation::Download => write!(f, "download"),
        }
    }
}

/// Single benchmark measurement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub tool: Tool,
    pub operation: Operation,
    pub scenario: String,
    pub cold_start_ms: Vec<f64>,
    pub warm_start_ms: Vec<f64>,
    pub mean_cold_ms: f64,
    pub mean_warm_ms: f64,
    pub std_dev_cold: f64,
    pub std_dev_warm: f64,
}

impl BenchmarkResult {
    /// Create a new benchmark result from measurements
    pub fn new(
        tool: Tool,
        operation: Operation,
        scenario: String,
        cold_start_ms: Vec<f64>,
        warm_start_ms: Vec<f64>,
    ) -> Self {
        let mean_cold_ms = mean(&cold_start_ms);
        let mean_warm_ms = mean(&warm_start_ms);
        let std_dev_cold = std_dev(&cold_start_ms);
        let std_dev_warm = std_dev(&warm_start_ms);

        BenchmarkResult {
            tool,
            operation,
            scenario,
            cold_start_ms,
            warm_start_ms,
            mean_cold_ms,
            mean_warm_ms,
            std_dev_cold,
            std_dev_warm,
        }
    }
}

/// Comparison summary between dx-py and uv
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonSummary {
    pub resolution_speedup: f64,
    pub installation_speedup: f64,
    pub venv_speedup: f64,
    pub overall_speedup: f64,
}

/// Aggregated results for all benchmarks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResults {
    pub results: Vec<BenchmarkResult>,
    pub system_info: SystemInfo,
    pub timestamp: String,
}

impl BenchmarkResults {
    /// Create new benchmark results
    pub fn new(results: Vec<BenchmarkResult>, system_info: SystemInfo) -> Self {
        let timestamp = chrono_now();
        BenchmarkResults {
            results,
            system_info,
            timestamp,
        }
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Generate markdown comparison table
    pub fn to_markdown_table(&self) -> String {
        let mut output = String::new();
        output.push_str("## Performance Comparison: dx-py vs uv\n\n");
        output.push_str(&format!("*Benchmarked on: {}*\n\n", self.timestamp));

        // System info
        output.push_str("### System Information\n\n");
        output.push_str(&format!("- **OS**: {}\n", self.system_info.os));
        output.push_str(&format!("- **Architecture**: {}\n", self.system_info.arch));
        output.push_str(&format!("- **CPU**: {}\n", self.system_info.cpu));
        output.push_str(&format!("- **CPU Cores**: {}\n", self.system_info.cpu_cores));
        output.push_str(&format!("- **Memory**: {:.1} GB\n", self.system_info.memory_gb));
        output.push_str(&format!("- **dx-py version**: {}\n", self.system_info.dx_py_version));
        if let Some(ref uv_ver) = self.system_info.uv_version {
            output.push_str(&format!("- **uv version**: {}\n", uv_ver));
        }
        output.push('\n');

        // Results table
        output.push_str("### Benchmark Results\n\n");
        output.push_str("| Operation | Scenario | dx-py (cold) | uv (cold) | Speedup | dx-py (warm) | uv (warm) | Speedup |\n");
        output.push_str("|-----------|----------|--------------|-----------|---------|--------------|-----------|--------|\n");

        // Group results by operation and scenario
        type ResultPair<'a> = (Option<&'a BenchmarkResult>, Option<&'a BenchmarkResult>);
        let mut grouped: HashMap<(Operation, String), ResultPair> = HashMap::new();

        for result in &self.results {
            let key = (result.operation, result.scenario.clone());
            let entry = grouped.entry(key).or_insert((None, None));
            match result.tool {
                Tool::DxPy => entry.0 = Some(result),
                Tool::Uv => entry.1 = Some(result),
            }
        }

        for ((operation, scenario), (dx_py, uv)) in grouped {
            let dx_cold = dx_py.map(|r| r.mean_cold_ms).unwrap_or(0.0);
            let uv_cold = uv.map(|r| r.mean_cold_ms).unwrap_or(0.0);
            let dx_warm = dx_py.map(|r| r.mean_warm_ms).unwrap_or(0.0);
            let uv_warm = uv.map(|r| r.mean_warm_ms).unwrap_or(0.0);

            let cold_speedup = if dx_cold > 0.0 {
                uv_cold / dx_cold
            } else {
                0.0
            };
            let warm_speedup = if dx_warm > 0.0 {
                uv_warm / dx_warm
            } else {
                0.0
            };

            output.push_str(&format!(
                "| {} | {} | {:.0}ms | {:.0}ms | {:.1}x | {:.0}ms | {:.0}ms | {:.1}x |\n",
                operation, scenario, dx_cold, uv_cold, cold_speedup, dx_warm, uv_warm, warm_speedup
            ));
        }

        output
    }

    /// Calculate comparison summary
    pub fn comparison_summary(&self) -> ComparisonSummary {
        let mut resolution_times = (Vec::new(), Vec::new());
        let mut installation_times = (Vec::new(), Vec::new());
        let mut venv_times = (Vec::new(), Vec::new());

        for result in &self.results {
            let times = match result.operation {
                Operation::Resolution => &mut resolution_times,
                Operation::Installation => &mut installation_times,
                Operation::VenvCreation => &mut venv_times,
                Operation::Download => continue,
            };

            match result.tool {
                Tool::DxPy => times.0.push(result.mean_cold_ms),
                Tool::Uv => times.1.push(result.mean_cold_ms),
            }
        }

        let resolution_speedup = calc_speedup(&resolution_times.0, &resolution_times.1);
        let installation_speedup = calc_speedup(&installation_times.0, &installation_times.1);
        let venv_speedup = calc_speedup(&venv_times.0, &venv_times.1);

        let overall_speedup = (resolution_speedup + installation_speedup + venv_speedup) / 3.0;

        ComparisonSummary {
            resolution_speedup,
            installation_speedup,
            venv_speedup,
            overall_speedup,
        }
    }
}

// ============================================================================
// Statistics Helpers
// ============================================================================

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

fn std_dev(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let m = mean(values);
    let variance = values.iter().map(|v| (v - m).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
    variance.sqrt()
}

fn calc_speedup(dx_py_times: &[f64], uv_times: &[f64]) -> f64 {
    let dx_mean = mean(dx_py_times);
    let uv_mean = mean(uv_times);
    if dx_mean > 0.0 {
        uv_mean / dx_mean
    } else {
        1.0
    }
}

fn chrono_now() -> String {
    // Simple timestamp without chrono dependency
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();
    // Convert to ISO-like format (approximate)
    format!("{}", secs)
}

// ============================================================================
// Benchmark Runner
// ============================================================================

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub iterations: usize,
    pub warmup_iterations: usize,
    pub timeout_seconds: u64,
    pub include_cold_start: bool,
    pub include_warm_start: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        BenchmarkConfig {
            iterations: 5,
            warmup_iterations: 1,
            timeout_seconds: 300,
            include_cold_start: true,
            include_warm_start: true,
        }
    }
}

/// Main benchmark orchestrator
pub struct BenchmarkRunner {
    pub dx_py_path: PathBuf,
    pub uv_path: Option<PathBuf>,
    pub output_dir: PathBuf,
    pub config: BenchmarkConfig,
    pub cache_manager: CacheManager,
}

impl BenchmarkRunner {
    /// Create a new benchmark runner
    pub fn new() -> Result<Self, String> {
        let dx_py_path = Self::find_dx_py()?;
        let uv_path = Self::detect_uv();
        let output_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let config = BenchmarkConfig::default();
        let cache_manager = CacheManager::new();

        Ok(BenchmarkRunner {
            dx_py_path,
            uv_path,
            output_dir,
            config,
            cache_manager,
        })
    }

    /// Find dx-py executable
    fn find_dx_py() -> Result<PathBuf, String> {
        let exe_name = if cfg!(windows) { "dx-py.exe" } else { "dx-py" };

        // Check current directory
        let local_path = PathBuf::from(exe_name);
        if local_path.exists() {
            return Ok(local_path);
        }

        // Check playground directory
        let playground_path = PathBuf::from("playground").join(exe_name);
        if playground_path.exists() {
            return Ok(playground_path);
        }

        // Check target/release
        let release_path = PathBuf::from("target/release").join(exe_name);
        if release_path.exists() {
            return Ok(release_path);
        }

        // Check PATH
        if let Ok(output) = Command::new("where").arg(exe_name).output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout);
                if let Some(first_line) = path.lines().next() {
                    return Ok(PathBuf::from(first_line.trim()));
                }
            }
        }

        Err("dx-py executable not found. Build with: cargo build --release".to_string())
    }

    /// Detect uv installation in PATH
    pub fn detect_uv() -> Option<PathBuf> {
        let exe_name = if cfg!(windows) { "uv.exe" } else { "uv" };

        // Check playground directory first
        let playground_path = PathBuf::from("playground").join(exe_name);
        if playground_path.exists() {
            return Some(playground_path);
        }

        // Check PATH using 'where' on Windows or 'which' on Unix
        #[cfg(windows)]
        {
            if let Ok(output) = Command::new("where").arg(exe_name).output() {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout);
                    if let Some(first_line) = path.lines().next() {
                        return Some(PathBuf::from(first_line.trim()));
                    }
                }
            }
        }

        #[cfg(not(windows))]
        {
            if let Ok(output) = Command::new("which").arg(exe_name).output() {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout);
                    return Some(PathBuf::from(path.trim()));
                }
            }
        }

        None
    }

    /// Check if uv is available
    pub fn has_uv(&self) -> bool {
        self.uv_path.is_some()
    }

    /// Run all benchmarks
    pub fn run_all(&self) -> BenchmarkResults {
        let mut results = Vec::new();

        println!("Running benchmark suite...");
        println!("dx-py: {:?}", self.dx_py_path);
        println!("uv: {:?}", self.uv_path);
        println!();

        // Resolution benchmarks
        println!("=== Resolution Benchmarks ===");
        results.extend(self.run_resolution_benchmarks());

        // Installation benchmarks
        println!("\n=== Installation Benchmarks ===");
        results.extend(self.run_installation_benchmarks());

        // Venv benchmarks
        println!("\n=== Venv Benchmarks ===");
        results.extend(self.run_venv_benchmarks());

        let system_info = SystemInfo::detect();
        BenchmarkResults::new(results, system_info)
    }

    /// Run resolution benchmarks
    pub fn run_resolution_benchmarks(&self) -> Vec<BenchmarkResult> {
        let mut results = Vec::new();
        let projects = vec![
            TestProject::simple(),
            TestProject::medium(),
            // Skip complex for now - too slow
        ];

        for project in projects {
            println!("  Benchmarking resolution: {}", project.category);

            // dx-py resolution
            if let Some(result) = self.benchmark_resolution(Tool::DxPy, &project) {
                results.push(result);
            }

            // uv resolution
            if self.has_uv() {
                if let Some(result) = self.benchmark_resolution(Tool::Uv, &project) {
                    results.push(result);
                }
            }
        }

        results
    }

    /// Run installation benchmarks
    pub fn run_installation_benchmarks(&self) -> Vec<BenchmarkResult> {
        let mut results = Vec::new();
        let projects = vec![TestProject::simple()];

        for project in projects {
            println!("  Benchmarking installation: {}", project.category);

            // dx-py installation
            if let Some(result) = self.benchmark_installation(Tool::DxPy, &project) {
                results.push(result);
            }

            // uv installation
            if self.has_uv() {
                if let Some(result) = self.benchmark_installation(Tool::Uv, &project) {
                    results.push(result);
                }
            }
        }

        results
    }

    /// Run venv benchmarks
    pub fn run_venv_benchmarks(&self) -> Vec<BenchmarkResult> {
        let mut results = Vec::new();

        println!("  Benchmarking venv creation");

        // dx-py venv
        if let Some(result) = self.benchmark_venv(Tool::DxPy) {
            results.push(result);
        }

        // uv venv
        if self.has_uv() {
            if let Some(result) = self.benchmark_venv(Tool::Uv) {
                results.push(result);
            }
        }

        results
    }

    fn benchmark_resolution(&self, tool: Tool, project: &TestProject) -> Option<BenchmarkResult> {
        let temp_dir = tempfile::tempdir().ok()?;
        let project_dir = temp_dir.path();

        // Write pyproject.toml
        let pyproject_path = project_dir.join("pyproject.toml");
        fs::write(&pyproject_path, project.to_pyproject_toml()).ok()?;

        let mut cold_times = Vec::new();
        let mut warm_times = Vec::new();

        // Cold start benchmarks
        if self.config.include_cold_start {
            for _ in 0..self.config.iterations {
                let _ = self.cache_manager.clear_all();
                // Remove lock file
                let _ = fs::remove_file(project_dir.join("uv.lock"));
                let _ = fs::remove_file(project_dir.join("dx-py.lock"));

                if let Some(duration) = self.run_lock_command(tool, project_dir) {
                    cold_times.push(duration.as_millis() as f64);
                }
            }
        }

        // Warm start benchmarks
        if self.config.include_warm_start {
            for _ in 0..self.config.iterations {
                if let Some(duration) = self.run_lock_command(tool, project_dir) {
                    warm_times.push(duration.as_millis() as f64);
                }
            }
        }

        if cold_times.is_empty() && warm_times.is_empty() {
            return None;
        }

        Some(BenchmarkResult::new(
            tool,
            Operation::Resolution,
            project.category.to_string(),
            cold_times,
            warm_times,
        ))
    }

    fn benchmark_installation(&self, tool: Tool, project: &TestProject) -> Option<BenchmarkResult> {
        let temp_dir = tempfile::tempdir().ok()?;
        let project_dir = temp_dir.path();

        // Write pyproject.toml
        let pyproject_path = project_dir.join("pyproject.toml");
        fs::write(&pyproject_path, project.to_pyproject_toml()).ok()?;

        // First create lock file
        let _ = self.run_lock_command(tool, project_dir);

        let mut cold_times = Vec::new();
        let mut warm_times = Vec::new();

        // Cold start benchmarks
        if self.config.include_cold_start {
            for _ in 0..self.config.iterations {
                let _ = self.cache_manager.clear_all();
                // Remove venv
                let _ = fs::remove_dir_all(project_dir.join(".venv"));

                if let Some(duration) = self.run_sync_command(tool, project_dir) {
                    cold_times.push(duration.as_millis() as f64);
                }
            }
        }

        // Warm start benchmarks
        if self.config.include_warm_start {
            for _ in 0..self.config.iterations {
                // Remove venv but keep cache
                let _ = fs::remove_dir_all(project_dir.join(".venv"));

                if let Some(duration) = self.run_sync_command(tool, project_dir) {
                    warm_times.push(duration.as_millis() as f64);
                }
            }
        }

        if cold_times.is_empty() && warm_times.is_empty() {
            return None;
        }

        Some(BenchmarkResult::new(
            tool,
            Operation::Installation,
            project.category.to_string(),
            cold_times,
            warm_times,
        ))
    }

    fn benchmark_venv(&self, tool: Tool) -> Option<BenchmarkResult> {
        let mut cold_times = Vec::new();

        // Venv creation benchmarks
        for _ in 0..self.config.iterations {
            let temp_dir = tempfile::tempdir().ok()?;
            let venv_path = temp_dir.path().join(".venv");

            if let Some(duration) = self.run_venv_command(tool, &venv_path) {
                cold_times.push(duration.as_millis() as f64);
            }
        }

        if cold_times.is_empty() {
            return None;
        }

        // Warm times are same as cold for venv (no caching)
        let warm_times = cold_times.clone();

        Some(BenchmarkResult::new(
            tool,
            Operation::VenvCreation,
            "empty".to_string(),
            cold_times,
            warm_times,
        ))
    }

    fn run_lock_command(&self, tool: Tool, project_dir: &std::path::Path) -> Option<Duration> {
        let start = Instant::now();

        let status = match tool {
            Tool::DxPy => Command::new(&self.dx_py_path)
                .args(["lock"])
                .current_dir(project_dir)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status(),
            Tool::Uv => {
                let uv_path = self.uv_path.as_ref()?;
                Command::new(uv_path)
                    .args(["lock"])
                    .current_dir(project_dir)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
            }
        };

        match status {
            Ok(s) if s.success() => Some(start.elapsed()),
            _ => None,
        }
    }

    fn run_sync_command(&self, tool: Tool, project_dir: &std::path::Path) -> Option<Duration> {
        let start = Instant::now();

        let status = match tool {
            Tool::DxPy => Command::new(&self.dx_py_path)
                .args(["sync"])
                .current_dir(project_dir)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status(),
            Tool::Uv => {
                let uv_path = self.uv_path.as_ref()?;
                Command::new(uv_path)
                    .args(["sync"])
                    .current_dir(project_dir)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
            }
        };

        match status {
            Ok(s) if s.success() => Some(start.elapsed()),
            _ => None,
        }
    }

    fn run_venv_command(&self, tool: Tool, venv_path: &std::path::Path) -> Option<Duration> {
        let start = Instant::now();

        let status = match tool {
            Tool::DxPy => Command::new(&self.dx_py_path)
                .args(["venv", venv_path.to_str()?])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status(),
            Tool::Uv => {
                let uv_path = self.uv_path.as_ref()?;
                Command::new(uv_path)
                    .args(["venv", venv_path.to_str()?])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
            }
        };

        match status {
            Ok(s) if s.success() => Some(start.elapsed()),
            _ => None,
        }
    }
}

impl Default for BenchmarkRunner {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| BenchmarkRunner {
            dx_py_path: PathBuf::from("dx-py"),
            uv_path: None,
            output_dir: PathBuf::from("."),
            config: BenchmarkConfig::default(),
            cache_manager: CacheManager::new(),
        })
    }
}

// ============================================================================
// Main Entry Point for Comparison Benchmarks
// ============================================================================

/// Run the full comparison benchmark suite
pub fn run_comparison_benchmarks() -> BenchmarkResults {
    match BenchmarkRunner::new() {
        Ok(runner) => runner.run_all(),
        Err(e) => {
            eprintln!("Failed to initialize benchmark runner: {}", e);
            BenchmarkResults::new(Vec::new(), SystemInfo::detect())
        }
    }
}

// ============================================================================
// Benchmark Main
// ============================================================================

fn main() {
    println!("DX-Py vs UV Comparison Benchmarks");
    println!("==================================\n");

    let results = run_comparison_benchmarks();

    // Output JSON
    let json_output = results.to_json();
    let json_path = "benchmark_results.json";
    if let Err(e) = fs::write(json_path, &json_output) {
        eprintln!("Failed to write JSON results: {}", e);
    } else {
        println!("\nJSON results written to: {}", json_path);
    }

    // Output Markdown
    let markdown_output = results.to_markdown_table();
    println!("\n{}", markdown_output);

    // Summary
    let summary = results.comparison_summary();
    println!("\n### Summary");
    println!("- Resolution speedup: {:.1}x", summary.resolution_speedup);
    println!("- Installation speedup: {:.1}x", summary.installation_speedup);
    println!("- Venv speedup: {:.1}x", summary.venv_speedup);
    println!("- Overall speedup: {:.1}x", summary.overall_speedup);
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_system_info_detect() {
        let info = SystemInfo::detect();
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
        assert!(info.cpu_cores > 0);
    }

    #[test]
    fn test_cache_manager_paths() {
        let manager = CacheManager::new();
        assert!(manager.dx_py_cache.to_string_lossy().contains("dx-py"));
        assert!(manager.uv_cache.to_string_lossy().contains("uv"));
    }

    #[test]
    fn test_test_project_simple() {
        let project = TestProject::simple();
        assert_eq!(project.category, ProjectCategory::Simple);
        assert!(project.dependencies.len() >= 5);
        assert!(project.dependencies.len() <= 10);
    }

    #[test]
    fn test_test_project_medium() {
        let project = TestProject::medium();
        assert_eq!(project.category, ProjectCategory::Medium);
        assert!(project.dependencies.len() >= 20);
    }

    #[test]
    fn test_test_project_pyproject_toml() {
        let project = TestProject::simple();
        let toml = project.to_pyproject_toml();
        assert!(toml.contains("[project]"));
        assert!(toml.contains("requests"));
    }

    #[test]
    fn test_statistics_mean() {
        assert_eq!(mean(&[1.0, 2.0, 3.0]), 2.0);
        assert_eq!(mean(&[]), 0.0);
    }

    #[test]
    fn test_statistics_std_dev() {
        let values = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let sd = std_dev(&values);
        assert!((sd - 2.0).abs() < 0.1);
    }

    #[test]
    fn test_benchmark_result_new() {
        let result = BenchmarkResult::new(
            Tool::DxPy,
            Operation::Resolution,
            "simple".to_string(),
            vec![100.0, 110.0, 90.0],
            vec![50.0, 55.0, 45.0],
        );
        assert_eq!(result.tool, Tool::DxPy);
        assert_eq!(result.mean_cold_ms, 100.0);
        assert_eq!(result.mean_warm_ms, 50.0);
    }

    #[test]
    fn test_uv_detection() {
        // This test just verifies the detection doesn't panic
        let _uv_path = BenchmarkRunner::detect_uv();
    }
}
