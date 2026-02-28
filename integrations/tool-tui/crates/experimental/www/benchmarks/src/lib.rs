//! # dx-www Benchmarks
//!
//! Benchmark suite for measuring performance of dx-www components.
//!
//! ## Running Benchmarks
//!
//! Run all benchmarks:
//! ```bash
//! cargo bench -p dx-www-benchmarks
//! ```
//!
//! Run specific benchmark:
//! ```bash
//! cargo bench -p dx-www-benchmarks --bench htip_benchmarks
//! cargo bench -p dx-www-benchmarks --bench delta_benchmarks
//! cargo bench -p dx-www-benchmarks --bench ssr_benchmarks
//! cargo bench -p dx-www-benchmarks --bench parser_benchmarks
//! ```
//!
//! ## Benchmark Categories
//!
//! - **HTIP Benchmarks**: Serialization and deserialization throughput
//! - **Delta Benchmarks**: Patch generation and application performance
//! - **SSR Benchmarks**: Server-side rendering (template inflation) speed
//! - **Parser Benchmarks**: TSX/JSX parsing throughput
//!
//! ## Output Format
//!
//! Criterion generates HTML reports in `target/criterion/` with:
//! - Throughput measurements (bytes/second, elements/second)
//! - Statistical analysis with confidence intervals
//! - Comparison against previous runs (regression detection)

/// Re-export benchmark utilities
pub mod utils {
    /// Generate random bytes for testing
    pub fn random_bytes(size: usize, seed: u64) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(size);
        let mut state = seed;
        for _ in 0..size {
            // Simple LCG for reproducible "random" data
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            bytes.push((state >> 56) as u8);
        }
        bytes
    }

    /// Generate similar byte sequences with a given similarity ratio
    pub fn similar_bytes(size: usize, similarity: f64, seed: u64) -> (Vec<u8>, Vec<u8>) {
        let base = random_bytes(size, seed);
        let mut target = base.clone();

        let changes = ((1.0 - similarity) * size as f64) as usize;
        let mut state = seed.wrapping_add(12345);

        for _ in 0..changes {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let idx = (state as usize) % size;
            target[idx] = target[idx].wrapping_add(1);
        }

        (base, target)
    }
}
