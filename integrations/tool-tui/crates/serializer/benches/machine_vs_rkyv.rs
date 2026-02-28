use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rkyv::{Archive, Deserialize, Serialize, rancor::Error};
use serializer::llm::{
    DxDocument, DxLlmValue, DxSection, document_to_machine, machine_to_document,
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
    doc.context.insert("active".to_string(), DxLlmValue::Bool(true));
    doc.context.insert("debug".to_string(), DxLlmValue::Bool(false));

    // Add section with multiple rows
    let mut section =
        DxSection::new(vec!["id".to_string(), "name".to_string(), "version".to_string()]);
    for i in 1..=100 {
        section.rows.push(vec![
            DxLlmValue::Num(i as f64),
            DxLlmValue::Str(format!("package-{}", i)),
            DxLlmValue::Str(format!("{}.0.0", i)),
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
    context.insert("active".to_string(), RkyvValue::Bool(true));
    context.insert("debug".to_string(), RkyvValue::Bool(false));

    let mut sections = HashMap::new();
    let mut rows = Vec::new();
    for i in 1..=100 {
        rows.push(vec![
            RkyvValue::Num(i as f64),
            RkyvValue::Str(format!("package-{}", i)),
            RkyvValue::Str(format!("{}.0.0", i)),
        ]);
    }
    sections.insert(
        'd',
        RkyvSection {
            schema: vec!["id".to_string(), "name".to_string(), "version".to_string()],
            rows,
        },
    );

    RkyvDocument { context, sections }
}

fn bench_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize");

    let dx_doc = create_test_document();
    let rkyv_doc = create_rkyv_document();

    group.bench_function("dx_machine", |b| {
        b.iter(|| {
            let machine = document_to_machine(black_box(&dx_doc));
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

    let dx_doc = create_test_document();
    let dx_machine = document_to_machine(&dx_doc);

    let rkyv_doc = create_rkyv_document();
    let rkyv_bytes = rkyv::to_bytes::<Error>(&rkyv_doc).unwrap();

    group.bench_function("dx_machine", |b| {
        b.iter(|| {
            let doc = machine_to_document(black_box(&dx_machine)).unwrap();
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

    let dx_doc = create_test_document();
    let rkyv_doc = create_rkyv_document();

    group.bench_function("dx_machine", |b| {
        b.iter(|| {
            let machine = document_to_machine(black_box(&dx_doc));
            let doc = machine_to_document(&machine).unwrap();
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

    let rkyv_doc = create_rkyv_document();
    let rkyv_bytes = rkyv::to_bytes::<Error>(&rkyv_doc).unwrap();

    println!("\n=== Size Comparison ===");
    println!("DX Machine format: {} bytes", dx_machine.data.len());
    println!("rkyv format:       {} bytes", rkyv_bytes.len());
    println!(
        "Difference:        {} bytes ({:.1}%)",
        rkyv_bytes.len() as i32 - dx_machine.data.len() as i32,
        ((rkyv_bytes.len() as f64 - dx_machine.data.len() as f64) / rkyv_bytes.len() as f64)
            * 100.0
    );
    println!("======================\n");
}

criterion_group!(benches, bench_serialize, bench_deserialize, bench_roundtrip, bench_size);
criterion_main!(benches);
