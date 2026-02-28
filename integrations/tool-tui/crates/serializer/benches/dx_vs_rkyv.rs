//! DX-Machine vs RKYV: Honest Performance Comparison
//!
//! This benchmark compares:
//! - RKYV naive: Individual serialize calls without pre-allocation
//! - DX-Machine: Adaptive strategy (no pre-alloc for small, pre-alloc for large)
//!
//! Run with: cargo bench --bench dx_vs_rkyv -p dx-serializer

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use rkyv::{Archive, Deserialize, Serialize};
use serializer::machine::serialize_batch;

#[derive(Archive, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[rkyv(compare(PartialEq), derive(Debug))]
struct Person {
    id: u64,
    name: String,
    age: u32,
    salary: u32,
}

impl Person {
    fn new(id: u64) -> Self {
        Self {
            id,
            name: format!("Person{}", id),
            age: (25 + (id % 40)) as u32,
            salary: (50000 + (id * 1000)) as u32,
        }
    }
}

/// RKYV naive: No pre-allocation
fn rkyv_naive(items: &[Person]) -> Vec<rkyv::util::AlignedVec> {
    let mut results = Vec::new();
    for item in items {
        results.push(rkyv::to_bytes::<rkyv::rancor::Error>(item).unwrap());
    }
    results
}

/// DX-Machine: Adaptive strategy
fn dx_machine(items: &[Person]) -> Vec<rkyv::util::AlignedVec> {
    serialize_batch(items).unwrap()
}

fn bench_batch_50(c: &mut Criterion) {
    let items: Vec<Person> = (0..50).map(Person::new).collect();
    let mut group = c.benchmark_group("batch_50");
    group.throughput(Throughput::Elements(50));

    group.bench_function("rkyv_naive", |b| b.iter(|| rkyv_naive(black_box(&items))));

    group.bench_function("dx_machine", |b| b.iter(|| dx_machine(black_box(&items))));

    group.finish();
}

fn bench_batch_100(c: &mut Criterion) {
    let items: Vec<Person> = (0..100).map(Person::new).collect();
    let mut group = c.benchmark_group("batch_100");
    group.throughput(Throughput::Elements(100));

    group.bench_function("rkyv_naive", |b| b.iter(|| rkyv_naive(black_box(&items))));

    group.bench_function("dx_machine", |b| b.iter(|| dx_machine(black_box(&items))));

    group.finish();
}

fn bench_batch_500(c: &mut Criterion) {
    let items: Vec<Person> = (0..500).map(Person::new).collect();
    let mut group = c.benchmark_group("batch_500");
    group.throughput(Throughput::Elements(500));

    group.bench_function("rkyv_naive", |b| b.iter(|| rkyv_naive(black_box(&items))));

    group.bench_function("dx_machine", |b| b.iter(|| dx_machine(black_box(&items))));

    group.finish();
}

fn bench_batch_1k(c: &mut Criterion) {
    let items: Vec<Person> = (0..1000).map(Person::new).collect();
    let mut group = c.benchmark_group("batch_1k");
    group.throughput(Throughput::Elements(1000));

    group.bench_function("rkyv_naive", |b| b.iter(|| rkyv_naive(black_box(&items))));

    group.bench_function("dx_machine", |b| b.iter(|| dx_machine(black_box(&items))));

    group.finish();
}

fn bench_batch_10k(c: &mut Criterion) {
    let items: Vec<Person> = (0..10_000).map(Person::new).collect();
    let mut group = c.benchmark_group("batch_10k");
    group.throughput(Throughput::Elements(10_000));

    group.bench_function("rkyv_naive", |b| b.iter(|| rkyv_naive(black_box(&items))));

    group.bench_function("dx_machine", |b| b.iter(|| dx_machine(black_box(&items))));

    group.finish();
}

criterion_group!(
    benches,
    bench_batch_50,
    bench_batch_100,
    bench_batch_500,
    bench_batch_1k,
    bench_batch_10k
);
criterion_main!(benches);
