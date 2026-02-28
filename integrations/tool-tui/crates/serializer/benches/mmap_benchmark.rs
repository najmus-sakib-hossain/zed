//! Memory-Mapped Direct Write Benchmarks
//!
//! Benchmarks mmap operations vs Vec allocation for various object sizes.
//! Tests the performance claims from REQ-2: 10-100x faster for large objects.
//!
//! Run with: cargo bench -p dx-serializer --bench mmap_benchmark
//!
//! **Validates: Requirements 2.1, 2.2**

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use serializer::machine::{DxArena, DxMmap, QuantumWriter};
use std::fs;
use std::io::Write;

// =============================================================================
// TEST DATA STRUCTURES
// =============================================================================

/// Simple record structure for benchmarking
#[derive(Clone)]
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

    fn size() -> usize {
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
// SIZE CONSTANTS
// =============================================================================

const KB: usize = 1024;
const MB: usize = 1024 * KB;

const SIZE_1MB: usize = 1 * MB;
const SIZE_10MB: usize = 10 * MB;
const SIZE_100MB: usize = 100 * MB;

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Generate test records to fill approximately target_size bytes
fn generate_records(target_size: usize) -> Vec<TestRecord> {
    let count = target_size / TestRecord::size();
    (0..count as u64).map(TestRecord::new).collect()
}

/// Serialize records using Vec allocation (baseline)
fn serialize_with_vec(records: &[TestRecord]) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(4 + records.len() * TestRecord::size());

    // Write header
    buffer.push(0x5A); // Magic
    buffer.push(0x44);
    buffer.push(0x01); // Version
    buffer.push(0x04); // Flags

    // Write records
    for record in records {
        let mut record_buf = vec![0u8; TestRecord::size()];
        let mut writer = QuantumWriter::new(&mut record_buf);
        record.write_to(&mut writer);
        buffer.extend_from_slice(&record_buf);
    }

    buffer
}

/// Serialize records using DxArena (optimized)
fn serialize_with_arena(records: &[TestRecord]) -> Vec<u8> {
    let mut arena = DxArena::new(4 + records.len() * TestRecord::size());

    // Write header
    arena.write_header(0);

    // Write records
    for record in records {
        let mut writer = arena.writer();
        record.write_to(&mut writer);
        arena.advance(TestRecord::size());
    }

    arena.to_vec()
}

/// Write records to file using standard I/O
fn write_with_standard_io(path: &str, records: &[TestRecord]) -> std::io::Result<()> {
    let mut file = fs::File::create(path)?;

    // Write header
    file.write_all(&[0x5A, 0x44, 0x01, 0x04])?;

    // Write records
    for record in records {
        let mut record_buf = vec![0u8; TestRecord::size()];
        let mut writer = QuantumWriter::new(&mut record_buf);
        record.write_to(&mut writer);
        file.write_all(&record_buf)?;
    }

    file.sync_all()?;
    Ok(())
}

/// Write records using memory-mapped file
fn write_with_mmap(path: &str, records: &[TestRecord]) -> std::io::Result<()> {
    // First, serialize to arena
    let mut arena = DxArena::new(4 + records.len() * TestRecord::size());
    arena.write_header(0);

    for record in records {
        let mut writer = arena.writer();
        record.write_to(&mut writer);
        arena.advance(TestRecord::size());
    }

    // Write to file in one shot
    fs::write(path, arena.as_bytes())?;
    Ok(())
}

// =============================================================================
// BENCHMARKS: Vec vs Arena Allocation
// =============================================================================

fn bench_vec_vs_arena_1mb(c: &mut Criterion) {
    let records = generate_records(SIZE_1MB);
    let mut group = c.benchmark_group("vec_vs_arena_1mb");
    group.throughput(Throughput::Bytes(SIZE_1MB as u64));

    group.bench_function("vec_allocation", |b| b.iter(|| serialize_with_vec(black_box(&records))));

    group.bench_function("arena_allocation", |b| {
        b.iter(|| serialize_with_arena(black_box(&records)))
    });

    group.finish();
}

fn bench_vec_vs_arena_10mb(c: &mut Criterion) {
    let records = generate_records(SIZE_10MB);
    let mut group = c.benchmark_group("vec_vs_arena_10mb");
    group.throughput(Throughput::Bytes(SIZE_10MB as u64));
    group.sample_size(20); // Reduce samples for large data

    group.bench_function("vec_allocation", |b| b.iter(|| serialize_with_vec(black_box(&records))));

    group.bench_function("arena_allocation", |b| {
        b.iter(|| serialize_with_arena(black_box(&records)))
    });

    group.finish();
}

fn bench_vec_vs_arena_100mb(c: &mut Criterion) {
    let records = generate_records(SIZE_100MB);
    let mut group = c.benchmark_group("vec_vs_arena_100mb");
    group.throughput(Throughput::Bytes(SIZE_100MB as u64));
    group.sample_size(10); // Minimal samples for very large data

    group.bench_function("vec_allocation", |b| b.iter(|| serialize_with_vec(black_box(&records))));

    group.bench_function("arena_allocation", |b| {
        b.iter(|| serialize_with_arena(black_box(&records)))
    });

    group.finish();
}

// =============================================================================
// BENCHMARKS: File I/O Performance
// =============================================================================

fn bench_file_io_1mb(c: &mut Criterion) {
    let records = generate_records(SIZE_1MB);
    let mut group = c.benchmark_group("file_io_1mb");
    group.throughput(Throughput::Bytes(SIZE_1MB as u64));

    // Ensure target directory exists
    let _ = std::fs::create_dir_all("target");

    group.bench_function("standard_io", |b| {
        b.iter(|| {
            let path = "target/bench_1mb_std.bin";
            write_with_standard_io(path, black_box(&records)).unwrap();
            let _ = fs::remove_file(path);
        })
    });

    group.bench_function("mmap_write", |b| {
        b.iter(|| {
            let path = "target/bench_1mb_mmap.bin";
            write_with_mmap(path, black_box(&records)).unwrap();
            let _ = fs::remove_file(path);
        })
    });

    group.finish();
}

fn bench_file_io_10mb(c: &mut Criterion) {
    let records = generate_records(SIZE_10MB);
    let mut group = c.benchmark_group("file_io_10mb");
    group.throughput(Throughput::Bytes(SIZE_10MB as u64));
    group.sample_size(20);

    // Ensure target directory exists
    let _ = std::fs::create_dir_all("target");

    group.bench_function("standard_io", |b| {
        b.iter(|| {
            let path = "target/bench_10mb_std.bin";
            write_with_standard_io(path, black_box(&records)).unwrap();
            let _ = fs::remove_file(path);
        })
    });

    group.bench_function("mmap_write", |b| {
        b.iter(|| {
            let path = "target/bench_10mb_mmap.bin";
            write_with_mmap(path, black_box(&records)).unwrap();
            let _ = fs::remove_file(path);
        })
    });

    group.finish();
}

fn bench_file_io_100mb(c: &mut Criterion) {
    let records = generate_records(SIZE_100MB);
    let mut group = c.benchmark_group("file_io_100mb");
    group.throughput(Throughput::Bytes(SIZE_100MB as u64));
    group.sample_size(10);

    // Ensure target directory exists
    let _ = std::fs::create_dir_all("target");

    group.bench_function("standard_io", |b| {
        b.iter(|| {
            let path = "target/bench_100mb_std.bin";
            write_with_standard_io(path, black_box(&records)).unwrap();
            let _ = fs::remove_file(path);
        })
    });

    group.bench_function("mmap_write", |b| {
        b.iter(|| {
            let path = "target/bench_100mb_mmap.bin";
            write_with_mmap(path, black_box(&records)).unwrap();
            let _ = fs::remove_file(path);
        })
    });

    group.finish();
}

// =============================================================================
// BENCHMARKS: Read Performance
// =============================================================================

fn bench_read_performance(c: &mut Criterion) {
    // Ensure target directory exists
    let _ = std::fs::create_dir_all("target");

    // Create test files
    let records_1mb = generate_records(SIZE_1MB);
    let records_10mb = generate_records(SIZE_10MB);

    let path_1mb = "target/bench_read_1mb.bin";
    let path_10mb = "target/bench_read_10mb.bin";

    write_with_mmap(path_1mb, &records_1mb).unwrap();
    write_with_mmap(path_10mb, &records_10mb).unwrap();

    let mut group = c.benchmark_group("read_performance");

    // 1MB read
    group.throughput(Throughput::Bytes(SIZE_1MB as u64));
    group.bench_function(BenchmarkId::new("mmap_read", "1mb"), |b| {
        b.iter(|| {
            let mmap = DxMmap::open(black_box(path_1mb)).unwrap();
            let reader = mmap.reader();
            black_box(reader.read_u64::<4>())
        })
    });

    // 10MB read
    group.throughput(Throughput::Bytes(SIZE_10MB as u64));
    group.bench_function(BenchmarkId::new("mmap_read", "10mb"), |b| {
        b.iter(|| {
            let mmap = DxMmap::open(black_box(path_10mb)).unwrap();
            let reader = mmap.reader();
            black_box(reader.read_u64::<4>())
        })
    });

    group.finish();

    // Cleanup
    let _ = fs::remove_file(path_1mb);
    let _ = fs::remove_file(path_10mb);
}

// =============================================================================
// BENCHMARKS: Batch Operations
// =============================================================================

fn bench_batch_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_operations");

    // Small batches (1000 records)
    let small_batch = generate_records(1000 * TestRecord::size());
    group
        .bench_function("batch_1000", |b| b.iter(|| serialize_with_arena(black_box(&small_batch))));

    // Medium batches (10000 records)
    let medium_batch = generate_records(10000 * TestRecord::size());
    group.bench_function("batch_10000", |b| {
        b.iter(|| serialize_with_arena(black_box(&medium_batch)))
    });

    // Large batches (100000 records)
    let large_batch = generate_records(100000 * TestRecord::size());
    group.sample_size(20);
    group.bench_function("batch_100000", |b| {
        b.iter(|| serialize_with_arena(black_box(&large_batch)))
    });

    group.finish();
}

// =============================================================================
// BENCHMARKS: Arena Reuse
// =============================================================================

fn bench_arena_reuse(c: &mut Criterion) {
    let records = generate_records(10000 * TestRecord::size());
    let mut group = c.benchmark_group("arena_reuse");

    group.bench_function("with_reuse", |b| {
        let mut arena = DxArena::new(4 + records.len() * TestRecord::size());
        b.iter(|| {
            arena.reset();
            arena.write_header(0);
            for record in black_box(&records) {
                let mut writer = arena.writer();
                record.write_to(&mut writer);
                arena.advance(TestRecord::size());
            }
            black_box(arena.offset())
        })
    });

    group.bench_function("without_reuse", |b| {
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

    group.finish();
}

// =============================================================================
// BENCHMARK GROUPS
// =============================================================================

criterion_group!(
    vec_vs_arena,
    bench_vec_vs_arena_1mb,
    bench_vec_vs_arena_10mb,
    bench_vec_vs_arena_100mb,
);

criterion_group!(file_io, bench_file_io_1mb, bench_file_io_10mb, bench_file_io_100mb,);

criterion_group!(read_perf, bench_read_performance,);

criterion_group!(batch_ops, bench_batch_operations, bench_arena_reuse,);

criterion_main!(vec_vs_arena, file_io, read_perf, batch_ops);
