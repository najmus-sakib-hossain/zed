//! Performance benchmarks comparing DX to TOON and JSON
//!
//! Run with: cargo bench --bench dx_vs_toon

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use serializer::*;

// Sample data for benchmarks
const SIMPLE_DX: &[u8] = b"name:Alice
age:30
email:alice_example_com
active:+
score:95.5";

const SIMPLE_JSON: &str = r#"{
  "name": "Alice",
  "age": 30,
  "email": "alice@example.com",
  "active": true,
  "score": 95.5
}"#;

// Note: Uses \n explicitly for cross-platform compatibility
const COMPLEX_DX: &[u8] = b"project:DX_Runtime\nversion:0.1.0\nstatus:active\nuser1:Alice\nuser2:Bob\nuser3:Charlie\nloc:50000\nfunctions:2500\ncoverage:95";

const COMPLEX_JSON: &str = r#"{
  "context": {
    "project": "DX Runtime",
    "version": "0.1.0",
    "status": "active"
  },
  "team": ["alice", "bob", "charlie", "diana", "evan"],
  "tasks": [
    {"id": 1, "name": "Parser Implementation", "hours": 12.5, "urgent": true, "assignee": "alice"},
    {"id": 2, "name": "Encoder Optimization", "hours": 8.0, "urgent": true, "assignee": "bob"},
    {"id": 3, "name": "Documentation", "hours": 6.5, "urgent": false, "assignee": "charlie"},
    {"id": 4, "name": "Testing Suite", "hours": 15.0, "urgent": true, "assignee": "diana"},
    {"id": 5, "name": "Benchmarking", "hours": 4.0, "urgent": false, "assignee": "evan"},
    {"id": 6, "name": "Performance Tuning", "hours": 10.0, "urgent": true, "assignee": "alice"},
    {"id": 7, "name": "Security Audit", "hours": 20.0, "urgent": true, "assignee": "bob"}
  ],
  "metrics": [
    {"metric": "LOC", "value": 50000, "unit": "lines"},
    {"metric": "Functions", "value": 2500, "unit": "count"},
    {"metric": "Coverage", "value": 95, "unit": "percent"},
    {"metric": "Performance", "value": 820, "unit": "percent"},
    {"metric": "Compression", "value": 75, "unit": "percent"}
  ]
}"#;

// Large dataset for stress testing
fn generate_large_dataset() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(b"users=id%i name%s email%s age%i active%b score%f\n");

    for i in 1..=1000 {
        let line = format!(
            "{} User{} user{}_example_com {} + {:.2}\n",
            i,
            i,
            i,
            20 + (i % 50),
            50.0 + (i as f64 % 50.0)
        );
        data.extend_from_slice(line.as_bytes());
    }

    data
}

fn benchmark_parse_simple(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_simple");

    group.throughput(Throughput::Bytes(SIMPLE_DX.len() as u64));
    group.bench_function("dx", |b| b.iter(|| parse(black_box(SIMPLE_DX)).unwrap()));

    group.throughput(Throughput::Bytes(SIMPLE_JSON.len() as u64));
    group.bench_function("json", |b| {
        b.iter(|| serde_json::from_str::<serde_json::Value>(black_box(SIMPLE_JSON)).unwrap())
    });

    group.finish();
}

fn benchmark_parse_complex(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_complex");

    group.throughput(Throughput::Bytes(COMPLEX_DX.len() as u64));
    group.bench_function("dx", |b| b.iter(|| parse(black_box(COMPLEX_DX)).unwrap()));

    group.throughput(Throughput::Bytes(COMPLEX_JSON.len() as u64));
    group.bench_function("json", |b| {
        b.iter(|| serde_json::from_str::<serde_json::Value>(black_box(COMPLEX_JSON)).unwrap())
    });

    group.finish();
}

fn benchmark_parse_large(c: &mut Criterion) {
    let large_dx = generate_large_dataset();

    let mut group = c.benchmark_group("parse_large");
    group.sample_size(20);

    group.throughput(Throughput::Bytes(large_dx.len() as u64));
    group.bench_function("dx_1000_rows", |b| b.iter(|| parse(black_box(&large_dx)).unwrap()));

    group.finish();
}

fn benchmark_encode_round_trip(c: &mut Criterion) {
    let parsed = parse(COMPLEX_DX).unwrap();

    let mut group = c.benchmark_group("encode");

    group.bench_function("dx_encode", |b| b.iter(|| encode(black_box(&parsed)).unwrap()));

    group.finish();
}

fn benchmark_human_format(c: &mut Criterion) {
    let parsed = parse(COMPLEX_DX).unwrap();

    let mut group = c.benchmark_group("format");

    group.bench_function("dx_human_format", |b| {
        b.iter(|| format_human(black_box(&parsed)).unwrap())
    });

    group.finish();
}

fn benchmark_compression_ratio(c: &mut Criterion) {
    let group = c.benchmark_group("size_comparison");

    // Measure actual sizes
    println!("\n=== SIZE COMPARISON ===");
    println!("Simple Object:");
    println!("  DX:   {} bytes", SIMPLE_DX.len());
    println!("  JSON: {} bytes", SIMPLE_JSON.len());
    println!(
        "  Compression: {:.1}%",
        (1.0 - SIMPLE_DX.len() as f64 / SIMPLE_JSON.len() as f64) * 100.0
    );

    println!("\nComplex Object:");
    println!("  DX:   {} bytes", COMPLEX_DX.len());
    println!("  JSON: {} bytes", COMPLEX_JSON.len());
    println!(
        "  Compression: {:.1}%",
        (1.0 - COMPLEX_DX.len() as f64 / COMPLEX_JSON.len() as f64) * 100.0
    );

    let large_dx = generate_large_dataset();
    println!("\nLarge Dataset (1000 rows):");
    println!("  DX: {} bytes", large_dx.len());

    group.finish();
}

criterion_group!(benches, benchmark_parse_simple, benchmark_compression_ratio);

criterion_main!(benches);
