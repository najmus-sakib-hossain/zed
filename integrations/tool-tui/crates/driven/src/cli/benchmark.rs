//! Benchmark Command
//!
//! Performance benchmarking for DX Binary Dawn features.

use crate::Result;
use crate::binary::{StringTable, StringTableBuilder, compute_blake3};
use crate::streaming::XorPatcher;
use console::style;
use std::path::Path;
use std::time::Instant;

/// Benchmark command for performance testing
#[derive(Debug)]
pub struct BenchmarkCommand;

/// Benchmark results
#[derive(Debug, Default)]
pub struct BenchmarkResults {
    /// String table lookup time (ns)
    pub string_lookup_ns: f64,
    /// XOR patch compute time (µs)
    pub xor_compute_us: f64,
    /// Blake3 hash throughput (bytes/sec)
    pub hash_throughput: f64,
}

impl BenchmarkCommand {
    /// Run all benchmarks
    pub fn execute(iterations: usize) -> Result<BenchmarkResults> {
        println!(
            "{} Running DX Binary Dawn Benchmarks ({} iterations)",
            style("⚡").bold(),
            iterations
        );
        println!();

        let mut results = BenchmarkResults::default();

        // String table benchmark
        println!("  {} String Table Lookup...", style("▸").cyan());
        results.string_lookup_ns = Self::bench_string_table(iterations);
        println!("    Lookup: {:.2} ns/op", results.string_lookup_ns);

        // XOR patcher benchmark
        println!("  {} XOR Patcher...", style("▸").cyan());
        results.xor_compute_us = Self::bench_xor_patcher(iterations);
        println!("    Compute: {:.2} µs/op", results.xor_compute_us);

        // Blake3 benchmark
        println!("  {} Blake3 Checksum...", style("▸").cyan());
        results.hash_throughput = Self::bench_blake3(iterations);
        println!("    Throughput: {:.2} MB/s", results.hash_throughput / 1_000_000.0);

        println!();
        println!("{} Benchmarks complete!", style("✓").green().bold());

        Ok(results)
    }

    /// Benchmark string table lookup
    fn bench_string_table(iterations: usize) -> f64 {
        let mut builder = StringTableBuilder::new();
        let strings: Vec<_> = (0..1000).map(|i| format!("string_{}", i)).collect();
        let ids: Vec<_> = strings.iter().map(|s| builder.intern(s)).collect();
        let table_bytes = builder.build();
        let table = StringTable::from_bytes(&table_bytes).unwrap();

        let start = Instant::now();
        for _ in 0..iterations {
            for &id in &ids {
                let _ = table.get(id);
            }
        }
        let elapsed = start.elapsed();

        let total_ops = ids.len() * iterations;
        elapsed.as_nanos() as f64 / total_ops as f64
    }

    /// Benchmark XOR patcher
    fn bench_xor_patcher(iterations: usize) -> f64 {
        let old = vec![0u8; 4096];
        let mut new = old.clone();
        // Simulate small changes
        for i in (0..new.len()).step_by(100) {
            new[i] = 1;
        }

        let patcher = XorPatcher::new(64);

        let start = Instant::now();
        for _ in 0..iterations {
            let _ = patcher.compute(&old, &new);
        }
        let elapsed = start.elapsed();

        elapsed.as_micros() as f64 / iterations as f64
    }

    /// Benchmark Blake3 hashing
    fn bench_blake3(iterations: usize) -> f64 {
        let data = vec![0u8; 65536]; // 64KB

        let start = Instant::now();
        for _ in 0..iterations {
            let _ = compute_blake3(&data);
        }
        let elapsed = start.elapsed();

        let total_bytes = data.len() * iterations;
        total_bytes as f64 / elapsed.as_secs_f64()
    }

    /// Run file-based benchmark
    pub fn bench_file(path: &Path) -> Result<()> {
        println!("{} Benchmarking file: {}", style("⚡").bold(), path.display());

        let data = std::fs::read(path)?;
        let size = data.len();

        // Hash
        let start = Instant::now();
        let hash = compute_blake3(&data);
        let hash_time = start.elapsed();

        println!();
        println!("  File size: {} bytes", size);
        println!("  Hash: {:?}", hash_time);
        println!("  Blake3: {:02x?}", hash);

        Ok(())
    }
}
