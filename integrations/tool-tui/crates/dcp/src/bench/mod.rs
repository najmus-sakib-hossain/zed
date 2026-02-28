//! Benchmarking utilities for DCP vs MCP comparison.

pub mod json_rpc;
pub mod suite;

pub use json_rpc::{
    compare_sizes_auto, encode_json_rpc, estimate_binary_size, measure_dcp_size,
    measure_json_rpc_size, SizeComparison,
};

pub use suite::{
    BenchmarkConfig, BenchmarkResult, BenchmarkResults, BenchmarkSuite, LatencyStats, MemoryUsage,
    ProtocolComparison, ThroughputResult,
};
