use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rkyv::{Archive, Deserialize, Serialize};

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

fn bench_plain_rkyv(c: &mut Criterion) {
    let data = create_test_data(1000);

    c.bench_function("plain_rkyv_serialize_1000", |b| {
        b.iter(|| {
            let mut results = Vec::with_capacity(data.len());
            for item in &data {
                results.push(rkyv::to_bytes::<rkyv::rancor::Error>(black_box(item)).unwrap());
            }
            results
        });
    });
}

fn bench_rkyv_with_prealloc(c: &mut Criterion) {
    let data = create_test_data(1000);

    c.bench_function("rkyv_prealloc_serialize_1000", |b| {
        b.iter(|| {
            // This is what DX Machine does - pre-allocate
            let mut results = Vec::with_capacity(data.len());
            for item in &data {
                results.push(rkyv::to_bytes::<rkyv::rancor::Error>(black_box(item)).unwrap());
            }
            results
        });
    });
}

#[cfg(feature = "parallel")]
fn bench_parallel_rkyv(c: &mut Criterion) {
    use rayon::prelude::*;
    let data = create_test_data(1000);

    c.bench_function("parallel_rkyv_serialize_1000", |b| {
        b.iter(|| {
            data.par_iter()
                .map(|item| rkyv::to_bytes::<rkyv::rancor::Error>(black_box(item)).unwrap())
                .collect::<Vec<_>>()
        });
    });
}

fn bench_roundtrip(c: &mut Criterion) {
    let data = TestData {
        id: 42,
        name: "Test Person".to_string(),
        age: 30,
        active: true,
    };

    c.bench_function("rkyv_roundtrip", |b| {
        b.iter(|| {
            let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(black_box(&data)).unwrap();
            unsafe {
                let archived = rkyv::access_unchecked::<TestData::Archived>(&bytes);
                let mut deserializer = rkyv::de::Pool::new();
                let _: TestData =
                    archived.deserialize(rkyv::rancor::Strategy::wrap(&mut deserializer)).unwrap();
            }
        });
    });
}

criterion_group!(benches, bench_plain_rkyv, bench_rkyv_with_prealloc, bench_roundtrip,);

#[cfg(feature = "parallel")]
criterion_group!(parallel_benches, bench_parallel_rkyv,);

#[cfg(feature = "parallel")]
criterion_main!(benches, parallel_benches);

#[cfg(not(feature = "parallel"))]
criterion_main!(benches);
