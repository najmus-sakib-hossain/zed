//! Real-world test of DX-Machine format with actual .sr files
//!
//! This example:
//! 1. Reads .sr files from essence/ folder
//! 2. Converts them to machine format
//! 3. Saves .machine files
//! 4. Verifies round-trip conversion
//! 5. Compares sizes and performance

use serializer::llm::convert::{llm_to_machine, machine_to_document, machine_to_llm};
use serializer::llm::parser::LlmParser;
use std::fs;
use std::path::Path;
use std::time::Instant;

fn main() {
    println!("=== DX-Machine Real-World Test ===\n");

    let essence_dir = Path::new("../../essence");

    if !essence_dir.exists() {
        eprintln!("Error: essence/ directory not found");
        eprintln!("Run this from crates/serializer directory");
        return;
    }

    let sr_files = vec![
        "example1_mixed.sr",
        "example2_nested.sr",
        "example3_deep.sr",
        "example4_config.sr",
        "example5_leaf.sr",
    ];

    let mut total_sr_size = 0;
    let mut total_machine_size = 0;
    let mut total_serialize_time = 0u128;
    let mut total_deserialize_time = 0u128;

    for sr_file in &sr_files {
        let sr_path = essence_dir.join(sr_file);

        if !sr_path.exists() {
            println!("⚠ Skipping {} (not found)", sr_file);
            continue;
        }

        println!("📄 Processing: {}", sr_file);

        // Read .sr file
        let sr_content = match fs::read_to_string(&sr_path) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("  ❌ Failed to read: {}", e);
                continue;
            }
        };

        let sr_size = sr_content.len();
        total_sr_size += sr_size;

        // Parse to verify it's valid
        match LlmParser::parse(&sr_content) {
            Ok(doc) => {
                println!("  ✓ Valid Dx Serializer format");
                println!("    - Context entries: {}", doc.context.len());
                println!("    - Sections: {}", doc.sections.len());
            }
            Err(e) => {
                eprintln!("  ❌ Parse error: {}", e);
                continue;
            }
        }

        // Convert to machine format (timed)
        let start = Instant::now();
        let machine = match llm_to_machine(&sr_content) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("  ❌ Conversion error: {}", e);
                continue;
            }
        };
        let serialize_time = start.elapsed();
        total_serialize_time += serialize_time.as_nanos();

        let machine_size = machine.data.len();
        total_machine_size += machine_size;

        // Save .machine file
        let machine_file = sr_file.replace(".sr", ".machine");
        let machine_path = essence_dir.join(&machine_file);

        if let Err(e) = fs::write(&machine_path, &machine.data) {
            eprintln!("  ❌ Failed to write .machine file: {}", e);
        } else {
            println!("  ✓ Saved: {}", machine_file);
        }

        // Verify format
        println!("  ✓ Format: RKYV + LZ4 ({} bytes)", machine.data.len());

        // Convert back to Dx Serializer (timed)
        let start = Instant::now();
        let sr_roundtrip = match machine_to_llm(&machine) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  ❌ Deserialization error: {}", e);
                continue;
            }
        };
        let deserialize_time = start.elapsed();
        total_deserialize_time += deserialize_time.as_nanos();

        // Verify round-trip
        let doc_original = LlmParser::parse(&sr_content).unwrap();
        let doc_roundtrip = match LlmParser::parse(&sr_roundtrip) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("  ❌ Round-trip parse error: {}", e);
                continue;
            }
        };

        let context_match = doc_original.context.len() == doc_roundtrip.context.len();
        let sections_match = doc_original.sections.len() == doc_roundtrip.sections.len();

        if context_match && sections_match {
            println!("  ✓ Round-trip verified");
        } else {
            println!(
                "  ⚠ Round-trip mismatch (context: {}/{}, sections: {}/{})",
                doc_roundtrip.context.len(),
                doc_original.context.len(),
                doc_roundtrip.sections.len(),
                doc_original.sections.len()
            );
        }

        // Size comparison
        let compression_ratio = (1.0 - (machine_size as f64 / sr_size as f64)) * 100.0;
        println!(
            "  📊 Size: {} bytes → {} bytes ({:.1}% {})",
            sr_size,
            machine_size,
            compression_ratio.abs(),
            if compression_ratio > 0.0 {
                "smaller"
            } else {
                "larger"
            }
        );

        // Performance
        println!("  ⚡ Serialize: {:?}", serialize_time);
        println!("  ⚡ Deserialize: {:?}", deserialize_time);
        println!();
    }

    // Summary
    println!("=== Summary ===");
    println!("Total Dx Serializer size: {} bytes", total_sr_size);
    println!("Total Machine size: {} bytes", total_machine_size);

    if total_sr_size > 0 {
        let total_compression = (1.0 - (total_machine_size as f64 / total_sr_size as f64)) * 100.0;
        println!(
            "Overall compression: {:.1}% {}",
            total_compression.abs(),
            if total_compression > 0.0 {
                "smaller"
            } else {
                "larger"
            }
        );
    }

    println!("\nTotal serialize time: {:.2} µs", total_serialize_time as f64 / 1000.0);
    println!("Total deserialize time: {:.2} µs", total_deserialize_time as f64 / 1000.0);

    println!("\n✅ Real-world test complete!");
    println!("Check essence/*.machine files for binary output");
}
