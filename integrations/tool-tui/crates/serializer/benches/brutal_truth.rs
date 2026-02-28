use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rkyv::{Archive, Deserialize, Serialize};
use serializer::machine::{deserialize, deserialize_batch, serialize, serialize_batch};

#[derive(Archive, Serialize, Deserialize, Debug, Clone, PartialEq)]
struct TestData {
    id: u64,
    name: String,
    value: f64,
    active: bool,
}

fn create_test_data(count: usize) -> Vec<TestData> {
    (0..count)
        .map(|i| TestData {
            id: i as u64,
            name: format!("item-{}", i),
            value: i as f64 * 1.5,
            active: i % 2 == 0,
        })
        .collect()
}

fn test_correctness() {
    println!("\n=== BRUTAL TRUTH: Correctness Test ===");

    let original = vec![
        TestData {
            id: 1,
            name: "test1".to_string(),
            value: 1.5,
            active: true,
        },
        TestData {
            id: 2,
            name: "test2".to_string(),
            value: 2.5,
            active: false,
        },
        TestData {
            id: 3,
            name: "test3".to_string(),
            value: 3.5,
            active: true,
        },
    ];

    // Test DX-Machine serialize
    println!("Testing DX-Machine serialize...");
    let dx_bytes = match serialize(&original) {
        Ok(b) => {
            println!("✓ DX-Machine serialize: SUCCESS");
            b
        }
        Err(e) => {
            println!("✗ DX-Machine serialize: FAILED - {}", e);
            return;
        }
    };

    // Test DX-Machine deserialize
    println!("Testing DX-Machine deserialize...");
    let dx_result = unsafe { deserialize::<Vec<TestData>>(&dx_bytes) };

    if dx_result.len() == original.len() && dx_result[0].id == original[0].id {
        println!("✓ DX-Machine deserialize: SUCCESS");
    } else {
        println!("✗ DX-Machine deserialize: FAILED - Data mismatch");
        return;
    }

    // Test RKYV for comparison
    println!("\nTesting RKYV serialize...");
    let rkyv_bytes = match rkyv::to_bytes::<rkyv::rancor::Error>(&original) {
        Ok(b) => {
            println!("✓ RKYV serialize: SUCCESS");
            b
        }
        Err(e) => {
            println!("✗ RKYV serialize: FAILED - {}", e);
            return;
        }
    };

    println!("Testing RKYV deserialize...");
    match rkyv::from_bytes::<Vec<TestData>, rkyv::rancor::Error>(&rkyv_bytes) {
        Ok(result) => {
            if result == original {
                println!("✓ RKYV deserialize: SUCCESS");
            } else {
                println!("✗ RKYV deserialize: Data mismatch");
            }
        }
        Err(e) => {
            println!("✗ RKYV deserialize: FAILED - {}", e);
        }
    }

    // Size comparison
    println!("\n=== Size Comparison ===");
    println!("DX-Machine: {} bytes", dx_bytes.len());
    println!("RKYV:       {} bytes", rkyv_bytes.len());
    println!(
        "Difference: {} bytes ({:.1}%)",
        dx_bytes.len() as i64 - rkyv_bytes.len() as i64,
        ((dx_bytes.len() as f64 - rkyv_bytes.len() as f64) / rkyv_bytes.len() as f64) * 100.0
    );

    // Test batch
    println!("\n=== Batch Test ===");
    let items = create_test_data(10);

    println!("Testing DX-Machine batch serialize...");
    match serialize_batch(&items) {
        Ok(batches) => {
            println!("✓ DX-Machine batch serialize: SUCCESS ({} items)", batches.len());

            println!("Testing DX-Machine batch deserialize...");
            let deserialized = unsafe { deserialize_batch::<Vec<TestData>>(&batches) };
            if deserialized.len() == items.len() {
                println!("✓ DX-Machine batch deserialize: SUCCESS");
            } else {
                println!("✗ DX-Machine batch deserialize: Count mismatch");
            }
        }
        Err(e) => {
            println!("✗ DX-Machine batch serialize: FAILED - {}", e);
        }
    }

    println!("\n=== VERDICT ===");
    println!("DX-Machine is literally just RKYV with a wrapper.");
    println!("Performance: IDENTICAL to RKYV (it calls rkyv::to_bytes directly)");
    println!("Size: IDENTICAL to RKYV (same format)");
    println!("Batch optimization: Pre-allocates Vec capacity (minor improvement)");
    println!("================\n");
}

fn bench_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize");
    group.sample_size(50);

    let data = create_test_data(100);

    group.bench_function("dx_machine", |b| {
        b.iter(|| {
            let bytes = serialize(black_box(&data)).unwrap();
            black_box(bytes);
        });
    });

    group.bench_function("rkyv", |b| {
        b.iter(|| {
            let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(black_box(&data)).unwrap();
            black_box(bytes);
        });
    });

    group.finish();
}

fn bench_deserialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("deserialize");
    group.sample_size(50);

    let data = create_test_data(100);
    let dx_bytes = serialize(&data).unwrap();
    let rkyv_bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&data).unwrap();

    group.bench_function("dx_machine", |b| {
        b.iter(|| {
            let result = unsafe { deserialize::<Vec<TestData>>(black_box(&dx_bytes)) };
            black_box(result);
        });
    });

    group.bench_function("rkyv", |b| {
        b.iter(|| {
            let result =
                rkyv::from_bytes::<Vec<TestData>, rkyv::rancor::Error>(black_box(&rkyv_bytes))
                    .unwrap();
            black_box(result);
        });
    });

    group.finish();
}

fn bench_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch");
    group.sample_size(50);

    let items: Vec<TestData> = (0..100)
        .map(|i| TestData {
            id: i,
            name: format!("item-{}", i),
            value: i as f64,
            active: true,
        })
        .collect();

    group.bench_function("dx_machine_batch", |b| {
        b.iter(|| {
            let batches = serialize_batch(black_box(&items)).unwrap();
            black_box(batches);
        });
    });

    group.bench_function("rkyv_naive", |b| {
        b.iter(|| {
            let mut results = Vec::new();
            for item in black_box(&items) {
                results.push(rkyv::to_bytes::<rkyv::rancor::Error>(item).unwrap());
            }
            black_box(results);
        });
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_output_color(true);
    targets = bench_serialize, bench_deserialize, bench_batch
}

fn main() {
    test_correctness();
    criterion_main!(benches);
}
