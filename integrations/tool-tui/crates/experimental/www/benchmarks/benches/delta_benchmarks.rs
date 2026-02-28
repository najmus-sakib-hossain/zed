//! # Delta Patch Benchmarks
//!
//! Benchmarks for delta patch generation and application.
//!
//! Run with: `cargo bench --bench delta_benchmarks -p dx-www-benchmarks`

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use dx_www_binary::delta::{apply_delta, generate_delta, generate_delta_with_block_size};

/// Generate test data with a specific similarity ratio
fn generate_similar_data(size: usize, similarity: f64) -> (Vec<u8>, Vec<u8>) {
    let base: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
    let mut target = base.clone();

    // Modify a portion of the target based on similarity
    let changes = ((1.0 - similarity) * size as f64) as usize;
    for i in 0..changes {
        let idx = (i * 7) % size; // Spread changes throughout
        target[idx] = target[idx].wrapping_add(1);
    }

    (base, target)
}

/// Generate completely different data
fn generate_different_data(size: usize) -> (Vec<u8>, Vec<u8>) {
    let base: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
    let target: Vec<u8> = (0..size).map(|i| ((i + 128) % 256) as u8).collect();
    (base, target)
}

/// Benchmark delta patch generation
fn bench_delta_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("delta_generation");

    // Test different data sizes
    let sizes = [1024, 10 * 1024, 100 * 1024, 1024 * 1024];

    for size in sizes {
        let (base, target) = generate_similar_data(size, 0.9); // 90% similar

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(
            BenchmarkId::new("similar_90pct", format!("{}KB", size / 1024)),
            &(base.clone(), target.clone()),
            |b, (base, target)| {
                b.iter(|| {
                    black_box(generate_delta(black_box(base), black_box(target)).expect("generate"))
                });
            },
        );
    }

    // Test with different similarity ratios
    let size = 100 * 1024; // 100KB
    for similarity in [0.5, 0.75, 0.95, 0.99] {
        let (base, target) = generate_similar_data(size, similarity);

        group.bench_with_input(
            BenchmarkId::new("similarity", format!("{}pct", (similarity * 100.0) as u32)),
            &(base, target),
            |b, (base, target)| {
                b.iter(|| {
                    black_box(generate_delta(black_box(base), black_box(target)).expect("generate"))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark delta patch application
fn bench_delta_application(c: &mut Criterion) {
    let mut group = c.benchmark_group("delta_application");

    let sizes = [1024, 10 * 1024, 100 * 1024, 1024 * 1024];

    for size in sizes {
        let (base, target) = generate_similar_data(size, 0.9);
        let patch = generate_delta(&base, &target).expect("generate");

        group.throughput(Throughput::Bytes(target.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("apply", format!("{}KB", size / 1024)),
            &(base.clone(), patch),
            |b, (base, patch)| {
                b.iter(|| {
                    black_box(apply_delta(black_box(base), black_box(patch)).expect("apply"))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark different block sizes
fn bench_block_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("delta_block_sizes");

    let size = 100 * 1024; // 100KB
    let (base, target) = generate_similar_data(size, 0.9);

    let block_sizes = [32, 64, 128, 256, 512];

    for block_size in block_sizes {
        group.bench_with_input(
            BenchmarkId::new("generate", format!("block_{}", block_size)),
            &block_size,
            |b, &block_size| {
                b.iter(|| {
                    black_box(
                        generate_delta_with_block_size(
                            black_box(&base),
                            black_box(&target),
                            block_size,
                        )
                        .expect("generate"),
                    )
                });
            },
        );
    }

    group.finish();
}

/// Benchmark compression ratio
fn bench_compression_ratio(c: &mut Criterion) {
    let mut group = c.benchmark_group("delta_compression");

    let size = 100 * 1024;

    // Test with completely different data (worst case)
    let (base_diff, target_diff) = generate_different_data(size);
    let patch_diff = generate_delta(&base_diff, &target_diff).expect("generate");

    group.bench_function("worst_case_100KB", |b| {
        b.iter(|| {
            black_box(apply_delta(black_box(&base_diff), black_box(&patch_diff)).expect("apply"))
        });
    });

    // Test with identical data (best case)
    let base_same: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
    let target_same = base_same.clone();
    let patch_same = generate_delta(&base_same, &target_same).expect("generate");

    group.bench_function("best_case_100KB", |b| {
        b.iter(|| {
            black_box(apply_delta(black_box(&base_same), black_box(&patch_same)).expect("apply"))
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_delta_generation,
    bench_delta_application,
    bench_block_sizes,
    bench_compression_ratio
);

criterion_main!(benches);
