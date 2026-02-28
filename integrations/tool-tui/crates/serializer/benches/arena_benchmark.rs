//! Arena Allocator Benchmarks
//!
//! Comprehensive benchmarks for DxArena vs individual allocations.
//! Tests the performance claims from REQ-3: 5-10x faster for batches.
//!
//! Run with: cargo bench -p dx-serializer --bench arena_benchmark
//!
//! **Validates: Requirements 3.1, 3.2**

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use serializer::machine::{DxArena, QuantumWriter};

// =============================================================================
// TEST DATA STRUCTURES
// =============================================================================

/// Simple record structure for benchmarking
#[derive(Clone, Copy)]
struct TestRecord {
    id: u64,
    value: u64,
    timestamp: u64,
    flags: u32,
    padding: u32,
}

impl TestRecord {
    fn new(id: u64) -> Self {
        Self {
            id,
            value: id * 100,
            timestamp: 1234567890 + id,
            flags: (id % 256) as u32,
            padding: 0,
        }
    }

    const fn size() -> usize {
        32 // 8 + 8 + 8 + 4 + 4
    }

    fn write_to(&self, writer: &mut QuantumWriter) {
        writer.write_u64::<0>(self.id);
        writer.write_u64::<8>(self.value);
        writer.write_u64::<16>(self.timestamp);
        writer.write_u32::<24>(self.flags);
        writer.write_u32::<28>(self.padding);
    }
}

// =============================================================================
// BATCH SIZE CONSTANTS
// =============================================================================

const BATCH_100: usize = 100;
const BATCH_1K: usize = 1_000;
const BATCH_10K: usize = 10_000;
const BATCH_100K: usize = 100_000;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Generate test records
fn generate_records(count: usize) -> Vec<TestRecord> {
    (0..count as u64).map(TestRecord::new).collect()
}

/// Serialize using individual Vec allocations (baseline)
fn serialize_individual_allocations(records: &[TestRecord]) -> Vec<Vec<u8>> {
    let mut results = Vec::with_capacity(records.len());

    for record in records {
        let mut buffer = vec![0u8; TestRecord::size()];
        let mut writer = QuantumWriter::new(&mut buffer);
        record.write_to(&mut writer);
        results.push(buffer);
    }

    results
}

/// Serialize using DxArena (optimized)
fn serialize_with_arena(records: &[TestRecord]) -> Vec<u8> {
    let mut arena = DxArena::new(4 + records.len() * TestRecord::size());
    arena.write_header(0);

    for record in records {
        let mut writer = arena.writer();
        record.write_to(&mut writer);
        arena.advance(TestRecord::size());
    }

    arena.to_vec()
}

/// Serialize using DxArena with reuse
fn serialize_with_arena_reuse(arena: &mut DxArena, records: &[TestRecord]) {
    arena.reset();
    arena.write_header(0);

    for record in records {
        let mut writer = arena.writer();
        record.write_to(&mut writer);
        arena.advance(TestRecord::size());
    }
}

// =============================================================================
// BENCHMARKS: Individual Allocations vs Arena
// =============================================================================

fn bench_individual_vs_arena_100(c: &mut Criterion) {
    let records = generate_records(BATCH_100);
    let mut group = c.benchmark_group("individual_vs_arena_100");
    group.throughput(Throughput::Elements(BATCH_100 as u64));

    group.bench_function("individual_allocations", |b| {
        b.iter(|| serialize_individual_allocations(black_box(&records)))
    });

    group.bench_function("arena_allocation", |b| {
        b.iter(|| serialize_with_arena(black_box(&records)))
    });

    group.finish();
}

fn bench_individual_vs_arena_1k(c: &mut Criterion) {
    let records = generate_records(BATCH_1K);
    let mut group = c.benchmark_group("individual_vs_arena_1k");
    group.throughput(Throughput::Elements(BATCH_1K as u64));

    group.bench_function("individual_allocations", |b| {
        b.iter(|| serialize_individual_allocations(black_box(&records)))
    });

    group.bench_function("arena_allocation", |b| {
        b.iter(|| serialize_with_arena(black_box(&records)))
    });

    group.finish();
}

fn bench_individual_vs_arena_10k(c: &mut Criterion) {
    let records = generate_records(BATCH_10K);
    let mut group = c.benchmark_group("individual_vs_arena_10k");
    group.throughput(Throughput::Elements(BATCH_10K as u64));

    group.bench_function("individual_allocations", |b| {
        b.iter(|| serialize_individual_allocations(black_box(&records)))
    });

    group.bench_function("arena_allocation", |b| {
        b.iter(|| serialize_with_arena(black_box(&records)))
    });

    group.finish();
}

fn bench_individual_vs_arena_100k(c: &mut Criterion) {
    let records = generate_records(BATCH_100K);
    let mut group = c.benchmark_group("individual_vs_arena_100k");
    group.throughput(Throughput::Elements(BATCH_100K as u64));
    group.sample_size(20); // Reduce samples for large data

    group.bench_function("individual_allocations", |b| {
        b.iter(|| serialize_individual_allocations(black_box(&records)))
    });

    group.bench_function("arena_allocation", |b| {
        b.iter(|| serialize_with_arena(black_box(&records)))
    });

    group.finish();
}

// =============================================================================
// BENCHMARKS: Batch Sizes
// =============================================================================

fn bench_batch_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_sizes");

    // 100 records
    let batch_100 = generate_records(BATCH_100);
    group.throughput(Throughput::Elements(BATCH_100 as u64));
    group.bench_with_input(BenchmarkId::new("arena", BATCH_100), &batch_100, |b, records| {
        b.iter(|| serialize_with_arena(black_box(records)))
    });

    // 1K records
    let batch_1k = generate_records(BATCH_1K);
    group.throughput(Throughput::Elements(BATCH_1K as u64));
    group.bench_with_input(BenchmarkId::new("arena", BATCH_1K), &batch_1k, |b, records| {
        b.iter(|| serialize_with_arena(black_box(records)))
    });

    // 10K records
    let batch_10k = generate_records(BATCH_10K);
    group.throughput(Throughput::Elements(BATCH_10K as u64));
    group.bench_with_input(BenchmarkId::new("arena", BATCH_10K), &batch_10k, |b, records| {
        b.iter(|| serialize_with_arena(black_box(records)))
    });

    // 100K records
    let batch_100k = generate_records(BATCH_100K);
    group.throughput(Throughput::Elements(BATCH_100K as u64));
    group.sample_size(20);
    group.bench_with_input(BenchmarkId::new("arena", BATCH_100K), &batch_100k, |b, records| {
        b.iter(|| serialize_with_arena(black_box(records)))
    });

    group.finish();
}

// =============================================================================
// BENCHMARKS: Arena Reuse
// =============================================================================

fn bench_arena_reuse_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("arena_reuse");

    // Test with 1K records
    let records = generate_records(BATCH_1K);
    group.throughput(Throughput::Elements(BATCH_1K as u64));

    group.bench_function("with_reuse", |b| {
        let mut arena = DxArena::new(4 + records.len() * TestRecord::size());
        b.iter(|| {
            serialize_with_arena_reuse(black_box(&mut arena), black_box(&records));
            black_box(arena.offset())
        })
    });

    group.bench_function("without_reuse", |b| {
        b.iter(|| {
            let mut arena = DxArena::new(4 + records.len() * TestRecord::size());
            serialize_with_arena_reuse(black_box(&mut arena), black_box(&records));
            black_box(arena.offset())
        })
    });

    group.finish();
}

// =============================================================================
// BENCHMARKS: Arena Pool
// =============================================================================

fn bench_arena_pool(c: &mut Criterion) {
    use serializer::machine::DxArenaPool;

    let records = generate_records(BATCH_1K);
    let mut group = c.benchmark_group("arena_pool");
    group.throughput(Throughput::Elements(BATCH_1K as u64));

    group.bench_function("pool_acquire_release", |b| {
        let mut pool = DxArenaPool::with_count(4 + records.len() * TestRecord::size(), 4);
        b.iter(|| {
            let mut arena = pool.acquire();
            serialize_with_arena_reuse(black_box(&mut arena), black_box(&records));
            pool.release(arena);
        })
    });

    group.bench_function("new_arena_each_time", |b| {
        b.iter(|| {
            let mut arena = DxArena::new(4 + records.len() * TestRecord::size());
            serialize_with_arena_reuse(black_box(&mut arena), black_box(&records));
        })
    });

    group.finish();
}

// =============================================================================
// BENCHMARKS: Memory Overhead
// =============================================================================

fn bench_memory_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_overhead");

    // Measure allocation overhead for different batch sizes
    for &size in &[BATCH_100, BATCH_1K, BATCH_10K] {
        let records = generate_records(size);

        group.bench_with_input(
            BenchmarkId::new("individual_allocs", size),
            &records,
            |b, records| {
                b.iter(|| {
                    let results = serialize_individual_allocations(black_box(records));
                    black_box(results.len())
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("arena_single_alloc", size),
            &records,
            |b, records| {
                b.iter(|| {
                    let result = serialize_with_arena(black_box(records));
                    black_box(result.len())
                })
            },
        );
    }

    group.finish();
}

// =============================================================================
// BENCHMARKS: Throughput Scaling
// =============================================================================

fn bench_throughput_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_scaling");

    // Test how throughput scales with batch size
    for &size in &[100, 500, 1_000, 5_000, 10_000, 50_000, 100_000] {
        let records = generate_records(size);
        let bytes = (4 + size * TestRecord::size()) as u64;

        group.throughput(Throughput::Bytes(bytes));
        group.bench_with_input(BenchmarkId::new("arena", size), &records, |b, records| {
            b.iter(|| serialize_with_arena(black_box(records)))
        });
    }

    group.finish();
}

// =============================================================================
// BENCHMARKS: Write Batch API
// =============================================================================

fn bench_write_batch_api(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_batch_api");

    let records = generate_records(BATCH_10K);
    group.throughput(Throughput::Elements(BATCH_10K as u64));

    group.bench_function("manual_loop", |b| {
        b.iter(|| {
            let mut arena = DxArena::new(4 + records.len() * TestRecord::size());
            arena.write_header(0);
            for record in black_box(&records) {
                let mut writer = arena.writer();
                record.write_to(&mut writer);
                arena.advance(TestRecord::size());
            }
            black_box(arena.offset())
        })
    });

    group.bench_function("write_batch", |b| {
        b.iter(|| {
            let mut arena = DxArena::new(4 + records.len() * TestRecord::size());
            arena.write_header(0);
            arena.write_batch(TestRecord::size(), records.len(), |writer, i| {
                records[i].write_to(writer);
            });
            black_box(arena.offset())
        })
    });

    group.finish();
}

// =============================================================================
// BENCHMARK GROUPS
// =============================================================================

criterion_group!(
    individual_vs_arena,
    bench_individual_vs_arena_100,
    bench_individual_vs_arena_1k,
    bench_individual_vs_arena_10k,
    bench_individual_vs_arena_100k,
);

criterion_group!(batch_sizes, bench_batch_sizes, bench_throughput_scaling,);

criterion_group!(
    arena_features,
    bench_arena_reuse_comparison,
    bench_arena_pool,
    bench_write_batch_api,
);

criterion_group!(memory, bench_memory_overhead,);

criterion_main!(individual_vs_arena, batch_sizes, arena_features, memory);
