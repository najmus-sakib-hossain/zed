use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use rkyv::{Archive, Deserialize, Serialize};
use serializer::machine::optimized_rkyv::OptimizedRkyv;
use std::path::PathBuf;
use tempfile::TempDir;

#[derive(Archive, Serialize, Deserialize, Clone, Debug)]
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

fn bench_adaptive_file_io(c: &mut Criterion) {
    let mut group = c.benchmark_group("adaptive_file_io");
    let opt = OptimizedRkyv::new();
    let dir = TempDir::new().unwrap();

    // Test different file sizes to verify adaptive strategy
    for size in [100, 512, 1024, 2048, 4096, 8192].iter() {
        let data = TestData::new(1, *size);
        let path = dir.path().join(format!("test_{}.rkyv", size));

        group.bench_with_input(BenchmarkId::new("write", size), size, |b, _| {
            b.iter(|| {
                opt.serialize_to_file(black_box(&data), black_box(&path)).unwrap();
            });
        });

        // Write once for read benchmark
        opt.serialize_to_file(&data, &path).unwrap();

        group.bench_with_input(BenchmarkId::new("read", size), size, |b, _| {
            b.iter(|| {
                let _: TestData = opt.deserialize_from_file(black_box(&path)).unwrap();
            });
        });
    }

    group.finish();
}

fn bench_adaptive_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("adaptive_batch");
    let opt = OptimizedRkyv::new();

    // Test different batch sizes to verify adaptive strategy
    for count in [10, 100, 1000, 10000].iter() {
        let items: Vec<TestData> = (0..*count).map(|i| TestData::new(i, 50)).collect();

        group.bench_with_input(BenchmarkId::new("serialize", count), count, |b, _| {
            b.iter(|| {
                opt.serialize_batch_smart(black_box(&items)).unwrap();
            });
        });
    }

    group.finish();
}

#[cfg(feature = "compression")]
fn bench_adaptive_compression(c: &mut Criterion) {
    use serializer::machine::compress::CompressionLevel;
    use serializer::machine::optimized_rkyv::CompressedRkyv;

    let mut group = c.benchmark_group("adaptive_compression");
    let mut comp = CompressedRkyv::new(CompressionLevel::Fast);

    // Test different data sizes to verify compression benefit threshold
    for size in [50, 100, 200, 500, 1000, 5000].iter() {
        let data = TestData::new(1, *size);

        group.bench_with_input(BenchmarkId::new("compress", size), size, |b, _| {
            b.iter(|| {
                comp.serialize_compressed(black_box(&data)).unwrap();
            });
        });

        let compressed = comp.serialize_compressed(&data).unwrap();

        group.bench_with_input(BenchmarkId::new("decompress", size), size, |b, _| {
            b.iter(|| {
                let _: TestData = comp.deserialize_compressed(black_box(&compressed)).unwrap();
            });
        });
    }

    group.finish();
}

#[cfg(feature = "arena")]
fn bench_adaptive_arena(c: &mut Criterion) {
    use serializer::machine::optimized_rkyv::ArenaRkyv;

    let mut group = c.benchmark_group("adaptive_arena");

    for count in [10, 100, 1000].iter() {
        let items: Vec<TestData> = (0..*count).map(|i| TestData::new(i, 50)).collect();
        let mut arena = ArenaRkyv::new();

        group.bench_with_input(BenchmarkId::new("serialize", count), count, |b, _| {
            b.iter(|| {
                arena.serialize_batch(black_box(&items)).unwrap();
                arena.reset();
            });
        });
    }

    group.finish();
}

fn bench_batch_file_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_file_operations");
    let opt = OptimizedRkyv::new();
    let dir = TempDir::new().unwrap();

    for count in [10, 50, 100].iter() {
        let items: Vec<(TestData, PathBuf)> = (0..*count)
            .map(|i| {
                let data = TestData::new(i, 100);
                let path = dir.path().join(format!("batch_{}.rkyv", i));
                (data, path)
            })
            .collect();

        let items_ref: Vec<_> = items.iter().map(|(d, p)| (d.clone(), p.as_path())).collect();

        group.bench_with_input(BenchmarkId::new("write_batch", count), count, |b, _| {
            b.iter(|| {
                let items_clone: Vec<_> =
                    items.iter().map(|(d, p)| (d.clone(), p.as_path())).collect();
                opt.serialize_batch_to_files(black_box(&items_clone)).unwrap();
            });
        });

        // Write once for read benchmark
        opt.serialize_batch_to_files(&items_ref).unwrap();

        let paths: Vec<_> = items.iter().map(|(_, p)| p.as_path()).collect();

        group.bench_with_input(BenchmarkId::new("read_batch", count), count, |b, _| {
            b.iter(|| {
                let _: Vec<Result<TestData, _>> =
                    opt.deserialize_batch_from_files(black_box(&paths)).unwrap();
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_adaptive_file_io,
    bench_adaptive_batch,
    bench_batch_file_operations,
);

#[cfg(feature = "compression")]
criterion_group!(compression_benches, bench_adaptive_compression);

#[cfg(feature = "arena")]
criterion_group!(arena_benches, bench_adaptive_arena);

#[cfg(all(feature = "compression", feature = "arena"))]
criterion_main!(benches, compression_benches, arena_benches);

#[cfg(all(feature = "compression", not(feature = "arena")))]
criterion_main!(benches, compression_benches);

#[cfg(all(not(feature = "compression"), feature = "arena"))]
criterion_main!(benches, arena_benches);

#[cfg(not(any(feature = "compression", feature = "arena")))]
criterion_main!(benches);
