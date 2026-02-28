use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use serializer::llm::parser::LlmParser;
use serializer::{document_to_machine, llm_to_machine, machine_to_document};

fn create_test_data() -> Vec<(&'static str, String)> {
    vec![
        ("small", "name=John\nage=30\nemail=john@example.com".to_string()),
        (
            "medium",
            format!(
                "{}\n{}\n{}\n{}\n{}",
                "name=John Doe", "age=30", "email=john@example.com", "city=New York", "country=USA",
            ),
        ),
        ("large", {
            let mut data = String::new();
            for i in 0..100 {
                data.push_str(&format!("field{}=value{}\n", i, i));
            }
            data
        }),
        ("xlarge", {
            let mut data = String::new();
            for i in 0..1000 {
                data.push_str(&format!("field{}=value{}\n", i, i));
            }
            data
        }),
    ]
}

fn bench_with_compression_first_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("with_lz4_first_access");

    for (name, llm_data) in create_test_data() {
        group.bench_with_input(BenchmarkId::new("deserialize", name), &llm_data, |b, data| {
            b.iter(|| {
                let machine = llm_to_machine(black_box(data)).unwrap();
                let doc = machine_to_document(&machine).unwrap();
                black_box(doc);
            });
        });
    }

    group.finish();
}

fn bench_with_compression_cached_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("with_lz4_cached_access");

    for (name, llm_data) in create_test_data() {
        let machine = llm_to_machine(&llm_data).unwrap();

        group.bench_with_input(BenchmarkId::new("deserialize", name), &machine, |b, m| {
            // First access to populate cache
            let _ = machine_to_document(m).unwrap();

            b.iter(|| {
                let doc = machine_to_document(black_box(m)).unwrap();
                black_box(doc);
            });
        });
    }

    group.finish();
}

fn bench_without_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("pure_rkyv_no_compression");

    for (name, llm_data) in create_test_data() {
        let doc = LlmParser::parse(&llm_data).unwrap();

        group.bench_with_input(BenchmarkId::new("serialize", name), &doc, |b, d| {
            b.iter(|| {
                let machine = document_to_machine(black_box(d));
                black_box(machine);
            });
        });
    }

    group.finish();
}

fn bench_compression_ratios(c: &mut Criterion) {
    let _group = c.benchmark_group("compression_analysis");

    println!("\n=== COMPRESSION RATIO ANALYSIS ===\n");

    for (name, llm_data) in create_test_data() {
        let doc = LlmParser::parse(&llm_data).unwrap();
        let machine = document_to_machine(&doc);

        let llm_size = llm_data.len();
        let machine_size = machine.data.len();
        let ratio = (machine_size as f64 / llm_size as f64) * 100.0;
        let savings = 100.0 - ratio;

        println!(
            "{:8} | LLM: {:6} bytes | Machine: {:6} bytes | Ratio: {:5.1}% | Savings: {:5.1}%",
            name, llm_size, machine_size, ratio, savings
        );
    }

    println!("\n=== RECOMMENDATION ===");
    println!("If savings < 20% on average: Use pure RKYV by default");
    println!("If savings > 30% on average: Use LZ4 by default");
    println!("If 20-30%: Borderline - consider use case\n");
}

criterion_group!(
    benches,
    bench_with_compression_first_access,
    bench_with_compression_cached_access,
    bench_without_compression,
    bench_compression_ratios
);
criterion_main!(benches);
