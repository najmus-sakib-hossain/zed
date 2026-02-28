//! Benchmark configuration types

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Output format for benchmark results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OutputFormat {
    Markdown,
    Json,
    #[default]
    Both,
}

/// Configuration for benchmark execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    pub warmup_iterations: u32,
    pub measurement_iterations: u32,
    pub timeout_seconds: u64,
    pub output_format: OutputFormat,
    pub output_dir: PathBuf,
    pub seed: Option<u64>,
    pub suites: Vec<String>,
    pub filter: Option<String>,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            warmup_iterations: 5,
            measurement_iterations: 30,
            timeout_seconds: 300,
            output_format: OutputFormat::Both,
            output_dir: PathBuf::from("benchmark_results"),
            seed: None,
            suites: vec![],
            filter: None,
        }
    }
}
