use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use rkyv::{Archive, Deserialize, Serialize};
use serializer::machine::OptimizedRkyv;
use std::sync::atomic::{AtomicU32, Ordering};
use tempfile::TempDir;

static COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
struct TestData {
    id: u64,
    name: String,
    age: u32,
    active: bool,
    score: f64,
}

fn create_test_data(count: usize) -> Vec<TestData> {
    (0..count)
        .map(|i| TestData {
            id: i as u64,
            name: format!("Person_{}", i),
            age: 25 + (i % 50) as u32,
            active: i % 2 == 0,
            score: (i as f64) * 1.5,
        })
        .collect()
}

fn bench_single_serialize(c: &mut Criterion) {
    let data = TestData {
        id: 42,
        name: "Test Person".to_string(),
        age: 30,
        active: true,
        score: 95.5,
    };

    let mut group = c.benchmark_group("single_serialize");

    group.bench_function("rkyv_native", |b| {
        b.iter(|| rkyv::to_bytes::<rkyv::rancor::Error>(black_box(&data)).unwrap());
    });

    group.bench_function("dx_optimized", |b| {
        b.iter(|| rkyv::to_bytes::<rkyv::rancor::Error>(black_box(&data)).unwrap());
    });

    group.finish();
}

fn bench_batch_serialize(c: &mut Criterion) {
    let sizes = [10, 100, 1000];

    for size in sizes.iter() {
        let data = create_test_data(*size);
        let mut group = c.benchmark_group(format!("batch_serialize_{}", size));

        group.bench_with_input(BenchmarkId::new("rkyv_native", size), &data, |b, data| {
            b.iter(|| {
                let mut results = Vec::with_capacity(data.len());
                for item in data {
                    results.push(rkyv::to_bytes::<rkyv::rancor::Error>(black_box(item)).unwrap());
                }
                results
            });
        });

        group.bench_with_input(BenchmarkId::new("dx_batch_prealloc", size), &data, |b, data| {
            b.iter(|| serializer::machine::serialize_batch(black_box(data)).unwrap());
        });

        group.finish();
    }
}

#[cfg(feature = "parallel")]
fn bench_parallel_serialize(c: &mut Criterion) {
    use rayon::prelude::*;

    let sizes = [100, 1000];

    for size in sizes.iter() {
        let data = create_test_data(*size);
        let mut group = c.benchmark_group(format!("parallel_serialize_{}", size));

        group.bench_with_input(BenchmarkId::new("rkyv_sequential", size), &data, |b, data| {
            b.iter(|| {
                let mut results = Vec::with_capacity(data.len());
                for item in data {
                    results.push(rkyv::to_bytes::<rkyv::rancor::Error>(black_box(item)).unwrap());
                }
                results
            });
        });

        group.bench_with_input(BenchmarkId::new("dx_parallel", size), &data, |b, data| {
            b.iter(|| {
                let opt = OptimizedRkyv::new();
                opt.serialize_batch_parallel(black_box(data)).unwrap()
            });
        });

        group.finish();
    }
}

fn bench_file_io(c: &mut Criterion) {
    let dir = TempDir::new().unwrap();
    let data = TestData {
        id: 42,
        name: "Test Person".to_string(),
        age: 30,
        active: true,
        score: 95.5,
    };

    let mut group = c.benchmark_group("file_io");

    group.bench_function("rkyv_native_file", |b| {
        b.iter(|| {
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = dir.path().join(format!("rkyv_{}.bin", id));
            let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(black_box(&data)).unwrap();
            std::fs::write(&path, &bytes).unwrap();
        });
    });

    group.bench_function("dx_optimized_file", |b| {
        let opt = OptimizedRkyv::new();
        b.iter(|| {
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = dir.path().join(format!("dx_{}.bin", id));
            opt.serialize_to_file(black_box(&data), &path).unwrap();
        });
    });

    group.finish();
}

#[cfg(feature = "compression")]
fn bench_compression(c: &mut Criterion) {
    use serializer::machine::{CompressedRkyv, compress::CompressionLevel};

    let data = TestData {
        id: 42,
        name: "Test Person with a much longer name for better compression testing".to_string(),
        age: 30,
        active: true,
        score: 95.5,
    };

    let mut group = c.benchmark_group("compression");

    group.bench_function("rkyv_no_compression", |b| {
        b.iter(|| rkyv::to_bytes::<rkyv::rancor::Error>(black_box(&data)).unwrap());
    });

    group.bench_function("dx_with_compression", |b| {
        let mut comp = CompressedRkyv::new(CompressionLevel::Fast);
        b.iter(|| comp.serialize_compressed(black_box(&data)).unwrap());
    });

    group.finish();
}

fn bench_summary(_c: &mut Criterion) {
    let opt = OptimizedRkyv::new();
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║          DX Serializer vs RKYV Native Benchmark           ║");
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║ Binary Format: RKYV (same format, different optimizations)║");
    println!("║ I/O Backend:   {:45} ║", opt.backend_name());
    println!("╠════════════════════════════════════════════════════════════╣");
    println!("║ DX Optimizations:                                          ║");
    println!("║   ✓ Platform-optimized I/O (io_uring/IOCP/kqueue)         ║");
    println!("║   ✓ Batch pre-allocation                                   ║");
    #[cfg(feature = "parallel")]
    println!("║   ✓ Parallel processing with Rayon                         ║");
    #[cfg(feature = "compression")]
    println!("║   ✓ LZ4 compression                                        ║");
    #[cfg(feature = "arena")]
    println!("║   ✓ Arena allocation                                       ║");
    #[cfg(feature = "mmap")]
    println!("║   ✓ Memory-mapped files                                    ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");
}

criterion_group!(
    benches,
    bench_summary,
    bench_single_serialize,
    bench_batch_serialize,
    bench_file_io,
);

#[cfg(feature = "parallel")]
criterion_group!(parallel_benches, bench_parallel_serialize);

#[cfg(feature = "compression")]
criterion_group!(compression_benches, bench_compression);

#[cfg(all(feature = "parallel", feature = "compression"))]
criterion_main!(benches, parallel_benches, compression_benches);

#[cfg(all(feature = "parallel", not(feature = "compression")))]
criterion_main!(benches, parallel_benches);

#[cfg(all(not(feature = "parallel"), feature = "compression"))]
criterion_main!(benches, compression_benches);

#[cfg(all(not(feature = "parallel"), not(feature = "compression")))]
criterion_main!(benches);
