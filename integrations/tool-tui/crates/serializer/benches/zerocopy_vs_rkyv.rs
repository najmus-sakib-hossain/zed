use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rkyv::{Archive, Deserialize, Serialize, rancor::Error};
use serializer::llm::{
    DxDocument, DxLlmValue, DxSection, ZeroCopyMachine, document_to_machine, machine_to_document,
};
use std::collections::HashMap;

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq, Clone)]
struct RkyvDocument {
    context: HashMap<String, RkyvValue>,
    sections: HashMap<char, RkyvSection>,
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq, Clone)]
struct RkyvSection {
    schema: Vec<String>,
    rows: Vec<Vec<RkyvValue>>,
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq, Clone)]
enum RkyvValue {
    Str(String),
    Num(f64),
    Bool(bool),
    Null,
}

fn create_test_document() -> DxDocument {
    let mut doc = DxDocument::new();

    // Add context
    doc.context.insert("name".to_string(), DxLlmValue::Str("dx".to_string()));
    doc.context.insert("version".to_string(), DxLlmValue::Str("0.0.1".to_string()));
    doc.context.insert("port".to_string(), DxLlmValue::Num(8080.0));

    // Add section with 10 rows instead of 100
    let mut section = DxSection::new(vec!["id".to_string(), "name".to_string()]);
    for i in 1..=10 {
        section.rows.push(vec![
            DxLlmValue::Num(i as f64),
            DxLlmValue::Str(format!("pkg-{}", i)),
        ]);
    }
    doc.sections.insert('d', section);

    doc
}

fn create_rkyv_document() -> RkyvDocument {
    let mut context = HashMap::new();
    context.insert("name".to_string(), RkyvValue::Str("dx".to_string()));
    context.insert("version".to_string(), RkyvValue::Str("0.0.1".to_string()));
    context.insert("port".to_string(), RkyvValue::Num(8080.0));

    let mut sections = HashMap::new();
    let mut rows = Vec::new();
    for i in 1..=10 {
        rows.push(vec![
            RkyvValue::Num(i as f64),
            RkyvValue::Str(format!("pkg-{}", i)),
        ]);
    }
    sections.insert(
        'd',
        RkyvSection {
            schema: vec!["id".to_string(), "name".to_string()],
            rows,
        },
    );

    RkyvDocument { context, sections }
}

fn bench_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize");
    group.sample_size(50); // Reduce sample size
    group.measurement_time(std::time::Duration::from_secs(3)); // Reduce measurement time

    let dx_doc = create_test_document();
    let rkyv_doc = create_rkyv_document();

    group.bench_function("dx_machine_original", |b| {
        b.iter(|| {
            let machine = document_to_machine(black_box(&dx_doc));
            black_box(machine);
        });
    });

    group.bench_function("dx_machine_zerocopy", |b| {
        b.iter(|| {
            let machine = ZeroCopyMachine::from_document(black_box(&dx_doc));
            black_box(machine);
        });
    });

    group.bench_function("rkyv", |b| {
        b.iter(|| {
            let bytes = rkyv::to_bytes::<Error>(black_box(&rkyv_doc)).unwrap();
            black_box(bytes);
        });
    });

    group.finish();
}

fn bench_deserialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("deserialize");
    group.sample_size(50);
    group.measurement_time(std::time::Duration::from_secs(3));

    let dx_doc = create_test_document();
    let dx_machine = document_to_machine(&dx_doc);
    let dx_zerocopy = ZeroCopyMachine::from_document(&dx_doc);

    let rkyv_doc = create_rkyv_document();
    let rkyv_bytes = rkyv::to_bytes::<Error>(&rkyv_doc).unwrap();

    group.bench_function("dx_machine_original", |b| {
        b.iter(|| {
            let doc = machine_to_document(black_box(&dx_machine)).unwrap();
            black_box(doc);
        });
    });

    group.bench_function("dx_machine_zerocopy", |b| {
        b.iter(|| {
            let doc = black_box(&dx_zerocopy).to_document().unwrap();
            black_box(doc);
        });
    });

    group.bench_function("rkyv", |b| {
        b.iter(|| {
            let archived =
                rkyv::access::<ArchivedRkyvDocument, Error>(black_box(&rkyv_bytes)).unwrap();
            let doc: RkyvDocument = rkyv::deserialize::<RkyvDocument, Error>(archived).unwrap();
            black_box(doc);
        });
    });

    group.finish();
}

fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");
    group.sample_size(50);
    group.measurement_time(std::time::Duration::from_secs(3));

    let dx_doc = create_test_document();
    let rkyv_doc = create_rkyv_document();

    group.bench_function("dx_machine_original", |b| {
        b.iter(|| {
            let machine = document_to_machine(black_box(&dx_doc));
            let doc = machine_to_document(&machine).unwrap();
            black_box(doc);
        });
    });

    group.bench_function("dx_machine_zerocopy", |b| {
        b.iter(|| {
            let machine = ZeroCopyMachine::from_document(black_box(&dx_doc));
            let doc = machine.to_document().unwrap();
            black_box(doc);
        });
    });

    group.bench_function("rkyv", |b| {
        b.iter(|| {
            let bytes = rkyv::to_bytes::<Error>(black_box(&rkyv_doc)).unwrap();
            let archived = rkyv::access::<ArchivedRkyvDocument, Error>(&bytes).unwrap();
            let doc: RkyvDocument = rkyv::deserialize::<RkyvDocument, Error>(archived).unwrap();
            black_box(doc);
        });
    });

    group.finish();
}

fn bench_size(_c: &mut Criterion) {
    let dx_doc = create_test_document();
    let dx_machine = document_to_machine(&dx_doc);
    let dx_zerocopy = ZeroCopyMachine::from_document(&dx_doc);

    let rkyv_doc = create_rkyv_document();
    let rkyv_bytes = rkyv::to_bytes::<Error>(&rkyv_doc).unwrap();

    println!("\n=== Size Comparison ===");
    println!("DX Machine (original): {} bytes", dx_machine.data.len());
    println!("DX Machine (zerocopy): {} bytes", dx_zerocopy.as_bytes().len());
    println!("rkyv format:           {} bytes", rkyv_bytes.len());

    let zerocopy_vs_original = ((dx_zerocopy.as_bytes().len() as f64
        - dx_machine.data.len() as f64)
        / dx_machine.data.len() as f64)
        * 100.0;
    let zerocopy_vs_rkyv = ((rkyv_bytes.len() as f64 - dx_zerocopy.as_bytes().len() as f64)
        / rkyv_bytes.len() as f64)
        * 100.0;

    println!(
        "\nZero-copy vs Original: {:.1}% {}",
        zerocopy_vs_original.abs(),
        if zerocopy_vs_original > 0.0 {
            "larger"
        } else {
            "smaller"
        }
    );
    println!("Zero-copy vs rkyv:     {:.1}% smaller", zerocopy_vs_rkyv);
    println!("======================\n");
}

criterion_group!(benches, bench_serialize, bench_deserialize, bench_roundtrip, bench_size);
criterion_main!(benches);
