use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use serializer::llm::parser::LlmParser;
use serializer::machine::machine_types::MachineDocument;
use serializer::machine::serialize;

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

fn bench_pure_rkyv_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("pure_rkyv_serialize");

    for (name, llm_data) in create_test_data() {
        let doc = LlmParser::parse(&llm_data).unwrap();
        let machine_doc = MachineDocument::from(&doc);

        group.bench_with_input(BenchmarkId::new("serialize", name), &machine_doc, |b, m| {
            b.iter(|| {
                let bytes = serialize(black_box(m)).unwrap();
                black_box(bytes);
            });
        });
    }

    group.finish();
}

fn bench_pure_rkyv_deserialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("pure_rkyv_deserialize");

    for (name, llm_data) in create_test_data() {
        let doc = LlmParser::parse(&llm_data).unwrap();
        let machine_doc = MachineDocument::from(&doc);
        let bytes = serialize(&machine_doc).unwrap();

        group.bench_with_input(BenchmarkId::new("deserialize", name), &bytes, |b, data| {
            b.iter(|| {
                let doc: MachineDocument =
                    rkyv::from_bytes::<_, rkyv::rancor::Error>(black_box(data)).unwrap();
                black_box(doc);
            });
        });
    }

    group.finish();
}

fn bench_pure_rkyv_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("pure_rkyv_access");

    for (name, llm_data) in create_test_data() {
        let doc = LlmParser::parse(&llm_data).unwrap();
        let machine_doc = MachineDocument::from(&doc);
        let bytes = serialize(&machine_doc).unwrap();

        group.bench_with_input(BenchmarkId::new("access", name), &bytes, |b, data| {
            b.iter(|| {
                // This simulates zero-copy access - just validate the archive
                let archived = unsafe {
                    rkyv::access_unchecked::<rkyv::Archived<MachineDocument>>(black_box(data))
                };
                black_box(archived);
            });
        });
    }

    group.finish();
}

fn bench_pure_rkyv_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("pure_rkyv_roundtrip");

    for (name, llm_data) in create_test_data() {
        let doc = LlmParser::parse(&llm_data).unwrap();
        let machine_doc = MachineDocument::from(&doc);

        group.bench_with_input(BenchmarkId::new("roundtrip", name), &machine_doc, |b, m| {
            b.iter(|| {
                let bytes = serialize(black_box(m)).unwrap();
                let doc: MachineDocument =
                    rkyv::from_bytes::<_, rkyv::rancor::Error>(&bytes).unwrap();
                black_box(doc);
            });
        });
    }

    group.finish();
}

fn bench_size_comparison(c: &mut Criterion) {
    let group = c.benchmark_group("size_comparison");

    for (name, llm_data) in create_test_data() {
        let doc = LlmParser::parse(&llm_data).unwrap();
        let machine_doc = MachineDocument::from(&doc);
        let rkyv_bytes = serialize(&machine_doc).unwrap();

        #[cfg(feature = "compression")]
        {
            use serializer::machine::compress::DxCompressed;
            let compressed = DxCompressed::compress(&rkyv_bytes);
            let compressed_size = compressed.compressed_size();

            println!(
                "\n{}: LLM={} bytes, Pure RKYV={} bytes, RKYV+LZ4={} bytes",
                name,
                llm_data.len(),
                rkyv_bytes.len(),
                compressed_size
            );
            println!(
                "  Pure RKYV: {:.1}% of LLM",
                (rkyv_bytes.len() as f64 / llm_data.len() as f64) * 100.0
            );
            println!(
                "  RKYV+LZ4: {:.1}% of LLM, {:.1}% of Pure RKYV",
                (compressed_size as f64 / llm_data.len() as f64) * 100.0,
                (compressed_size as f64 / rkyv_bytes.len() as f64) * 100.0
            );
        }

        #[cfg(not(feature = "compression"))]
        {
            println!(
                "\n{}: LLM={} bytes, Pure RKYV={} bytes ({:.1}% of LLM)",
                name,
                llm_data.len(),
                rkyv_bytes.len(),
                (rkyv_bytes.len() as f64 / llm_data.len() as f64) * 100.0
            );
        }
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_pure_rkyv_serialize,
    bench_pure_rkyv_deserialize,
    bench_pure_rkyv_access,
    bench_pure_rkyv_roundtrip,
    bench_size_comparison
);
criterion_main!(benches);
