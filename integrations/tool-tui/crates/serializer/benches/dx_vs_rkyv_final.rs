use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rkyv::{Archive, Deserialize, Serialize};
use serializer::machine::{serialize, serialize_batch};

#[derive(Archive, Serialize, Deserialize, Clone, Debug, PartialEq)]
struct TestData {
    id: u64,
    name: String,
    value: f64,
}

impl TestData {
    fn new(id: u64) -> Self {
        Self {
            id,
            name: format!("item_{}", id),
            value: id as f64 * 1.5,
        }
    }
}

fn bench_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("dx_vs_rkyv");

    // Single item
    let data = TestData::new(42);

    group.bench_function("rkyv_single", |b| {
        b.iter(|| {
            let _bytes = rkyv::to_bytes::<rkyv::rancor::Error>(black_box(&data)).unwrap();
        });
    });

    group.bench_function("dx_single", |b| {
        b.iter(|| {
            let _bytes = serialize(black_box(&data)).unwrap();
        });
    });

    // Batch 100
    let items: Vec<TestData> = (0..100).map(TestData::new).collect();

    group.bench_function("rkyv_batch_100", |b| {
        b.iter(|| {
            let mut results = Vec::new();
            for item in black_box(&items) {
                results.push(rkyv::to_bytes::<rkyv::rancor::Error>(item).unwrap());
            }
        });
    });

    group.bench_function("dx_batch_100", |b| {
        b.iter(|| {
            serialize_batch(black_box(&items)).unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, bench_comparison);
criterion_main!(benches);
