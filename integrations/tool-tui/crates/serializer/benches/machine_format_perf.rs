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
    ]
}

fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("machine_serialization");

    for (name, llm_data) in create_test_data() {
        group.bench_with_input(BenchmarkId::new("serialize", name), &llm_data, |b, data| {
            b.iter(|| {
                let machine = llm_to_machine(black_box(data)).unwrap();
                black_box(machine);
            });
        });
    }

    group.finish();
}

fn bench_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("machine_deserialization");

    for (name, llm_data) in create_test_data() {
        let machine = llm_to_machine(&llm_data).unwrap();

        group.bench_with_input(BenchmarkId::new("deserialize", name), &machine, |b, m| {
            b.iter(|| {
                let doc = machine_to_document(black_box(m)).unwrap();
                black_box(doc);
            });
        });
    }

    group.finish();
}

fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("machine_roundtrip");

    for (name, llm_data) in create_test_data() {
        group.bench_with_input(BenchmarkId::new("roundtrip", name), &llm_data, |b, data| {
            b.iter(|| {
                let machine = llm_to_machine(black_box(data)).unwrap();
                let doc = machine_to_document(&machine).unwrap();
                black_box(doc);
            });
        });
    }

    group.finish();
}

fn bench_compression_ratio(c: &mut Criterion) {
    let group = c.benchmark_group("compression_analysis");

    for (name, llm_data) in create_test_data() {
        let doc = LlmParser::parse(&llm_data).unwrap();
        let machine = document_to_machine(&doc);

        let llm_size = llm_data.len();
        let machine_size = machine.data.len();
        let ratio = (machine_size as f64 / llm_size as f64) * 100.0;

        println!(
            "\n{}: LLM={} bytes, Machine={} bytes ({:.1}% of LLM)",
            name, llm_size, machine_size, ratio
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_serialization,
    bench_deserialization,
    bench_roundtrip,
    bench_compression_ratio
);
criterion_main!(benches);
