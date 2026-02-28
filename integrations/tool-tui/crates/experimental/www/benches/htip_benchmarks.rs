//! # HTIP Benchmarks
//!
//! Benchmarks for HTIP serialization and deserialization throughput.
//!
//! Run with: `cargo bench --bench htip_benchmarks`

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use dx_www_binary::serializer::HtipWriter;
use dx_www_binary::deserializer::HtipStream;
use ed25519_dalek::SigningKey;

/// Generate a sample HTIP payload with the given number of templates and operations
fn generate_htip_payload(num_templates: usize, ops_per_template: usize) -> Vec<u8> {
    let mut writer = HtipWriter::new();
    
    // Create templates
    for i in 0..num_templates {
        let html = format!(
            r#"<div class="component-{}" data-id="{}">
                <h1><!--SLOT_0--></h1>
                <p class="description"><!--SLOT_1--></p>
                <span class="counter"><!--SLOT_2--></span>
            </div>"#,
            i, i
        );
        writer.write_template(i as u16, &html, vec![]);
    }
    
    // Create operations for each template
    for t in 0..num_templates {
        for op in 0..ops_per_template {
            let instance_id = (t * ops_per_template + op) as u32;
            writer.write_instantiate(instance_id, t as u16, 0);
            writer.write_patch_text(instance_id, 0, &format!("Title {}", op));
            writer.write_patch_text(instance_id, 1, &format!("Description for item {}", op));
            writer.write_patch_text(instance_id, 2, &format!("{}", op * 100));
            writer.write_class_toggle(instance_id, "active", op % 2 == 0);
        }
    }
    
    let signing_key = SigningKey::from_bytes(&[42u8; 32]);
    writer.finish_and_sign(&signing_key).expect("Failed to serialize HTIP")
}

/// Benchmark HTIP serialization throughput
fn bench_htip_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("htip_serialization");
    
    // Test different payload sizes
    let test_cases = [
        ("small", 5, 10),      // 5 templates, 10 ops each
        ("medium", 20, 50),    // 20 templates, 50 ops each
        ("large", 50, 100),    // 50 templates, 100 ops each
    ];
    
    for (name, num_templates, ops_per_template) in test_cases {
        // Estimate output size for throughput calculation
        let sample_output = generate_htip_payload(num_templates, ops_per_template);
        let output_size = sample_output.len();
        
        group.throughput(Throughput::Bytes(output_size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("serialize", name),
            &(num_templates, ops_per_template),
            |b, &(num_templates, ops_per_template)| {
                b.iter(|| {
                    let mut writer = HtipWriter::new();
                    
                    for i in 0..num_templates {
                        let html = format!(
                            r#"<div class="component-{}" data-id="{}">
                                <h1><!--SLOT_0--></h1>
                                <p class="description"><!--SLOT_1--></p>
                                <span class="counter"><!--SLOT_2--></span>
                            </div>"#,
                            i, i
                        );
                        writer.write_template(i as u16, &html, vec![]);
                    }
                    
                    for t in 0..num_templates {
                        for op in 0..ops_per_template {
                            let instance_id = (t * ops_per_template + op) as u32;
                            writer.write_instantiate(instance_id, t as u16, 0);
                            writer.write_patch_text(instance_id, 0, &format!("Title {}", op));
                            writer.write_patch_text(instance_id, 1, &format!("Description {}", op));
                            writer.write_patch_text(instance_id, 2, &format!("{}", op * 100));
                            writer.write_class_toggle(instance_id, "active", op % 2 == 0);
                        }
                    }
                    
                    let signing_key = SigningKey::from_bytes(&[42u8; 32]);
                    black_box(writer.finish_and_sign(&signing_key).expect("serialize"))
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark HTIP deserialization throughput
fn bench_htip_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("htip_deserialization");
    
    let test_cases = [
        ("small", 5, 10),
        ("medium", 20, 50),
        ("large", 50, 100),
    ];
    
    let signing_key = SigningKey::from_bytes(&[42u8; 32]);
    let verifying_key = signing_key.verifying_key();
    
    for (name, num_templates, ops_per_template) in test_cases {
        let binary = generate_htip_payload(num_templates, ops_per_template);
        let input_size = binary.len();
        
        group.throughput(Throughput::Bytes(input_size as u64));
        
        group.bench_with_input(
            BenchmarkId::new("deserialize", name),
            &binary,
            |b, binary| {
                b.iter(|| {
                    let stream = HtipStream::new(black_box(binary), &verifying_key)
                        .expect("deserialize");
                    black_box(stream)
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark HTIP operation iteration
fn bench_htip_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("htip_iteration");
    
    let signing_key = SigningKey::from_bytes(&[42u8; 32]);
    let verifying_key = signing_key.verifying_key();
    
    let binary = generate_htip_payload(20, 50);
    let stream = HtipStream::new(&binary, &verifying_key).expect("deserialize");
    let num_ops = stream.operations().len();
    
    group.throughput(Throughput::Elements(num_ops as u64));
    
    group.bench_function("iterate_operations", |b| {
        b.iter(|| {
            let stream = HtipStream::new(&binary, &verifying_key).expect("deserialize");
            let mut count = 0;
            for op in stream.operations() {
                black_box(op);
                count += 1;
            }
            black_box(count)
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_htip_serialization,
    bench_htip_deserialization,
    bench_htip_iteration
);

criterion_main!(benches);
