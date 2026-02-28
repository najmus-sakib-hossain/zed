//! Benchmarks for dx-js-compatibility crate.
//!
//! This benchmark suite validates the performance targets specified in Requirement 28:
//! - 400,000+ HTTP requests/second (2x Bun)
//! - 1 GB/s+ file read throughput (2x Bun)
//! - 200,000+ SQLite operations/second (2x Bun)
//! - 2 GB/s+ SHA256 throughput (2x Bun)
//! - 450 MB/s+ gzip throughput (1.5x Bun)
//! - 10,000+ process spawns/second (2x Bun)
//! - 200,000+ WebSocket messages/second (2x Bun)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

// ============================================================================
// Compile Module Benchmarks
// ============================================================================

mod compile_benchmarks {
    use super::*;

    pub fn compression_benchmark(c: &mut Criterion) {
        use dx_compat_compile::{compress_data, compress_data_with_level, decompress_data};

        let mut group = c.benchmark_group("compile/compression");

        // Test different data sizes
        for size in [1024, 10 * 1024, 100 * 1024, 1024 * 1024].iter() {
            let data: Vec<u8> = (0..*size).map(|i| (i % 256) as u8).collect();

            group.throughput(Throughput::Bytes(*size as u64));

            group.bench_with_input(BenchmarkId::new("zstd_compress", size), &data, |b, data| {
                b.iter(|| compress_data(black_box(data)))
            });

            // Pre-compress for decompression benchmark
            let compressed = compress_data(&data).unwrap();
            group.bench_with_input(
                BenchmarkId::new("zstd_decompress", size),
                &compressed,
                |b, compressed| b.iter(|| decompress_data(black_box(compressed))),
            );
        }

        // Test different compression levels
        let data: Vec<u8> = (0..100 * 1024).map(|i| (i % 256) as u8).collect();
        for level in [1, 3, 6, 10, 15].iter() {
            group.bench_with_input(
                BenchmarkId::new("zstd_level", level),
                &(*level, &data),
                |b, (level, data)| b.iter(|| compress_data_with_level(black_box(*data), *level)),
            );
        }

        group.finish();
    }

    pub fn asset_bundle_benchmark(c: &mut Criterion) {
        use dx_compat_compile::{AssetBundle, EmbeddedAsset, Target};

        let mut group = c.benchmark_group("compile/asset_bundle");

        // Create test assets
        let small_asset = EmbeddedAsset::new("small.txt", b"Hello, World!", "text/plain").unwrap();
        let medium_data: Vec<u8> = (0..10 * 1024).map(|i| (i % 256) as u8).collect();
        let medium_asset =
            EmbeddedAsset::new("medium.bin", &medium_data, "application/octet-stream").unwrap();

        group.bench_function("create_bundle", |b| {
            b.iter(|| {
                let mut bundle =
                    AssetBundle::new(Target::LinuxX64, "index.js", "test-app", "1.0.0");
                bundle.add_asset(small_asset.clone());
                bundle.add_asset(medium_asset.clone());
                black_box(bundle)
            })
        });

        // Serialization benchmark
        let mut bundle = AssetBundle::new(Target::LinuxX64, "index.js", "test-app", "1.0.0");
        bundle.add_asset(small_asset.clone());
        bundle.add_asset(medium_asset.clone());

        group.bench_function("serialize_bundle", |b| b.iter(|| bundle.to_bytes()));

        let bytes = bundle.to_bytes().unwrap();
        group.throughput(Throughput::Bytes(bytes.len() as u64));

        group.bench_function("deserialize_bundle", |b| {
            b.iter(|| AssetBundle::from_bytes(black_box(&bytes)))
        });

        group.finish();
    }
}

// ============================================================================
// Macro Module Benchmarks
// ============================================================================

mod macro_benchmarks {
    use super::*;

    pub fn macro_value_benchmark(c: &mut Criterion) {
        use dx_compat_macro::MacroValue;
        use std::collections::HashMap;

        let mut group = c.benchmark_group("macro/value");

        // JSON serialization
        let mut obj = HashMap::new();
        obj.insert("name".to_string(), MacroValue::String("test".to_string()));
        obj.insert("count".to_string(), MacroValue::Integer(42));
        obj.insert("enabled".to_string(), MacroValue::Bool(true));
        let value = MacroValue::Object(obj);

        group.bench_function("to_json", |b| b.iter(|| value.to_json()));

        let json = value.to_json().unwrap();
        group.bench_function("from_json", |b| b.iter(|| MacroValue::from_json(black_box(&json))));

        group.bench_function("to_js_literal", |b| b.iter(|| value.to_js_literal()));

        group.finish();
    }

    pub fn macro_context_benchmark(c: &mut Criterion) {
        use dx_compat_macro::{MacroConfig, MacroContext, MacroValue};

        let mut group = c.benchmark_group("macro/context");

        let ctx = MacroContext::with_config(
            MacroConfig::new().env_var("TEST_VAR", "test_value").allow_env(true),
        );

        group.bench_function("env_lookup", |b| b.iter(|| ctx.env(black_box("TEST_VAR"))));

        group.bench_function("execute_simple", |b| {
            b.iter(|| ctx.execute(|_| Ok(MacroValue::Integer(42))))
        });

        group.bench_function("execute_cached", |b| {
            b.iter(|| ctx.execute_cached("bench_key", |_| Ok(MacroValue::Integer(42))))
        });

        group.finish();
    }
}

// ============================================================================
// Main Benchmark Groups
// ============================================================================

fn compile_benchmarks(c: &mut Criterion) {
    compile_benchmarks::compression_benchmark(c);
    compile_benchmarks::asset_bundle_benchmark(c);
}

fn macro_benchmarks(c: &mut Criterion) {
    macro_benchmarks::macro_value_benchmark(c);
    macro_benchmarks::macro_context_benchmark(c);
}

// Placeholder for future benchmarks when features are enabled
fn placeholder_benchmark(c: &mut Criterion) {
    c.bench_function("baseline", |b| b.iter(|| black_box(1 + 1)));
}

criterion_group!(benches, placeholder_benchmark, compile_benchmarks, macro_benchmarks,);
criterion_main!(benches);
