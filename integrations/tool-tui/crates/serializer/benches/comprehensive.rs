//! Comprehensive benchmarks for dx-serializer
//!
//! This benchmark suite covers all critical paths:
//! - Machine format parsing (small, medium, large)
//! - LLM format parsing (small, medium, large)
//! - Machine format serialization (small, medium, large)
//! - LLM format serialization (small, medium, large)
//! - Round-trip operations (both formats)
//!
//! Run with: cargo bench -p dx-serializer --bench comprehensive
//!
//! **Validates: Requirements 9.1, 9.2, 9.3**

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use serializer::{
    DxDocument, DxLlmValue, DxValue, deserialize, document_to_llm, encode, llm_to_document, parse,
    serialize,
};

// =============================================================================
// DATA GENERATORS
// =============================================================================

/// Generate machine format input of approximately the target size in bytes.
/// Machine format uses key:value pairs with type hints.
fn generate_machine_input(target_size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(target_size);

    // Header with table schema
    data.extend_from_slice(b"users=id%i name%s email%s score%f active%b\n");

    let mut row_id = 1u64;
    while data.len() < target_size {
        let row = format!(
            "{} User{} user{}@example.com {:.2} +\n",
            row_id,
            row_id,
            row_id,
            50.0 + (row_id as f64 % 50.0)
        );
        data.extend_from_slice(row.as_bytes());
        row_id += 1;
    }

    data
}

/// Generate LLM format input of approximately the target size in bytes.
/// LLM format uses Dx Serializer syntax with key=value pairs.
fn generate_llm_input(target_size: usize) -> String {
    let mut data = String::with_capacity(target_size);

    // Context section
    data.push_str("project=BenchmarkTest\n");
    data.push_str("version=1.0.0\n");
    data.push_str("status=active\n");

    let mut item_id = 1u64;
    while data.len() < target_size {
        let entry = format!(
            "item{}=value{}\ncount{}={}\nactive{}=+\n",
            item_id,
            item_id,
            item_id,
            item_id * 10,
            item_id
        );
        data.push_str(&entry);
        item_id += 1;
    }

    data
}

/// Generate a DxDocument with approximately the target number of entries.
fn generate_dx_document(num_entries: usize) -> DxDocument {
    let mut doc = DxDocument::new();

    // Add context entries
    doc.context
        .insert("project".to_string(), DxLlmValue::Str("BenchmarkTest".to_string()));
    doc.context.insert("version".to_string(), DxLlmValue::Str("1.0.0".to_string()));
    doc.context.insert("status".to_string(), DxLlmValue::Str("active".to_string()));

    // Add many entries to reach target size
    for i in 0..num_entries {
        doc.context.insert(format!("item{}", i), DxLlmValue::Str(format!("value{}", i)));
        doc.context.insert(format!("count{}", i), DxLlmValue::Num(i as f64 * 10.0));
        doc.context.insert(format!("active{}", i), DxLlmValue::Bool(true));
    }

    doc
}

/// Generate a DxValue with approximately the target number of entries.
fn generate_dx_value(num_entries: usize) -> DxValue {
    use serializer::DxObject;

    let mut obj = DxObject::with_capacity(num_entries * 3 + 3);

    // Add header entries
    obj.insert("project".to_string(), DxValue::String("BenchmarkTest".to_string()));
    obj.insert("version".to_string(), DxValue::String("1.0.0".to_string()));
    obj.insert("status".to_string(), DxValue::String("active".to_string()));

    // Add many entries
    for i in 0..num_entries {
        obj.insert(format!("item{}", i), DxValue::String(format!("value{}", i)));
        obj.insert(format!("count{}", i), DxValue::Int((i * 10) as i64));
        obj.insert(format!("active{}", i), DxValue::Bool(true));
    }

    DxValue::Object(obj)
}

// =============================================================================
// SIZE CONSTANTS
// =============================================================================

/// Small input size (~100 bytes)
const SMALL_SIZE: usize = 100;
/// Medium input size (~10 KB)
const MEDIUM_SIZE: usize = 10 * 1024;
/// Large input size (~1 MB)
const LARGE_SIZE: usize = 1024 * 1024;

/// Number of entries for small document
const SMALL_ENTRIES: usize = 3;
/// Number of entries for medium document
const MEDIUM_ENTRIES: usize = 300;
/// Number of entries for large document
const LARGE_ENTRIES: usize = 30_000;

// =============================================================================
// MACHINE FORMAT PARSING BENCHMARKS
// =============================================================================

fn bench_parse_machine(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_machine");

    // Small input (~100 bytes)
    let small = generate_machine_input(SMALL_SIZE);
    group.throughput(Throughput::Bytes(small.len() as u64));
    group.bench_with_input(BenchmarkId::new("small", small.len()), &small, |b, input| {
        b.iter(|| parse(black_box(input)))
    });

    // Medium input (~10 KB)
    let medium = generate_machine_input(MEDIUM_SIZE);
    group.throughput(Throughput::Bytes(medium.len() as u64));
    group.bench_with_input(BenchmarkId::new("medium", medium.len()), &medium, |b, input| {
        b.iter(|| parse(black_box(input)))
    });

    // Large input (~1 MB)
    let large = generate_machine_input(LARGE_SIZE);
    group.throughput(Throughput::Bytes(large.len() as u64));
    group.sample_size(20); // Reduce sample size for large inputs
    group.bench_with_input(BenchmarkId::new("large", large.len()), &large, |b, input| {
        b.iter(|| parse(black_box(input)))
    });

    group.finish();
}

// =============================================================================
// LLM FORMAT PARSING BENCHMARKS
// =============================================================================

fn bench_parse_llm(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_llm");

    // Small input (~100 bytes)
    let small = generate_llm_input(SMALL_SIZE);
    group.throughput(Throughput::Bytes(small.len() as u64));
    group.bench_with_input(BenchmarkId::new("small", small.len()), &small, |b, input| {
        b.iter(|| llm_to_document(black_box(input)))
    });

    // Medium input (~10 KB)
    let medium = generate_llm_input(MEDIUM_SIZE);
    group.throughput(Throughput::Bytes(medium.len() as u64));
    group.bench_with_input(BenchmarkId::new("medium", medium.len()), &medium, |b, input| {
        b.iter(|| llm_to_document(black_box(input)))
    });

    // Large input (~1 MB)
    let large = generate_llm_input(LARGE_SIZE);
    group.throughput(Throughput::Bytes(large.len() as u64));
    group.sample_size(20); // Reduce sample size for large inputs
    group.bench_with_input(BenchmarkId::new("large", large.len()), &large, |b, input| {
        b.iter(|| llm_to_document(black_box(input)))
    });

    group.finish();
}

// =============================================================================
// MACHINE FORMAT SERIALIZATION BENCHMARKS
// =============================================================================

fn bench_serialize_machine(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize_machine");

    // Small document
    let small = generate_dx_value(SMALL_ENTRIES);
    group.bench_with_input(BenchmarkId::new("small", SMALL_ENTRIES), &small, |b, value| {
        b.iter(|| encode(black_box(value)))
    });

    // Medium document
    let medium = generate_dx_value(MEDIUM_ENTRIES);
    group.bench_with_input(BenchmarkId::new("medium", MEDIUM_ENTRIES), &medium, |b, value| {
        b.iter(|| encode(black_box(value)))
    });

    // Large document
    let large = generate_dx_value(LARGE_ENTRIES);
    group.sample_size(20); // Reduce sample size for large inputs
    group.bench_with_input(BenchmarkId::new("large", LARGE_ENTRIES), &large, |b, value| {
        b.iter(|| encode(black_box(value)))
    });

    group.finish();
}

// =============================================================================
// LLM FORMAT SERIALIZATION BENCHMARKS
// =============================================================================

fn bench_serialize_llm(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize_llm");

    // Small document
    let small = generate_dx_document(SMALL_ENTRIES);
    group.bench_with_input(BenchmarkId::new("small", SMALL_ENTRIES), &small, |b, doc| {
        b.iter(|| document_to_llm(black_box(doc)))
    });

    // Medium document
    let medium = generate_dx_document(MEDIUM_ENTRIES);
    group.bench_with_input(BenchmarkId::new("medium", MEDIUM_ENTRIES), &medium, |b, doc| {
        b.iter(|| document_to_llm(black_box(doc)))
    });

    // Large document
    let large = generate_dx_document(LARGE_ENTRIES);
    group.sample_size(20); // Reduce sample size for large inputs
    group.bench_with_input(BenchmarkId::new("large", LARGE_ENTRIES), &large, |b, doc| {
        b.iter(|| document_to_llm(black_box(doc)))
    });

    group.finish();
}

// =============================================================================
// ROUND-TRIP BENCHMARKS
// =============================================================================

fn bench_roundtrip_machine(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip_machine");

    // Small
    let small = generate_dx_value(SMALL_ENTRIES);
    group.bench_with_input(BenchmarkId::new("small", SMALL_ENTRIES), &small, |b, value| {
        b.iter(|| {
            let encoded = encode(black_box(value)).expect("encode failed");
            parse(black_box(&encoded))
        })
    });

    // Medium
    let medium = generate_dx_value(MEDIUM_ENTRIES);
    group.bench_with_input(BenchmarkId::new("medium", MEDIUM_ENTRIES), &medium, |b, value| {
        b.iter(|| {
            let encoded = encode(black_box(value)).expect("encode failed");
            parse(black_box(&encoded))
        })
    });

    // Large
    let large = generate_dx_value(LARGE_ENTRIES);
    group.sample_size(20);
    group.bench_with_input(BenchmarkId::new("large", LARGE_ENTRIES), &large, |b, value| {
        b.iter(|| {
            let encoded = encode(black_box(value)).expect("encode failed");
            parse(black_box(&encoded))
        })
    });

    group.finish();
}

fn bench_roundtrip_llm(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip_llm");

    // Small
    let small = generate_dx_document(SMALL_ENTRIES);
    group.bench_with_input(BenchmarkId::new("small", SMALL_ENTRIES), &small, |b, doc| {
        b.iter(|| {
            let serialized = serialize(black_box(doc));
            deserialize(black_box(&serialized))
        })
    });

    // Medium
    let medium = generate_dx_document(MEDIUM_ENTRIES);
    group.bench_with_input(BenchmarkId::new("medium", MEDIUM_ENTRIES), &medium, |b, doc| {
        b.iter(|| {
            let serialized = serialize(black_box(doc));
            deserialize(black_box(&serialized))
        })
    });

    // Large
    let large = generate_dx_document(LARGE_ENTRIES);
    group.sample_size(20);
    group.bench_with_input(BenchmarkId::new("large", LARGE_ENTRIES), &large, |b, doc| {
        b.iter(|| {
            let serialized = serialize(black_box(doc));
            deserialize(black_box(&serialized))
        })
    });

    group.finish();
}

// =============================================================================
// BENCHMARK GROUPS
// =============================================================================

criterion_group!(
    benches,
    bench_parse_machine,
    bench_parse_llm,
    bench_serialize_machine,
    bench_serialize_llm,
    bench_roundtrip_machine,
    bench_roundtrip_llm,
);

criterion_main!(benches);
