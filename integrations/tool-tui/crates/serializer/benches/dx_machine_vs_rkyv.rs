use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rkyv::{Archive, Deserialize, Serialize};
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};
use serializer::machine::{from_bytes_serde as dx_from_bytes, to_bytes_serde as dx_to_bytes};
use std::hint::black_box as hint_black_box;

// Test data structures
#[derive(Archive, Deserialize, Serialize, SerdeSerialize, SerdeDeserialize, Debug, Clone)]
struct LogEntry {
    timestamp: String,
    level: String,
    endpoint: String,
    status: u16,
    time: u32,
    error: Option<String>,
}

#[derive(Archive, Deserialize, Serialize, SerdeSerialize, SerdeDeserialize, Debug, Clone)]
struct Order {
    id: String,
    customer: String,
    email: String,
    items: String,
    total: u32,
    status: String,
    date: String,
}

#[derive(Archive, Deserialize, Serialize, SerdeSerialize, SerdeDeserialize, Debug, Clone)]
struct Config {
    name: String,
    version: String,
    author: String,
    enabled: bool,
    port: u16,
    timeout: u32,
}

fn generate_logs(count: usize) -> Vec<LogEntry> {
    (0..count)
        .map(|i| LogEntry {
            timestamp: format!("2025-01-15T10:{}:{}Z", i % 60, i % 60),
            level: if i % 3 == 0 {
                "error".to_string()
            } else {
                "info".to_string()
            },
            endpoint: format!("/api/endpoint{}", i % 10),
            status: if i % 3 == 0 { 500 } else { 200 },
            time: (i % 300) as u32,
            error: if i % 3 == 0 {
                Some("Error message".to_string())
            } else {
                None
            },
        })
        .collect()
}

fn generate_orders(count: usize) -> Vec<Order> {
    (0..count)
        .map(|i| Order {
            id: format!("ORD-{:03}", i),
            customer: format!("Customer {}", i),
            email: format!("customer{}@example.com", i),
            items: "WIDGET:2:30|GADGET:1:50".to_string(),
            total: 100 + (i % 200) as u32,
            status: if i % 2 == 0 {
                "shipped".to_string()
            } else {
                "delivered".to_string()
            },
            date: format!("2025-01-{:02}", (i % 28) + 1),
        })
        .collect()
}

fn generate_configs(count: usize) -> Vec<Config> {
    (0..count)
        .map(|i| Config {
            name: format!("config-{}", i),
            version: "1.2.3".to_string(),
            author: "test".to_string(),
            enabled: i % 2 == 0,
            port: 8080 + (i % 100) as u16,
            timeout: 30000,
        })
        .collect()
}

fn bench_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize");

    for size in [10, 100, 1000].iter() {
        let logs = generate_logs(*size);
        let orders = generate_orders(*size);
        let configs = generate_configs(*size);

        // DX-Machine
        group.bench_with_input(BenchmarkId::new("dx_machine_logs", size), size, |b, _| {
            b.iter(|| {
                let bytes = dx_to_bytes(&logs).unwrap();
                hint_black_box(bytes)
            })
        });

        group.bench_with_input(BenchmarkId::new("dx_machine_orders", size), size, |b, _| {
            b.iter(|| {
                let bytes = dx_to_bytes(&orders).unwrap();
                hint_black_box(bytes)
            })
        });

        group.bench_with_input(BenchmarkId::new("dx_machine_configs", size), size, |b, _| {
            b.iter(|| {
                let bytes = dx_to_bytes(&configs).unwrap();
                hint_black_box(bytes)
            })
        });

        // RKYV
        group.bench_with_input(BenchmarkId::new("rkyv_logs", size), size, |b, _| {
            b.iter(|| {
                let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&logs).unwrap();
                hint_black_box(bytes)
            })
        });

        group.bench_with_input(BenchmarkId::new("rkyv_orders", size), size, |b, _| {
            b.iter(|| {
                let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&orders).unwrap();
                hint_black_box(bytes)
            })
        });

        group.bench_with_input(BenchmarkId::new("rkyv_configs", size), size, |b, _| {
            b.iter(|| {
                let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&configs).unwrap();
                hint_black_box(bytes)
            })
        });

        // Bincode
        group.bench_with_input(BenchmarkId::new("bincode_logs", size), size, |b, _| {
            b.iter(|| {
                let bytes = bincode::serialize(&logs).unwrap();
                hint_black_box(bytes)
            })
        });

        group.bench_with_input(BenchmarkId::new("bincode_orders", size), size, |b, _| {
            b.iter(|| {
                let bytes = bincode::serialize(&orders).unwrap();
                hint_black_box(bytes)
            })
        });

        group.bench_with_input(BenchmarkId::new("bincode_configs", size), size, |b, _| {
            b.iter(|| {
                let bytes = bincode::serialize(&configs).unwrap();
                hint_black_box(bytes)
            })
        });

        // Postcard
        group.bench_with_input(BenchmarkId::new("postcard_logs", size), size, |b, _| {
            b.iter(|| {
                let bytes = postcard::to_allocvec(&logs).unwrap();
                hint_black_box(bytes)
            })
        });

        group.bench_with_input(BenchmarkId::new("postcard_orders", size), size, |b, _| {
            b.iter(|| {
                let bytes = postcard::to_allocvec(&orders).unwrap();
                hint_black_box(bytes)
            })
        });

        group.bench_with_input(BenchmarkId::new("postcard_configs", size), size, |b, _| {
            b.iter(|| {
                let bytes = postcard::to_allocvec(&configs).unwrap();
                hint_black_box(bytes)
            })
        });
    }

    group.finish();
}

fn bench_deserialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("deserialize");

    for size in [10, 100, 1000].iter() {
        let logs = generate_logs(*size);
        let orders = generate_orders(*size);
        let configs = generate_configs(*size);

        // DX-Machine
        let dx_logs = dx_to_bytes(&logs).unwrap();
        group.bench_with_input(BenchmarkId::new("dx_machine_logs", size), size, |b, _| {
            b.iter(|| {
                let decoded: Vec<LogEntry> = dx_from_bytes(&dx_logs).unwrap();
                hint_black_box(decoded)
            })
        });

        let dx_orders = dx_to_bytes(&orders).unwrap();
        group.bench_with_input(BenchmarkId::new("dx_machine_orders", size), size, |b, _| {
            b.iter(|| {
                let decoded: Vec<Order> = dx_from_bytes(&dx_orders).unwrap();
                hint_black_box(decoded)
            })
        });

        let dx_configs = dx_to_bytes(&configs).unwrap();
        group.bench_with_input(BenchmarkId::new("dx_machine_configs", size), size, |b, _| {
            b.iter(|| {
                let decoded: Vec<Config> = dx_from_bytes(&dx_configs).unwrap();
                hint_black_box(decoded)
            })
        });

        // RKYV - deserialize (not zero-copy access)
        let rkyv_logs = rkyv::to_bytes::<rkyv::rancor::Error>(&logs).unwrap();
        group.bench_with_input(BenchmarkId::new("rkyv_logs", size), size, |b, _| {
            b.iter(|| {
                let deserialized: Vec<LogEntry> =
                    rkyv::from_bytes::<Vec<LogEntry>, rkyv::rancor::Error>(&rkyv_logs).unwrap();
                hint_black_box(deserialized)
            })
        });

        let rkyv_orders = rkyv::to_bytes::<rkyv::rancor::Error>(&orders).unwrap();
        group.bench_with_input(BenchmarkId::new("rkyv_orders", size), size, |b, _| {
            b.iter(|| {
                let deserialized: Vec<Order> =
                    rkyv::from_bytes::<Vec<Order>, rkyv::rancor::Error>(&rkyv_orders).unwrap();
                hint_black_box(deserialized)
            })
        });

        let rkyv_configs = rkyv::to_bytes::<rkyv::rancor::Error>(&configs).unwrap();
        group.bench_with_input(BenchmarkId::new("rkyv_configs", size), size, |b, _| {
            b.iter(|| {
                let deserialized: Vec<Config> =
                    rkyv::from_bytes::<Vec<Config>, rkyv::rancor::Error>(&rkyv_configs).unwrap();
                hint_black_box(deserialized)
            })
        });

        // Bincode
        let bincode_logs = bincode::serialize(&logs).unwrap();
        group.bench_with_input(BenchmarkId::new("bincode_logs", size), size, |b, _| {
            b.iter(|| {
                let decoded: Vec<LogEntry> = bincode::deserialize(&bincode_logs).unwrap();
                hint_black_box(decoded)
            })
        });

        let bincode_orders = bincode::serialize(&orders).unwrap();
        group.bench_with_input(BenchmarkId::new("bincode_orders", size), size, |b, _| {
            b.iter(|| {
                let decoded: Vec<Order> = bincode::deserialize(&bincode_orders).unwrap();
                hint_black_box(decoded)
            })
        });

        let bincode_configs = bincode::serialize(&configs).unwrap();
        group.bench_with_input(BenchmarkId::new("bincode_configs", size), size, |b, _| {
            b.iter(|| {
                let decoded: Vec<Config> = bincode::deserialize(&bincode_configs).unwrap();
                hint_black_box(decoded)
            })
        });

        // Postcard
        let postcard_logs = postcard::to_allocvec(&logs).unwrap();
        group.bench_with_input(BenchmarkId::new("postcard_logs", size), size, |b, _| {
            b.iter(|| {
                let decoded: Vec<LogEntry> = postcard::from_bytes(&postcard_logs).unwrap();
                hint_black_box(decoded)
            })
        });

        let postcard_orders = postcard::to_allocvec(&orders).unwrap();
        group.bench_with_input(BenchmarkId::new("postcard_orders", size), size, |b, _| {
            b.iter(|| {
                let decoded: Vec<Order> = postcard::from_bytes(&postcard_orders).unwrap();
                hint_black_box(decoded)
            })
        });

        let postcard_configs = postcard::to_allocvec(&configs).unwrap();
        group.bench_with_input(BenchmarkId::new("postcard_configs", size), size, |b, _| {
            b.iter(|| {
                let decoded: Vec<Config> = postcard::from_bytes(&postcard_configs).unwrap();
                hint_black_box(decoded)
            })
        });
    }

    group.finish();
}

fn bench_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("size_comparison");

    for size in [10, 100, 1000].iter() {
        let logs = generate_logs(*size);

        let rkyv_size = rkyv::to_bytes::<rkyv::rancor::Error>(&logs).unwrap().len();
        let bincode_size = bincode::serialize(&logs).unwrap().len();
        let postcard_size = postcard::to_allocvec(&logs).unwrap().len();

        println!("\n=== Size Comparison (n={}) ===", size);
        println!("RKYV:     {} bytes", rkyv_size);
        println!("Bincode:  {} bytes", bincode_size);
        println!("Postcard: {} bytes", postcard_size);
    }

    group.finish();
}

criterion_group!(benches, bench_serialize, bench_deserialize);
criterion_main!(benches);
