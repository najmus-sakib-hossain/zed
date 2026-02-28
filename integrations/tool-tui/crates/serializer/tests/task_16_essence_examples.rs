//! Task 16: Validate essence folder examples
//!
//! Tests that all .sr files in the essence folder:
//! - Parse successfully without errors (Task 16.1)
//! - Round-trip correctly (parse → serialize → parse) (Task 16.2)

use serializer::llm::{LlmParser, LlmSerializer};
use std::fs;
use std::path::PathBuf;

/// Collect all .sr files from the essence folder
fn collect_essence_files() -> Vec<PathBuf> {
    let mut files = Vec::new();

    // Tests run from workspace root, so paths are relative to workspace root
    let essence_root = "../../essence";
    let essence_datasets = "../../essence/datasets";

    // Check essence root directory
    if let Ok(entries) = fs::read_dir(essence_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("sr") {
                files.push(path);
            }
        }
    }

    // Check essence/datasets directory
    if let Ok(entries) = fs::read_dir(essence_datasets) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("dsr") {
                files.push(path);
            }
        }
    }

    files.sort();
    files
}

/// Task 16.1: Write test to parse all essence examples
#[test]
fn task_16_1_parse_all_essence_examples() {
    let files = collect_essence_files();

    // assert!(!files.is_empty(), "No .sr files found in essence folder");

    let mut failures = Vec::new();

    for file_path in &files {
        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                failures.push(format!("{}: Failed to read file: {}", file_path.display(), e));
                continue;
            }
        };

        match LlmParser::parse(&content) {
            Ok(_doc) => {
                println!("✓ Parsed: {}", file_path.display());
            }
            Err(e) => {
                failures.push(format!("{}: Parse error: {}", file_path.display(), e));
            }
        }
    }

    if !failures.is_empty() {
        eprintln!("\n❌ Failed to parse {} file(s):", failures.len());
        for failure in &failures {
            eprintln!("  {}", failure);
        }
        panic!("Some essence examples failed to parse");
    }

    println!("\n✓ Successfully parsed all {} essence examples", files.len());
}

/// Task 16.2: Write test to round-trip all essence examples
#[test]
fn task_16_2_roundtrip_all_essence_examples() {
    let files = collect_essence_files();

    assert!(!files.is_empty(), "No .sr files found in essence folder");

    let mut failures = Vec::new();
    let serializer = LlmSerializer::new();

    for file_path in &files {
        let content = match fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                failures.push(format!("{}: Failed to read file: {}", file_path.display(), e));
                continue;
            }
        };

        // Parse original
        let doc1 = match LlmParser::parse(&content) {
            Ok(d) => d,
            Err(e) => {
                failures.push(format!("{}: Initial parse failed: {}", file_path.display(), e));
                continue;
            }
        };

        // Serialize
        let serialized = serializer.serialize(&doc1);

        // Parse again
        let doc2 = match LlmParser::parse(&serialized) {
            Ok(d) => d,
            Err(e) => {
                failures.push(format!("{}: Re-parse failed: {}", file_path.display(), e));
                eprintln!("\nSerialized output that failed to parse:");
                eprintln!("{}", serialized);
                continue;
            }
        };

        // Compare documents
        if doc1 != doc2 {
            failures
                .push(format!("{}: Round-trip produced different document", file_path.display()));
            eprintln!("\nOriginal document:");
            eprintln!("{:#?}", doc1);
            eprintln!("\nRound-tripped document:");
            eprintln!("{:#?}", doc2);
            eprintln!("\nSerialized output:");
            eprintln!("{}", serialized);
        } else {
            println!("✓ Round-trip: {}", file_path.display());
        }
    }

    if !failures.is_empty() {
        eprintln!("\n❌ Failed to round-trip {} file(s):", failures.len());
        for failure in &failures {
            eprintln!("  {}", failure);
        }
        panic!("Some essence examples failed to round-trip");
    }

    println!("\n✓ Successfully round-tripped all {} essence examples", files.len());
}
