//! Comprehensive benchmark suite for DCP performance testing.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::json_rpc::{compare_sizes_auto, SizeComparison};

/// Configuration for benchmark runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    /// Number of iterations for throughput tests
    pub iterations: usize,
    /// Number of warmup iterations
    pub warmup_iterations: usize,
    /// Message sizes to test (in bytes)
    pub message_sizes: Vec<usize>,
    /// Number of concurrent connections for throughput tests
    pub concurrency_levels: Vec<usize>,
    /// Whether to include DCP vs MCP comparison
    pub include_comparison: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            iterations: 10000,
            warmup_iterations: 1000,
            message_sizes: vec![64, 256, 1024, 4096, 16384],
            concurrency_levels: vec![1, 10, 50, 100],
            include_comparison: true,
        }
    }
}

/// Latency statistics with percentiles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyStats {
    /// Minimum latency
    pub min: Duration,
    /// Maximum latency
    pub max: Duration,
    /// Mean latency
    pub mean: Duration,
    /// Median (p50) latency
    pub p50: Duration,
    /// 95th percentile latency
    pub p95: Duration,
    /// 99th percentile latency
    pub p99: Duration,
    /// Standard deviation
    pub std_dev: Duration,
}

impl LatencyStats {
    /// Calculate latency statistics from a list of durations.
    pub fn from_durations(mut durations: Vec<Duration>) -> Self {
        if durations.is_empty() {
            return Self {
                min: Duration::ZERO,
                max: Duration::ZERO,
                mean: Duration::ZERO,
                p50: Duration::ZERO,
                p95: Duration::ZERO,
                p99: Duration::ZERO,
                std_dev: Duration::ZERO,
            };
        }

        durations.sort();
        let n = durations.len();

        let min = durations[0];
        let max = durations[n - 1];

        let total: Duration = durations.iter().sum();
        let mean = total / n as u32;

        let p50 = durations[n / 2];
        let p95 = durations[(n as f64 * 0.95) as usize];
        let p99 = durations[(n as f64 * 0.99).min((n - 1) as f64) as usize];

        // Calculate standard deviation
        let mean_nanos = mean.as_nanos() as f64;
        let variance: f64 = durations
            .iter()
            .map(|d| {
                let diff = d.as_nanos() as f64 - mean_nanos;
                diff * diff
            })
            .sum::<f64>()
            / n as f64;
        let std_dev = Duration::from_nanos(variance.sqrt() as u64);

        Self {
            min,
            max,
            mean,
            p50,
            p95,
            p99,
            std_dev,
        }
    }
}

/// Throughput measurement result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThroughputResult {
    /// Messages processed per second
    pub messages_per_second: f64,
    /// Bytes processed per second
    pub bytes_per_second: f64,
    /// Total messages processed
    pub total_messages: usize,
    /// Total bytes processed
    pub total_bytes: usize,
    /// Total duration
    pub duration: Duration,
}

/// Memory usage measurement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    /// Heap allocated bytes
    pub heap_bytes: usize,
    /// Peak memory usage
    pub peak_bytes: usize,
    /// Number of allocations
    pub allocation_count: usize,
}

impl Default for MemoryUsage {
    fn default() -> Self {
        Self {
            heap_bytes: 0,
            peak_bytes: 0,
            allocation_count: 0,
        }
    }
}

/// DCP vs MCP comparison result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolComparison {
    /// Payload description
    pub payload_name: String,
    /// JSON-RPC (MCP) message size
    pub mcp_size: usize,
    /// DCP binary message size
    pub dcp_size: usize,
    /// Size ratio (MCP / DCP)
    pub size_ratio: f64,
    /// MCP encoding latency
    pub mcp_encode_latency: Duration,
    /// DCP encoding latency
    pub dcp_encode_latency: Duration,
    /// Latency ratio (MCP / DCP)
    pub latency_ratio: f64,
}

/// Individual benchmark result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Benchmark name
    pub name: String,
    /// Latency statistics
    pub latency: LatencyStats,
    /// Throughput result
    pub throughput: ThroughputResult,
    /// Memory usage (if measured)
    pub memory: Option<MemoryUsage>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Complete benchmark suite results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResults {
    /// Configuration used
    pub config: BenchmarkConfig,
    /// Individual benchmark results
    pub results: Vec<BenchmarkResult>,
    /// Protocol comparison results
    pub comparisons: Vec<ProtocolComparison>,
    /// Timestamp when benchmarks were run
    pub timestamp: String,
    /// Total benchmark duration
    pub total_duration: Duration,
}

impl BenchmarkResults {
    /// Generate a JSON report.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Generate a Markdown report.
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str("# DCP Benchmark Results\n\n");
        md.push_str(&format!("Generated: {}\n\n", self.timestamp));
        md.push_str(&format!("Total Duration: {:.2}s\n\n", self.total_duration.as_secs_f64()));

        // Configuration
        md.push_str("## Configuration\n\n");
        md.push_str(&format!("- Iterations: {}\n", self.config.iterations));
        md.push_str(&format!("- Warmup Iterations: {}\n", self.config.warmup_iterations));
        md.push_str(&format!("- Message Sizes: {:?}\n", self.config.message_sizes));
        md.push_str(&format!("- Concurrency Levels: {:?}\n\n", self.config.concurrency_levels));

        // Latency Results
        md.push_str("## Latency Results\n\n");
        md.push_str("| Benchmark | Min | Mean | P50 | P95 | P99 | Max |\n");
        md.push_str("|-----------|-----|------|-----|-----|-----|-----|\n");

        for result in &self.results {
            md.push_str(&format!(
                "| {} | {:?} | {:?} | {:?} | {:?} | {:?} | {:?} |\n",
                result.name,
                result.latency.min,
                result.latency.mean,
                result.latency.p50,
                result.latency.p95,
                result.latency.p99,
                result.latency.max
            ));
        }
        md.push('\n');

        // Throughput Results
        md.push_str("## Throughput Results\n\n");
        md.push_str("| Benchmark | Messages/sec | MB/sec | Total Messages |\n");
        md.push_str("|-----------|--------------|--------|----------------|\n");

        for result in &self.results {
            md.push_str(&format!(
                "| {} | {:.0} | {:.2} | {} |\n",
                result.name,
                result.throughput.messages_per_second,
                result.throughput.bytes_per_second / 1_000_000.0,
                result.throughput.total_messages
            ));
        }
        md.push('\n');

        // Protocol Comparison
        if !self.comparisons.is_empty() {
            md.push_str("## DCP vs MCP Comparison\n\n");
            md.push_str("| Payload | MCP Size | DCP Size | Size Ratio | MCP Latency | DCP Latency | Latency Ratio |\n");
            md.push_str("|---------|----------|----------|------------|-------------|-------------|---------------|\n");

            for comp in &self.comparisons {
                md.push_str(&format!(
                    "| {} | {} B | {} B | {:.2}x | {:?} | {:?} | {:.2}x |\n",
                    comp.payload_name,
                    comp.mcp_size,
                    comp.dcp_size,
                    comp.size_ratio,
                    comp.mcp_encode_latency,
                    comp.dcp_encode_latency,
                    comp.latency_ratio
                ));
            }
            md.push('\n');
        }

        md
    }
}

/// Benchmark suite runner.
pub struct BenchmarkSuite {
    config: BenchmarkConfig,
    results: Vec<BenchmarkResult>,
    comparisons: Vec<ProtocolComparison>,
}

impl BenchmarkSuite {
    /// Create a new benchmark suite with the given configuration.
    pub fn new(config: BenchmarkConfig) -> Self {
        Self {
            config,
            results: Vec::new(),
            comparisons: Vec::new(),
        }
    }

    /// Create a benchmark suite with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(BenchmarkConfig::default())
    }

    /// Run all benchmarks and return results.
    pub fn run(&mut self) -> BenchmarkResults {
        let start = Instant::now();

        // Run throughput benchmarks
        self.run_throughput_benchmarks();

        // Run latency benchmarks
        self.run_latency_benchmarks();

        // Run protocol comparison
        if self.config.include_comparison {
            self.run_protocol_comparison();
        }

        let total_duration = start.elapsed();

        BenchmarkResults {
            config: self.config.clone(),
            results: self.results.clone(),
            comparisons: self.comparisons.clone(),
            timestamp: chrono_lite_timestamp(),
            total_duration,
        }
    }

    fn run_throughput_benchmarks(&mut self) {
        for &size in &self.config.message_sizes.clone() {
            let result = self.benchmark_throughput(size);
            self.results.push(result);
        }
    }

    fn run_latency_benchmarks(&mut self) {
        for &size in &self.config.message_sizes.clone() {
            let result = self.benchmark_latency(size);
            self.results.push(result);
        }
    }

    fn benchmark_throughput(&self, message_size: usize) -> BenchmarkResult {
        let payload = generate_payload(message_size);
        let iterations = self.config.iterations;

        // Warmup
        for _ in 0..self.config.warmup_iterations {
            let _ = serde_json::to_string(&payload);
        }

        let start = Instant::now();
        let mut total_bytes = 0;

        for _ in 0..iterations {
            let encoded = serde_json::to_string(&payload).unwrap_or_default();
            total_bytes += encoded.len();
        }

        let duration = start.elapsed();
        let messages_per_second = iterations as f64 / duration.as_secs_f64();
        let bytes_per_second = total_bytes as f64 / duration.as_secs_f64();

        BenchmarkResult {
            name: format!("throughput_{}b", message_size),
            latency: LatencyStats::from_durations(vec![duration / iterations as u32]),
            throughput: ThroughputResult {
                messages_per_second,
                bytes_per_second,
                total_messages: iterations,
                total_bytes,
                duration,
            },
            memory: None,
            metadata: HashMap::new(),
        }
    }

    fn benchmark_latency(&self, message_size: usize) -> BenchmarkResult {
        let payload = generate_payload(message_size);
        let iterations = self.config.iterations;

        // Warmup
        for _ in 0..self.config.warmup_iterations {
            let _ = serde_json::to_string(&payload);
        }

        let mut latencies = Vec::with_capacity(iterations);
        let mut total_bytes = 0;

        for _ in 0..iterations {
            let start = Instant::now();
            let encoded = serde_json::to_string(&payload).unwrap_or_default();
            latencies.push(start.elapsed());
            total_bytes += encoded.len();
        }

        let total_duration: Duration = latencies.iter().sum();
        let messages_per_second = iterations as f64 / total_duration.as_secs_f64();
        let bytes_per_second = total_bytes as f64 / total_duration.as_secs_f64();

        BenchmarkResult {
            name: format!("latency_{}b", message_size),
            latency: LatencyStats::from_durations(latencies),
            throughput: ThroughputResult {
                messages_per_second,
                bytes_per_second,
                total_messages: iterations,
                total_bytes,
                duration: total_duration,
            },
            memory: None,
            metadata: HashMap::new(),
        }
    }

    fn run_protocol_comparison(&mut self) {
        let payloads = get_realistic_payloads();

        for (name, params) in payloads {
            let comparison = self.compare_protocols(&name, &params);
            self.comparisons.push(comparison);
        }
    }

    fn compare_protocols(&self, name: &str, params: &Value) -> ProtocolComparison {
        let size_comparison = compare_sizes_auto("tools/call", params);

        // Measure MCP encoding latency
        let mcp_latency = {
            let mut total = Duration::ZERO;
            for _ in 0..1000 {
                let start = Instant::now();
                let _ = serde_json::to_string(params);
                total += start.elapsed();
            }
            total / 1000
        };

        // Measure DCP encoding latency (simulated binary encoding)
        let dcp_latency = {
            let mut total = Duration::ZERO;
            for _ in 0..1000 {
                let start = Instant::now();
                let _ = simulate_binary_encode(params);
                total += start.elapsed();
            }
            total / 1000
        };

        let latency_ratio = if dcp_latency.as_nanos() > 0 {
            mcp_latency.as_nanos() as f64 / dcp_latency.as_nanos() as f64
        } else {
            1.0
        };

        ProtocolComparison {
            payload_name: name.to_string(),
            mcp_size: size_comparison.json_rpc_size,
            dcp_size: size_comparison.dcp_size,
            size_ratio: size_comparison.ratio,
            mcp_encode_latency: mcp_latency,
            dcp_encode_latency: dcp_latency,
            latency_ratio,
        }
    }
}

/// Generate a payload of approximately the given size.
fn generate_payload(target_size: usize) -> Value {
    let base = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "id": 1,
        "params": {
            "name": "test_tool"
        }
    });

    let base_size = serde_json::to_string(&base).unwrap_or_default().len();

    if target_size <= base_size {
        return base;
    }

    let padding_size = target_size - base_size - 20; // Account for JSON overhead
    let padding = "x".repeat(padding_size.max(0));

    json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "id": 1,
        "params": {
            "name": "test_tool",
            "data": padding
        }
    })
}

/// Get realistic tool invocation payloads for comparison.
fn get_realistic_payloads() -> Vec<(String, Value)> {
    vec![
        (
            "simple_tool_call".to_string(),
            json!({
                "name": "read_file",
                "arguments": {
                    "path": "/home/user/project/src/main.rs"
                }
            }),
        ),
        (
            "tool_with_options".to_string(),
            json!({
                "name": "search",
                "arguments": {
                    "query": "function definition",
                    "path": "/home/user/project",
                    "include": ["*.rs", "*.ts"],
                    "exclude": ["target", "node_modules"],
                    "caseSensitive": false,
                    "maxResults": 100
                }
            }),
        ),
        (
            "code_execution".to_string(),
            json!({
                "name": "execute_code",
                "arguments": {
                    "language": "python",
                    "code": "def fibonacci(n):\n    if n <= 1:\n        return n\n    return fibonacci(n-1) + fibonacci(n-2)\n\nprint(fibonacci(10))",
                    "timeout": 30000
                }
            }),
        ),
        (
            "large_content".to_string(),
            json!({
                "name": "write_file",
                "arguments": {
                    "path": "/home/user/project/output.txt",
                    "content": "Lorem ipsum dolor sit amet, consectetur adipiscing elit. ".repeat(100)
                }
            }),
        ),
        (
            "structured_data".to_string(),
            json!({
                "name": "create_resource",
                "arguments": {
                    "type": "database",
                    "config": {
                        "host": "localhost",
                        "port": 5432,
                        "database": "myapp",
                        "user": "admin",
                        "ssl": true,
                        "poolSize": 10,
                        "timeout": 30000,
                        "retries": 3
                    }
                }
            }),
        ),
    ]
}

/// Simulate binary encoding (placeholder for actual DCP encoding).
fn simulate_binary_encode(value: &Value) -> Vec<u8> {
    // Simple simulation: convert to msgpack-like format
    let json_str = serde_json::to_string(value).unwrap_or_default();
    // Simulate compression ratio of ~0.6
    let compressed_size = (json_str.len() as f64 * 0.6) as usize;
    vec![0u8; compressed_size]
}

/// Simple timestamp without external dependency.
fn chrono_lite_timestamp() -> String {
    use std::time::SystemTime;
    let duration = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
    format!("{}s since epoch", duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_stats() {
        let durations: Vec<Duration> = (1..=100).map(|i| Duration::from_micros(i * 10)).collect();

        let stats = LatencyStats::from_durations(durations);

        assert_eq!(stats.min, Duration::from_micros(10));
        assert_eq!(stats.max, Duration::from_micros(1000));
        // p50 is the median - for 100 elements, index 50 gives us element 51 (510µs)
        assert_eq!(stats.p50, Duration::from_micros(510));
    }

    #[test]
    fn test_benchmark_config_default() {
        let config = BenchmarkConfig::default();
        assert_eq!(config.iterations, 10000);
        assert!(!config.message_sizes.is_empty());
    }

    #[test]
    fn test_generate_payload() {
        let payload = generate_payload(256);
        let size = serde_json::to_string(&payload).unwrap().len();
        // Should be approximately the target size
        assert!(size >= 200 && size <= 300);
    }

    #[test]
    fn test_benchmark_suite_runs() {
        let config = BenchmarkConfig {
            iterations: 100,
            warmup_iterations: 10,
            message_sizes: vec![64],
            concurrency_levels: vec![1],
            include_comparison: true,
        };

        let mut suite = BenchmarkSuite::new(config);
        let results = suite.run();

        assert!(!results.results.is_empty());
        assert!(!results.comparisons.is_empty());
    }

    #[test]
    fn test_results_to_json() {
        let results = BenchmarkResults {
            config: BenchmarkConfig::default(),
            results: vec![],
            comparisons: vec![],
            timestamp: "test".to_string(),
            total_duration: Duration::from_secs(1),
        };

        let json = results.to_json();
        assert!(json.contains("config"));
        assert!(json.contains("results"));
    }

    #[test]
    fn test_results_to_markdown() {
        let results = BenchmarkResults {
            config: BenchmarkConfig::default(),
            results: vec![],
            comparisons: vec![],
            timestamp: "test".to_string(),
            total_duration: Duration::from_secs(1),
        };

        let md = results.to_markdown();
        assert!(md.contains("# DCP Benchmark Results"));
        assert!(md.contains("## Configuration"));
    }
}
