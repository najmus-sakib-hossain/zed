use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rkyv::{Archive, Deserialize, Serialize};
use serializer::machine::optimized_rkyv::OptimizedRkyv;
use std::path::PathBuf;
use tempfile::TempDir;

#[derive(Archive, Serialize, Deserialize, Clone, Debug, PartialEq)]
struct TestData {
    id: u64,
    name: String,
    data: Vec<u8>,
}

impl TestData {
    fn new(id: u64, size: usize) -> Self {
        Self {
            id,
            name: format!("item_{}", id),
            data: vec![0u8; size],
        }
    }
}

fn bench_single_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_serialize");
    let data = TestData::new(1, 100);

    group.bench_function("rkyv_native", |b| {
        b.iter(|| {
            let _bytes = rkyv::to_bytes::<rkyv::rancor::Error>(black_box(&data)).unwrap();
        });
    });

    group.bench_function("dx_adaptive", |b| {
        b.iter(|| {
            let _bytes = rkyv::to_bytes::<rkyv::rancor::Error>(black_box(&data)).unwrap();
        });
    });

    group.finish();
}

fn bench_batch_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_serialize");
    let opt = OptimizedRkyv::new();

    // Small batch (10) - should use pre-allocation
    let items_10: Vec<TestData> = (0..10).map(|i| TestData::new(i, 50)).collect();

    group.bench_function("rkyv_native/10", |b| {
        b.iter(|| {
            let mut results = Vec::new();
            for item in black_box(&items_10) {
                results.push(rkyv::to_bytes::<rkyv::rancor::Error>(item).unwrap());
            }
        });
    });

    group.bench_function("dx_adaptive/10", |b| {
        b.iter(|| {
            opt.serialize_batch_smart(black_box(&items_10)).unwrap();
        });
    });

    // Medium batch (100) - should use pre-allocation
    let items_100: Vec<TestData> = (0..100).map(|i| TestData::new(i, 50)).collect();

    group.bench_function("rkyv_native/100", |b| {
        b.iter(|| {
            let mut results = Vec::new();
            for item in black_box(&items_100) {
                results.push(rkyv::to_bytes::<rkyv::rancor::Error>(item).unwrap());
            }
        });
    });

    group.bench_function("dx_adaptive/100", |b| {
        b.iter(|| {
            opt.serialize_batch_smart(black_box(&items_100)).unwrap();
        });
    });

    // Large batch (1000) - should use pre-allocation
    let items_1000: Vec<TestData> = (0..1000).map(|i| TestData::new(i, 50)).collect();

    group.bench_function("rkyv_native/1000", |b| {
        b.iter(|| {
            let mut results = Vec::new();
            for item in black_box(&items_1000) {
                results.push(rkyv::to_bytes::<rkyv::rancor::Error>(item).unwrap());
            }
        });
    });

    group.bench_function("dx_adaptive/1000", |b| {
        b.iter(|| {
            opt.serialize_batch_smart(black_box(&items_1000)).unwrap();
        });
    });

    group.finish();
}

fn bench_file_io(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_io");
    let dir = TempDir::new().unwrap();
    let opt = OptimizedRkyv::new();

    // Small file (<1KB) - should use std::fs
    let small_data = TestData::new(1, 100);
    let small_path = dir.path().join("small.rkyv");

    group.bench_function("rkyv_native_small", |b| {
        b.iter(|| {
            let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(black_box(&small_data)).unwrap();
            std::fs::write(black_box(&small_path), &bytes).unwrap();
        });
    });

    group.bench_function("dx_adaptive_small", |b| {
        b.iter(|| {
            opt.serialize_to_file(black_box(&small_data), black_box(&small_path)).unwrap();
        });
    });

    // Large file (>1KB) - should use platform I/O
    let large_data = TestData::new(1, 2000);
    let large_path = dir.path().join("large.rkyv");

    group.bench_function("rkyv_native_large", |b| {
        b.iter(|| {
            let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(black_box(&large_data)).unwrap();
            std::fs::write(black_box(&large_path), &bytes).unwrap();
        });
    });

    group.bench_function("dx_adaptive_large", |b| {
        b.iter(|| {
            opt.serialize_to_file(black_box(&large_data), black_box(&large_path)).unwrap();
        });
    });

    group.finish();
}

#[cfg(feature = "parallel")]
fn bench_parallel_batch(c: &mut Criterion) {
    use rayon::prelude::*;

    let mut group = c.benchmark_group("parallel_batch");
    let opt = OptimizedRkyv::new();

    // Huge batch (10k+) - should use parallel
    let items: Vec<TestData> = (0..15000).map(|i| TestData::new(i, 50)).collect();

    group.bench_function("rkyv_sequential", |b| {
        b.iter(|| {
            let mut results = Vec::new();
            for item in black_box(&items) {
                results.push(rkyv::to_bytes::<rkyv::rancor::Error>(item).unwrap());
            }
        });
    });

    group.bench_function("rkyv_parallel", |b| {
        b.iter(|| {
            let _results: Vec<_> = black_box(&items)
                .par_iter()
                .map(|item| rkyv::to_bytes::<rkyv::rancor::Error>(item).unwrap())
                .collect();
        });
    });

    group.bench_function("dx_adaptive", |b| {
        b.iter(|| {
            opt.serialize_batch_smart(black_box(&items)).unwrap();
        });
    });

    group.finish();
}

#[cfg(feature = "compression")]
fn bench_compression(c: &mut Criterion) {
    use serializer::machine::compress::CompressionLevel;
    use serializer::machine::optimized_rkyv::CompressedRkyv;

    let mut group = c.benchmark_group("compression");
    let mut comp = CompressedRkyv::new(CompressionLevel::Fast);

    // Small data (<100 bytes) - should skip compression
    let small = TestData::new(1, 50);

    group.bench_function("rkyv_no_compression_small", |b| {
        b.iter(|| {
            let _bytes = rkyv::to_bytes::<rkyv::rancor::Error>(black_box(&small)).unwrap();
        });
    });

    group.bench_function("dx_adaptive_small", |b| {
        b.iter(|| {
            comp.serialize_compressed(black_box(&small)).unwrap();
        });
    });

    // Large data (>100 bytes) - should compress
    let large = TestData::new(1, 500);

    group.bench_function("rkyv_no_compression_large", |b| {
        b.iter(|| {
            let _bytes = rkyv::to_bytes::<rkyv::rancor::Error>(black_box(&large)).unwrap();
        });
    });

    group.bench_function("dx_adaptive_large", |b| {
        b.iter(|| {
            comp.serialize_compressed(black_box(&large)).unwrap();
        });
    });

    group.finish();
}

fn bench_batch_file_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_file_operations");
    let dir = TempDir::new().unwrap();
    let opt = OptimizedRkyv::new();

    let items: Vec<(TestData, PathBuf)> = (0..50)
        .map(|i| {
            let data = TestData::new(i, 100);
            let path = dir.path().join(format!("batch_{}.rkyv", i));
            (data, path)
        })
        .collect();

    group.bench_function("rkyv_native", |b| {
        b.iter(|| {
            for (data, path) in black_box(&items) {
                let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(data).unwrap();
                std::fs::write(path, &bytes).unwrap();
            }
        });
    });

    group.bench_function("dx_adaptive", |b| {
        b.iter(|| {
            let items_ref: Vec<_> = items.iter().map(|(d, p)| (d.clone(), p.as_path())).collect();
            opt.serialize_batch_to_files(black_box(&items_ref)).unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_single_serialize,
    bench_batch_serialize,
    bench_file_io,
    bench_batch_file_operations,
);

#[cfg(feature = "parallel")]
criterion_group!(parallel_benches, bench_parallel_batch);

#[cfg(feature = "compression")]
criterion_group!(compression_benches, bench_compression);

#[cfg(all(feature = "parallel", feature = "compression"))]
criterion_main!(benches, parallel_benches, compression_benches);

#[cfg(all(feature = "parallel", not(feature = "compression")))]
criterion_main!(benches, parallel_benches);

#[cfg(all(not(feature = "parallel"), feature = "compression"))]
criterion_main!(benches, compression_benches);

#[cfg(not(any(feature = "parallel", feature = "compression")))]
criterion_main!(benches);
