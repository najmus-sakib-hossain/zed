use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use serializer::llm::convert::{document_to_machine, llm_to_machine, machine_to_document};
use serializer::llm::parser::LlmParser;

const THEME_SR: &str = r#"name=dx version=1.0.0 description=Dx_theme
dark:19[background=0,0,0 foreground=255,255,255 card=9,9,9 card_foreground=255,255,255 popover=18,18,18 popover_foreground=255,255,255 primary=0,201,80 primary_foreground=255,255,255 secondary=34,34,34 secondary_foreground=255,255,255 muted=29,29,29 muted_foreground=164,164,164 accent=0,201,80 accent_foreground=255,255,255 destructive=255,91,91 destructive_foreground=0,0,0 border=36,36,36 input=51,51,51 ring=164,164,164]
dark_modes:3[agent=0,201,80 plan=255,174,4 ask=38,113,244]
light:19[background=252,252,252 foreground=0,0,0 card=255,255,255 card_foreground=0,0,0 popover=252,252,252 popover_foreground=0,0,0 primary=0,0,0 primary_foreground=255,255,255 secondary=235,235,235 secondary_foreground=0,0,0 muted=245,245,245 muted_foreground=82,82,82 accent=235,235,235 accent_foreground=0,0,0 destructive=229,75,79 destructive_foreground=255,255,255 border=228,228,228 input=235,235,235 ring=0,0,0]
light_modes:3[agent=0,160,60 plan=200,130,0 ask=30,90,200]
"#;

fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("theme_serialization");

    // Benchmark: LLM to Machine (with LZ4 compression)
    group.bench_function("llm_to_machine_compressed", |b| {
        b.iter(|| {
            let machine = llm_to_machine(black_box(THEME_SR)).unwrap();
            black_box(machine);
        });
    });

    // Benchmark: LLM to Machine (uncompressed for comparison)
    group.bench_function("llm_to_machine_uncompressed", |b| {
        b.iter(|| {
            let doc = LlmParser::parse(black_box(THEME_SR)).unwrap();
            let machine = document_to_machine(&doc);
            black_box(machine);
        });
    });

    group.finish();
}

fn bench_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("theme_deserialization");

    // Prepare data
    let machine_compressed = llm_to_machine(THEME_SR).unwrap();
    let doc = LlmParser::parse(THEME_SR).unwrap();
    let machine_uncompressed = document_to_machine(&doc);

    // Benchmark: Machine to Document (compressed - includes decompression)
    group.bench_function("machine_to_document_compressed", |b| {
        b.iter(|| {
            let doc = machine_to_document(black_box(&machine_compressed)).unwrap();
            black_box(doc);
        });
    });

    // Benchmark: Machine to Document (uncompressed)
    group.bench_function("machine_to_document_uncompressed", |b| {
        b.iter(|| {
            let doc = machine_to_document(black_box(&machine_uncompressed)).unwrap();
            black_box(doc);
        });
    });

    group.finish();
}

fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("theme_roundtrip");

    // Benchmark: Full round-trip (compressed)
    group.bench_function("compressed_roundtrip", |b| {
        b.iter(|| {
            let machine = llm_to_machine(black_box(THEME_SR)).unwrap();
            let doc = machine_to_document(&machine).unwrap();
            black_box(doc);
        });
    });

    // Benchmark: Full round-trip (uncompressed)
    group.bench_function("uncompressed_roundtrip", |b| {
        b.iter(|| {
            let doc = LlmParser::parse(black_box(THEME_SR)).unwrap();
            let machine = document_to_machine(&doc);
            let doc2 = machine_to_document(&machine).unwrap();
            black_box(doc2);
        });
    });

    group.finish();
}

fn bench_compression_ratios(c: &mut Criterion) {
    let mut group = c.benchmark_group("compression_analysis");

    // Test different data sizes
    let sizes = vec![
        ("small_theme", THEME_SR.to_string()),
        ("medium_config", THEME_SR.repeat(5)),
        ("large_config", THEME_SR.repeat(20)),
    ];

    for (name, data) in &sizes {
        group.bench_with_input(BenchmarkId::new("compress", name), data, |b, data| {
            b.iter(|| {
                let machine = llm_to_machine(black_box(data)).unwrap();
                black_box(machine);
            });
        });
    }

    group.finish();

    // Print compression stats
    println!("\n=== COMPRESSION STATISTICS ===");
    for (name, data) in &sizes {
        let doc = LlmParser::parse(data).unwrap();
        let uncompressed = document_to_machine(&doc);
        let compressed = llm_to_machine(data).unwrap();

        let ratio = (compressed.data.len() as f64 / uncompressed.data.len() as f64) * 100.0;
        let savings = 100.0 - ratio;

        println!(
            "{}: {} bytes (LLM) -> {} bytes (uncompressed) -> {} bytes (compressed)",
            name,
            data.len(),
            uncompressed.data.len(),
            compressed.data.len()
        );
        println!("  Compression ratio: {:.1}% | Space savings: {:.1}%", ratio, savings);
    }
    println!("==============================\n");
}

criterion_group!(
    benches,
    bench_serialization,
    bench_deserialization,
    bench_roundtrip,
    bench_compression_ratios
);
criterion_main!(benches);
