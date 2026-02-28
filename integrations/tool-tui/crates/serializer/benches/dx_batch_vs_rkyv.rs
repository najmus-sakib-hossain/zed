use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use rkyv::{Archive, Deserialize, Serialize};
use serializer::machine::{serialize, serialize_batch};

#[derive(Archive, Serialize, Deserialize, Clone, Debug, PartialEq)]
struct TestData {
    id: u64,
    name: String,
    value: f64,
    active: bool,
}

impl TestData {
    fn new(id: u64) -> Self {
        Self {
            id,
            name: format!("item_{}", id),
            value: id as f64 * 1.5,
            active: id % 2 == 0,
        }
    }
}

fn bench_single_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_serialize");
    let data = TestData::new(42);

    group.bench_function("rkyv_native", |b| {
        b.iter(|| {
            let _bytes = rkyv::to_bytes::<rkyv::rancor::Error>(black_box(&data)).unwrap();
        });
    });

    group.bench_function("dx_machine", |b| {
        b.iter(|| {
            let _bytes = serialize(black_box(&data)).unwrap();
        });
    });

    group.finish();
}

fn bench_batch_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_serialize");

    for size in [10, 100, 1000, 10000].iter() {
        let items: Vec<TestData> = (0..*size).map(TestData::new).collect();

        group.bench_with_input(BenchmarkId::new("rkyv_naive", size), size, |b, _| {
            b.iter(|| {
                let mut results = Vec::new();
                for item in black_box(&items) {
                    results.push(rkyv::to_bytes::<rkyv::rancor::Error>(item).unwrap());
                }
            });
        });

        group.bench_with_input(BenchmarkId::new("rkyv_prealloc", size), size, |b, _| {
            b.iter(|| {
                let mut results = Vec::with_capacity(items.len());
                for item in black_box(&items) {
                    results.push(rkyv::to_bytes::<rkyv::rancor::Error>(item).unwrap());
                }
            });
        });

        group.bench_with_input(BenchmarkId::new("dx_batch", size), size, |b, _| {
            b.iter(|| {
                serialize_batch(black_box(&items)).unwrap();
            });
        });
    }

    group.finish();
}

fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");
    let data = TestData::new(42);

    group.bench_function("rkyv_native", |b| {
        b.iter(|| {
            let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(black_box(&data)).unwrap();
            let archived = unsafe { rkyv::access_unchecked::<ArchivedTestData>(&bytes) };
            black_box(archived);
        });
    });

    group.bench_function("dx_machine", |b| {
        b.iter(|| {
            let bytes = serialize(black_box(&data)).unwrap();
            let archived = unsafe { serializer::machine::deserialize::<TestData>(&bytes) };
            black_box(archived);
        });
    });

    group.finish();
}

fn bench_batch_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_roundtrip");
    let items: Vec<TestData> = (0..1000).map(TestData::new).collect();

    group.bench_function("rkyv_naive", |b| {
        b.iter(|| {
            let mut serialized = Vec::new();
            for item in black_box(&items) {
                serialized.push(rkyv::to_bytes::<rkyv::rancor::Error>(item).unwrap());
            }

            let mut deserialized = Vec::new();
            for bytes in &serialized {
                let archived = unsafe { rkyv::access_unchecked::<ArchivedTestData>(bytes) };
                deserialized.push(archived);
            }
            black_box(deserialized);
        });
    });

    group.bench_function("dx_batch", |b| {
        b.iter(|| {
            let serialized = serialize_batch(black_box(&items)).unwrap();
            let deserialized =
                unsafe { serializer::machine::deserialize_batch::<TestData>(&serialized) };
            black_box(deserialized);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_single_serialize,
    bench_batch_serialize,
    bench_roundtrip,
    bench_batch_roundtrip,
);
criterion_main!(benches);
