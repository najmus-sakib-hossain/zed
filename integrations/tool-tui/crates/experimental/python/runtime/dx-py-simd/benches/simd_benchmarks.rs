//! SIMD String Operation Benchmarks
//!
//! Benchmarks comparing SIMD vs scalar string operations.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use dx_py_simd::scalar::ScalarStringEngine;
use dx_py_simd::{get_engine, SimdStringEngine};

fn bench_find(c: &mut Criterion) {
    let simd_engine = get_engine();
    let scalar_engine = ScalarStringEngine::new();

    let mut group = c.benchmark_group("string_find");

    for size in [100, 1000, 10000, 100000].iter() {
        let haystack: String = "a".repeat(*size - 5) + "needle";
        let needle = "needle";

        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(BenchmarkId::new("simd", size), size, |b, _| {
            b.iter(|| simd_engine.find(&haystack, needle))
        });

        group.bench_with_input(BenchmarkId::new("scalar", size), size, |b, _| {
            b.iter(|| scalar_engine.find(&haystack, needle))
        });
    }

    group.finish();
}

fn bench_count(c: &mut Criterion) {
    let simd_engine = get_engine();
    let scalar_engine = ScalarStringEngine::new();

    let mut group = c.benchmark_group("string_count");

    for size in [100, 1000, 10000].iter() {
        let haystack: String = "ab".repeat(*size / 2);
        let needle = "ab";

        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(BenchmarkId::new("simd", size), size, |b, _| {
            b.iter(|| simd_engine.count(&haystack, needle))
        });

        group.bench_with_input(BenchmarkId::new("scalar", size), size, |b, _| {
            b.iter(|| scalar_engine.count(&haystack, needle))
        });
    }

    group.finish();
}

fn bench_eq(c: &mut Criterion) {
    let simd_engine = get_engine();
    let scalar_engine = ScalarStringEngine::new();

    let mut group = c.benchmark_group("string_eq");

    for size in [100, 1000, 10000, 100000].iter() {
        let a: String = "a".repeat(*size);
        let b: String = "a".repeat(*size);

        group.throughput(Throughput::Bytes(*size as u64 * 2));

        group.bench_with_input(BenchmarkId::new("simd", size), size, |b_iter, _| {
            b_iter.iter(|| simd_engine.eq(&a, &b))
        });

        group.bench_with_input(BenchmarkId::new("scalar", size), size, |b_iter, _| {
            b_iter.iter(|| scalar_engine.eq(&a, &b))
        });
    }

    group.finish();
}

fn bench_to_lowercase(c: &mut Criterion) {
    let simd_engine = get_engine();
    let scalar_engine = ScalarStringEngine::new();

    let mut group = c.benchmark_group("to_lowercase");

    for size in [100, 1000, 10000, 100000].iter() {
        let input: String = "HELLO WORLD ".repeat(*size / 12);

        group.throughput(Throughput::Bytes(input.len() as u64));

        group.bench_with_input(BenchmarkId::new("simd", size), size, |b, _| {
            b.iter(|| simd_engine.to_lowercase(&input))
        });

        group.bench_with_input(BenchmarkId::new("scalar", size), size, |b, _| {
            b.iter(|| scalar_engine.to_lowercase(&input))
        });
    }

    group.finish();
}

fn bench_to_uppercase(c: &mut Criterion) {
    let simd_engine = get_engine();
    let scalar_engine = ScalarStringEngine::new();

    let mut group = c.benchmark_group("to_uppercase");

    for size in [100, 1000, 10000, 100000].iter() {
        let input: String = "hello world ".repeat(*size / 12);

        group.throughput(Throughput::Bytes(input.len() as u64));

        group.bench_with_input(BenchmarkId::new("simd", size), size, |b, _| {
            b.iter(|| simd_engine.to_uppercase(&input))
        });

        group.bench_with_input(BenchmarkId::new("scalar", size), size, |b, _| {
            b.iter(|| scalar_engine.to_uppercase(&input))
        });
    }

    group.finish();
}

fn bench_split(c: &mut Criterion) {
    let simd_engine = get_engine();
    let scalar_engine = ScalarStringEngine::new();

    let mut group = c.benchmark_group("string_split");

    for size in [100, 1000, 10000].iter() {
        let input: String = (0..*size).map(|i| format!("item{}", i)).collect::<Vec<_>>().join(",");

        group.throughput(Throughput::Bytes(input.len() as u64));

        group.bench_with_input(BenchmarkId::new("simd", size), size, |b, _| {
            b.iter(|| simd_engine.split(&input, ","))
        });

        group.bench_with_input(BenchmarkId::new("scalar", size), size, |b, _| {
            b.iter(|| scalar_engine.split(&input, ","))
        });
    }

    group.finish();
}

fn bench_replace(c: &mut Criterion) {
    let simd_engine = get_engine();
    let scalar_engine = ScalarStringEngine::new();

    let mut group = c.benchmark_group("string_replace");

    for size in [100, 1000, 10000].iter() {
        let input: String = "hello world ".repeat(*size / 12);

        group.throughput(Throughput::Bytes(input.len() as u64));

        group.bench_with_input(BenchmarkId::new("simd", size), size, |b, _| {
            b.iter(|| simd_engine.replace(&input, "world", "rust"))
        });

        group.bench_with_input(BenchmarkId::new("scalar", size), size, |b, _| {
            b.iter(|| scalar_engine.replace(&input, "world", "rust"))
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_find,
    bench_count,
    bench_eq,
    bench_to_lowercase,
    bench_to_uppercase,
    bench_split,
    bench_replace,
);

criterion_main!(benches);
