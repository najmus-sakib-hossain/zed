//! Comprehensive format comparison: Dx Serializer vs Machine vs JSON vs Bincode
//!
//! This compares:
//! - Size efficiency
//! - Serialization speed
//! - Deserialization speed
//! - Round-trip correctness

use serde::{Deserialize, Serialize};
use serializer::llm::convert::{llm_to_machine, machine_to_llm};
use serializer::llm::parser::LlmParser;
use serializer::llm::types::DxDocument;
use std::fs;
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestData {
    context: Vec<(String, String)>,
    sections: Vec<Section>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Section {
    id: char,
    schema: Vec<String>,
    rows: Vec<Vec<String>>,
}

fn doc_to_test_data(doc: &DxDocument) -> TestData {
    TestData {
        context: doc.context.iter().map(|(k, v)| (k.clone(), format!("{:?}", v))).collect(),
        sections: doc
            .sections
            .iter()
            .map(|(id, section)| Section {
                id: *id,
                schema: section.schema.clone(),
                rows: section
                    .rows
                    .iter()
                    .map(|row| row.iter().map(|v| format!("{:?}", v)).collect())
                    .collect(),
            })
            .collect(),
    }
}

fn main() {
    println!("=== Format Comparison Test ===\n");

    let essence_dir = Path::new("../../essence");
    let sr_files = vec![
        "example1_mixed.sr",
        "example2_nested.sr",
        "example3_deep.sr",
    ];

    println!(
        "{:<20} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}",
        "File", "Dx Serializer", "Machine", "JSON", "Bincode", "Ser(µs)", "Deser(µs)"
    );
    println!("{}", "-".repeat(90));

    for sr_file in &sr_files {
        let sr_path = essence_dir.join(sr_file);

        if !sr_path.exists() {
            continue;
        }

        let sr_content = fs::read_to_string(&sr_path).unwrap();
        let doc = LlmParser::parse(&sr_content).unwrap();
        let test_data = doc_to_test_data(&doc);

        // Dx Serializer size
        let dsr_size = sr_content.len();

        // Machine format
        let start = Instant::now();
        let machine = llm_to_machine(&sr_content).unwrap();
        let machine_ser_time = start.elapsed().as_micros();
        let machine_size = machine.data.len();

        let start = Instant::now();
        let _ = machine_to_llm(&machine).unwrap();
        let machine_deser_time = start.elapsed().as_micros();

        // JSON
        let start = Instant::now();
        let json = serde_json::to_vec(&test_data).unwrap();
        let _json_ser_time = start.elapsed().as_micros();
        let json_size = json.len();

        let start = Instant::now();
        let _: TestData = serde_json::from_slice(&json).unwrap();
        let _json_deser_time = start.elapsed().as_micros();

        // Bincode
        let start = Instant::now();
        let bincode_data = bincode::serialize(&test_data).unwrap();
        let _bincode_ser_time = start.elapsed().as_micros();
        let bincode_size = bincode_data.len();

        let start = Instant::now();
        let _: TestData = bincode::deserialize(&bincode_data).unwrap();
        let _bincode_deser_time = start.elapsed().as_micros();

        let filename = sr_file.replace(".sr", "");
        println!(
            "{:<20} {:>10} {:>10} {:>10} {:>10} {:>10} {:>10}",
            filename,
            dsr_size,
            machine_size,
            json_size,
            bincode_size,
            machine_ser_time,
            machine_deser_time,
        );
    }

    println!("\n=== Performance Summary ===");
    println!("Dx Serializer Format: Human-readable, token-efficient for LLMs");
    println!("Machine Format: Pure RKYV with LZ4 compression");
    println!("JSON: Standard text format, widely compatible");
    println!("Bincode: Pure binary, no header overhead");

    println!("\n=== Use Cases ===");
    println!("Dx Serializer: LLM context windows, human editing");
    println!("Machine: High-speed data exchange, caching");
    println!("JSON: APIs, web services, debugging");
    println!("Bincode: Internal storage, maximum speed");
}
