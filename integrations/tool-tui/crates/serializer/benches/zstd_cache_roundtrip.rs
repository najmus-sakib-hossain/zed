use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use serializer::llm::convert::{document_to_machine, machine_to_document};
use serializer::llm::parser::LlmParser;
use serializer::llm::types::{DxDocument, DxLlmValue};
use std::collections::HashMap;

fn create_test_document(size: &str) -> DxDocument {
    let mut doc = DxDocument::new();

    match size {
        "small" => {
            doc.context.insert("name".to_string(), DxLlmValue::Str("Test".to_string()));
            doc.context.insert("count".to_string(), DxLlmValue::Num(42.0));
            doc.context.insert("active".to_string(), DxLlmValue::Bool(true));
        }
        "medium" => {
            for i in 0..50 {
                doc.context
                    .insert(format!("field_{}", i), DxLlmValue::Str(format!("value_{}", i)));
            }
        }
        "large" => {
            for i in 0..500 {
                doc.context.insert(
                    format!("field_{}", i),
                    DxLlmValue::Str(format!("This is a longer value for field {}", i)),
                );
            }
        }
        _ => {}
    }

    doc
}

fn bench_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("zstd_serialize");

    for size in ["small", "medium", "large"].iter() {
        let doc = create_test_document(size);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let machine = document_to_machine(black_box(&doc));
                black_box(machine);
            });
        });
    }

    group.finish();
}

fn bench_deserialize_first_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("zstd_deserialize_first");

    for size in ["small", "medium", "large"].iter() {
        let doc = create_test_document(size);
        let machine = document_to_machine(&doc);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                // Create fresh machine format each time (no cache)
                let machine_fresh = document_to_machine(&doc);
                let result = machine_to_document(black_box(&machine_fresh)).unwrap();
                black_box(result);
            });
        });
    }

    group.finish();
}

fn bench_deserialize_cached(c: &mut Criterion) {
    let mut group = c.benchmark_group("zstd_deserialize_cached");

    for size in ["small", "medium", "large"].iter() {
        let doc = create_test_document(size);
        let machine = document_to_machine(&doc);

        // Prime the cache
        let _ = machine_to_document(&machine).unwrap();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let result = machine_to_document(black_box(&machine)).unwrap();
                black_box(result);
            });
        });
    }

    group.finish();
}

fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("zstd_roundtrip");

    for size in ["small", "medium", "large"].iter() {
        let doc = create_test_document(size);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let machine = document_to_machine(black_box(&doc));
                let result = machine_to_document(black_box(&machine)).unwrap();
                black_box(result);
            });
        });
    }

    group.finish();
}

fn test_correctness() {
    println!("\n=== CORRECTNESS TESTS ===\n");

    for size in ["small", "medium", "large"].iter() {
        let original = create_test_document(size);
        let machine = document_to_machine(&original);
        let restored = machine_to_document(&machine).unwrap();

        println!("Size: {}", size);
        println!("  Original fields: {}", original.context.len());
        println!("  Restored fields: {}", restored.context.len());
        println!("  Machine bytes: {}", machine.as_bytes().len());

        // Verify all fields match
        assert_eq!(original.context.len(), restored.context.len());
        for (key, value) in &original.context {
            let restored_value = restored.context.get(key).unwrap();
            match (value, restored_value) {
                (DxLlmValue::Str(a), DxLlmValue::Str(b)) => assert_eq!(a, b),
                (DxLlmValue::Num(a), DxLlmValue::Num(b)) => assert_eq!(a, b),
                (DxLlmValue::Bool(a), DxLlmValue::Bool(b)) => assert_eq!(a, b),
                _ => panic!("Type mismatch"),
            }
        }
        println!("  ✓ Round-trip verified\n");
    }

    // Test cache effectiveness
    println!("=== CACHE TEST ===\n");
    let doc = create_test_document("medium");
    let machine = document_to_machine(&doc);

    let start = std::time::Instant::now();
    let _ = machine_to_document(&machine).unwrap();
    let first_access = start.elapsed();

    let start = std::time::Instant::now();
    let _ = machine_to_document(&machine).unwrap();
    let cached_access = start.elapsed();

    println!("First access (decompress): {:?}", first_access);
    println!("Cached access: {:?}", cached_access);
    println!(
        "Speedup: {:.1}x faster\n",
        first_access.as_nanos() as f64 / cached_access.as_nanos() as f64
    );
}

criterion_group!(
    benches,
    bench_serialize,
    bench_deserialize_first_access,
    bench_deserialize_cached,
    bench_roundtrip
);
criterion_main!(benches);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_correctness_tests() {
        test_correctness();
    }
}
