use criterion::{Criterion, black_box, criterion_group, criterion_main};
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
}

fn create_test_data(count: usize) -> Vec<TestData> {
    (0..count)
        .map(|i| TestData {
            id: i as u64,
            name: format!("Person_{}", i),
            age: 25 + (i % 50) as u32,
            active: i % 2 == 0,
        })
        .collect()
}

fn bench_optimized_io(c: &mut Criterion) {
    let dir = TempDir::new().unwrap();
    let opt = OptimizedRkyv::new();
    let data = TestData {
        id: 42,
        name: "Test Person".to_string(),
        age: 30,
        active: true,
    };

    c.bench_function("optimized_rkyv_file_write", |b| {
        b.iter(|| {
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = dir.path().join(format!("test_{}.rkyv", id));
            opt.serialize_to_file(black_box(&data), &path).unwrap();
        });
    });
}

#[cfg(feature = "parallel")]
fn bench_parallel_batch(c: &mut Criterion) {
    let opt = OptimizedRkyv::new();
    let data = create_test_data(1000);

    c.bench_function("optimized_rkyv_parallel_1000", |b| {
        b.iter(|| opt.serialize_batch_parallel(black_box(&data)).unwrap());
    });
}

#[cfg(feature = "compression")]
fn bench_compressed(c: &mut Criterion) {
    use serializer::machine::{CompressedRkyv, compress::CompressionLevel};

    let mut comp = CompressedRkyv::new(CompressionLevel::Fast);
    let data = TestData {
        id: 42,
        name: "Test Person with a longer name for better compression".to_string(),
        age: 30,
        active: true,
    };

    c.bench_function("compressed_rkyv_serialize", |b| {
        b.iter(|| comp.serialize_compressed(black_box(&data)).unwrap());
    });
}

#[cfg(feature = "arena")]
fn bench_arena(c: &mut Criterion) {
    use serializer::machine::ArenaRkyv;

    let mut arena = ArenaRkyv::new();
    let data = create_test_data(100);

    c.bench_function("arena_rkyv_batch_100", |b| {
        b.iter(|| {
            arena.serialize_batch(black_box(&data)).unwrap();
            arena.reset();
        });
    });
}

fn bench_backend_info(_c: &mut Criterion) {
    let opt = OptimizedRkyv::new();
    println!("\n=== DX Optimized RKYV ===");
    println!("I/O Backend: {}", opt.backend_name());
    println!("========================\n");
}

criterion_group!(benches, bench_backend_info, bench_optimized_io,);

#[cfg(feature = "parallel")]
criterion_group!(parallel_benches, bench_parallel_batch);

#[cfg(feature = "compression")]
criterion_group!(compression_benches, bench_compressed);

#[cfg(feature = "arena")]
criterion_group!(arena_benches, bench_arena);

#[cfg(all(feature = "parallel", feature = "compression", feature = "arena"))]
criterion_main!(benches, parallel_benches, compression_benches, arena_benches);

#[cfg(all(feature = "parallel", feature = "compression", not(feature = "arena")))]
criterion_main!(benches, parallel_benches, compression_benches);

#[cfg(all(feature = "parallel", not(feature = "compression"), feature = "arena"))]
criterion_main!(benches, parallel_benches, arena_benches);

#[cfg(all(not(feature = "parallel"), feature = "compression", feature = "arena"))]
criterion_main!(benches, compression_benches, arena_benches);

#[cfg(all(
    feature = "parallel",
    not(feature = "compression"),
    not(feature = "arena")
))]
criterion_main!(benches, parallel_benches);

#[cfg(all(
    not(feature = "parallel"),
    feature = "compression",
    not(feature = "arena")
))]
criterion_main!(benches, compression_benches);

#[cfg(all(
    not(feature = "parallel"),
    not(feature = "compression"),
    feature = "arena"
))]
criterion_main!(benches, arena_benches);

#[cfg(all(
    not(feature = "parallel"),
    not(feature = "compression"),
    not(feature = "arena")
))]
criterion_main!(benches);
