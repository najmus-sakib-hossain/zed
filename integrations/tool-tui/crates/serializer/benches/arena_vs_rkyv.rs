//! DxArena vs RKYV: Raw Performance Showdown
//!
//! This benchmark compares:
//! - RKYV: Standard zero-copy serialization
//! - DxArena: Raw memory writes with custom format
//!
//! Run with: cargo bench --bench arena_vs_rkyv -p dx-serializer

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use rkyv::{Archive, Deserialize, Serialize};
use serializer::machine::arena_batch::{DxArenaBatch, DxDeserialize, DxSerialize};
use serializer::machine::quantum::{QuantumReader, QuantumWriter};

// RKYV version
#[derive(Archive, Serialize, Deserialize, Clone, Debug, PartialEq)]
#[rkyv(compare(PartialEq), derive(Debug))]
struct PersonRkyv {
    id: u64,
    age: u32,
    salary: u32,
}

impl PersonRkyv {
    fn new(id: u64) -> Self {
        Self {
            id,
            age: (25 + (id % 40)) as u32,
            salary: (50000 + (id * 1000)) as u32,
        }
    }
}

// DxArena version
#[derive(Clone, Debug, PartialEq)]
struct PersonArena {
    id: u64,
    age: u32,
    salary: u32,
}

impl PersonArena {
    fn new(id: u64) -> Self {
        Self {
            id,
            age: (25 + (id % 40)) as u32,
            salary: (50000 + (id * 1000)) as u32,
        }
    }
}

impl DxSerialize for PersonArena {
    fn serialized_size(&self) -> usize {
        16
    }
    fn serialize_into(&self, writer: &mut QuantumWriter<'_>) {
        writer.write_u64::<0>(self.id);
        writer.write_u32::<8>(self.age);
        writer.write_u32::<12>(self.salary);
    }
}

impl DxDeserialize for PersonArena {
    const SIZE: usize = 16;
    fn deserialize_from(reader: &QuantumReader<'_>) -> Self {
        Self {
            id: reader.read_u64::<0>(),
            age: reader.read_u32::<8>(),
            salary: reader.read_u32::<12>(),
        }
    }
}

// RKYV serialization
fn rkyv_serialize(items: &[PersonRkyv]) -> Vec<rkyv::util::AlignedVec> {
    let mut results = Vec::with_capacity(items.len());
    for item in items {
        results.push(rkyv::to_bytes::<rkyv::rancor::Error>(item).unwrap());
    }
    results
}

// DxArena serialization
fn arena_serialize(items: &[PersonArena]) -> Vec<u8> {
    DxArenaBatch::serialize(items)
}

// RKYV deserialization
fn rkyv_deserialize(batches: &[rkyv::util::AlignedVec]) -> Vec<PersonRkyv> {
    batches
        .iter()
        .map(|bytes| {
            let archived = unsafe { rkyv::access_unchecked::<ArchivedPersonRkyv>(bytes) };
            PersonRkyv {
                id: archived.id.into(),
                age: archived.age.into(),
                salary: archived.salary.into(),
            }
        })
        .collect()
}

// DxArena deserialization
fn arena_deserialize(bytes: &[u8]) -> Vec<PersonArena> {
    DxArenaBatch::deserialize::<PersonArena>(bytes)
}

fn bench_serialize_50(c: &mut Criterion) {
    let rkyv_items: Vec<PersonRkyv> = (0..50).map(PersonRkyv::new).collect();
    let arena_items: Vec<PersonArena> = (0..50).map(PersonArena::new).collect();

    let mut group = c.benchmark_group("serialize_50");
    group.throughput(Throughput::Elements(50));

    group.bench_function("rkyv", |b| b.iter(|| rkyv_serialize(black_box(&rkyv_items))));

    group.bench_function("arena", |b| b.iter(|| arena_serialize(black_box(&arena_items))));

    group.finish();
}

fn bench_serialize_100(c: &mut Criterion) {
    let rkyv_items: Vec<PersonRkyv> = (0..100).map(PersonRkyv::new).collect();
    let arena_items: Vec<PersonArena> = (0..100).map(PersonArena::new).collect();

    let mut group = c.benchmark_group("serialize_100");
    group.throughput(Throughput::Elements(100));

    group.bench_function("rkyv", |b| b.iter(|| rkyv_serialize(black_box(&rkyv_items))));

    group.bench_function("arena", |b| b.iter(|| arena_serialize(black_box(&arena_items))));

    group.finish();
}

fn bench_serialize_1k(c: &mut Criterion) {
    let rkyv_items: Vec<PersonRkyv> = (0..1000).map(PersonRkyv::new).collect();
    let arena_items: Vec<PersonArena> = (0..1000).map(PersonArena::new).collect();

    let mut group = c.benchmark_group("serialize_1k");
    group.throughput(Throughput::Elements(1000));

    group.bench_function("rkyv", |b| b.iter(|| rkyv_serialize(black_box(&rkyv_items))));

    group.bench_function("arena", |b| b.iter(|| arena_serialize(black_box(&arena_items))));

    group.finish();
}

fn bench_serialize_10k(c: &mut Criterion) {
    let rkyv_items: Vec<PersonRkyv> = (0..10_000).map(PersonRkyv::new).collect();
    let arena_items: Vec<PersonArena> = (0..10_000).map(PersonArena::new).collect();

    let mut group = c.benchmark_group("serialize_10k");
    group.throughput(Throughput::Elements(10_000));

    group.bench_function("rkyv", |b| b.iter(|| rkyv_serialize(black_box(&rkyv_items))));

    group.bench_function("arena", |b| b.iter(|| arena_serialize(black_box(&arena_items))));

    group.finish();
}

fn bench_roundtrip_1k(c: &mut Criterion) {
    let rkyv_items: Vec<PersonRkyv> = (0..1000).map(PersonRkyv::new).collect();
    let arena_items: Vec<PersonArena> = (0..1000).map(PersonArena::new).collect();

    let mut group = c.benchmark_group("roundtrip_1k");
    group.throughput(Throughput::Elements(1000));

    group.bench_function("rkyv", |b| {
        b.iter(|| {
            let serialized = rkyv_serialize(black_box(&rkyv_items));
            rkyv_deserialize(black_box(&serialized))
        })
    });

    group.bench_function("arena", |b| {
        b.iter(|| {
            let serialized = arena_serialize(black_box(&arena_items));
            arena_deserialize(black_box(&serialized))
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_serialize_50,
    bench_serialize_100,
    bench_serialize_1k,
    bench_serialize_10k,
    bench_roundtrip_1k,
);

criterion_main!(benches);
