use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use serde::{Deserialize, Serialize};
use serializer::machine::serde_compat::{from_bytes, to_bytes};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Person {
    name: String,
    age: u32,
    email: String,
    active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Product {
    id: u64,
    name: String,
    price: f64,
    in_stock: bool,
    tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Order {
    order_id: u64,
    customer: Person,
    products: Vec<Product>,
    total: f64,
    timestamp: u64,
}

fn create_test_person() -> Person {
    Person {
        name: "John Doe".to_string(),
        age: 30,
        email: "john.doe@example.com".to_string(),
        active: true,
    }
}

fn create_test_product(id: u64) -> Product {
    Product {
        id,
        name: format!("Product {}", id),
        price: 19.99 + (id as f64 * 5.0),
        in_stock: id % 2 == 0,
        tags: vec![
            "electronics".to_string(),
            "gadgets".to_string(),
            format!("category-{}", id % 5),
        ],
    }
}

fn create_test_order(num_products: usize) -> Order {
    Order {
        order_id: 12345,
        customer: create_test_person(),
        products: (0..num_products).map(|i| create_test_product(i as u64)).collect(),
        total: 199.99,
        timestamp: 1704067200,
    }
}

fn bench_serialize_person(c: &mut Criterion) {
    let person = create_test_person();

    c.bench_function("dx_machine_serialize_person", |b| {
        b.iter(|| {
            let bytes = to_bytes(black_box(&person)).unwrap();
            black_box(bytes);
        });
    });
}

fn bench_deserialize_person(c: &mut Criterion) {
    let person = create_test_person();
    let bytes = to_bytes(&person).unwrap();

    c.bench_function("dx_machine_deserialize_person", |b| {
        b.iter(|| {
            let result: Person = from_bytes(black_box(&bytes)).unwrap();
            black_box(result);
        });
    });
}

fn bench_roundtrip_person(c: &mut Criterion) {
    let person = create_test_person();

    c.bench_function("dx_machine_roundtrip_person", |b| {
        b.iter(|| {
            let bytes = to_bytes(black_box(&person)).unwrap();
            let result: Person = from_bytes(&bytes).unwrap();
            black_box(result);
        });
    });
}

fn bench_serialize_orders(c: &mut Criterion) {
    let mut group = c.benchmark_group("dx_machine_serialize_orders");

    for size in [1, 10, 100, 1000].iter() {
        let order = create_test_order(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let bytes = to_bytes(black_box(&order)).unwrap();
                black_box(bytes);
            });
        });
    }

    group.finish();
}

fn bench_deserialize_orders(c: &mut Criterion) {
    let mut group = c.benchmark_group("dx_machine_deserialize_orders");

    for size in [1, 10, 100, 1000].iter() {
        let order = create_test_order(*size);
        let bytes = to_bytes(&order).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let result: Order = from_bytes(black_box(&bytes)).unwrap();
                black_box(result);
            });
        });
    }

    group.finish();
}

fn bench_roundtrip_orders(c: &mut Criterion) {
    let mut group = c.benchmark_group("dx_machine_roundtrip_orders");

    for size in [1, 10, 100, 1000].iter() {
        let order = create_test_order(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let bytes = to_bytes(black_box(&order)).unwrap();
                let result: Order = from_bytes(&bytes).unwrap();
                black_box(result);
            });
        });
    }

    group.finish();
}

fn verify_correctness() {
    println!("\n=== DX-Machine Format Correctness Verification ===\n");

    // Test 1: Simple person
    let person = create_test_person();
    let bytes = to_bytes(&person).expect("Failed to serialize person");
    println!("Person serialized: {} bytes", bytes.len());
    println!("Binary (hex): {}", hex::encode(&bytes[..bytes.len().min(64)]));

    let decoded: Person = from_bytes(&bytes).expect("Failed to deserialize person");
    assert_eq!(person, decoded, "Person roundtrip failed");
    println!("✓ Person roundtrip successful\n");

    // Test 2: Product with vector
    let product = create_test_product(42);
    let bytes = to_bytes(&product).expect("Failed to serialize product");
    println!("Product serialized: {} bytes", bytes.len());
    println!("Binary (hex): {}", hex::encode(&bytes[..bytes.len().min(64)]));

    let decoded: Product = from_bytes(&bytes).expect("Failed to deserialize product");
    assert_eq!(product, decoded, "Product roundtrip failed");
    println!("✓ Product roundtrip successful\n");

    // Test 3: Complex order
    for num_products in [1, 10, 100] {
        let order = create_test_order(num_products);
        let bytes = to_bytes(&order).expect("Failed to serialize order");
        println!("Order with {} products serialized: {} bytes", num_products, bytes.len());

        let decoded: Order = from_bytes(&bytes).expect("Failed to deserialize order");
        assert_eq!(order, decoded, "Order roundtrip failed");
        println!("✓ Order with {} products roundtrip successful", num_products);
    }

    println!("\n=== All Correctness Tests Passed ===\n");
}

criterion_group!(
    benches,
    bench_serialize_person,
    bench_deserialize_person,
    bench_roundtrip_person,
    bench_serialize_orders,
    bench_deserialize_orders,
    bench_roundtrip_orders,
);

criterion_main!(benches);

// Run correctness verification before benchmarks
#[ctor::ctor]
fn init() {
    verify_correctness();
}
