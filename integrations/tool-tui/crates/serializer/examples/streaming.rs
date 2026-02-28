//! Streaming and Large File Handling Example
//!
//! This example demonstrates how to handle large files efficiently using
//! the dx-serializer's streaming capabilities.
//!
//! Run with: `cargo run --example streaming`

use serializer::{
    DxDocument, DxLlmValue, DxObject, DxValue, encode_to_writer, parse, parse_stream,
};
use std::fs::File;
use std::io::{BufReader, BufWriter, Cursor, Write};
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== DX Serializer: Streaming & Large File Handling ===\n");

    // =========================================================================
    // Part 1: Stream Parsing from Reader
    // =========================================================================
    println!("--- Part 1: Stream Parsing from Reader ---\n");

    // Simulate reading from a file-like source
    let data = b"name:StreamTest\ncount:1000\nactive:+";
    let cursor = Cursor::new(data.as_slice());

    println!("1. Parsing from a reader (simulated file)");
    let start = Instant::now();
    let parsed = parse_stream(cursor)?;
    let elapsed = start.elapsed();

    if let DxValue::Object(obj) = &parsed {
        println!("   Parsed {} fields in {:?}", obj.fields.len(), elapsed);
        for (key, value) in obj.iter() {
            println!("   {} = {:?}", key, value);
        }
    }

    // =========================================================================
    // Part 2: Streaming Write to Writer
    // =========================================================================
    println!("\n--- Part 2: Streaming Write to Writer ---\n");

    // Create data to encode
    let mut obj = DxObject::new();
    obj.insert("project".to_string(), DxValue::String("LargeData".to_string()));
    obj.insert("records".to_string(), DxValue::Int(50000));
    obj.insert("compressed".to_string(), DxValue::Bool(true));
    let value = DxValue::Object(obj);

    // Write to an in-memory buffer (simulating file write)
    let mut buffer = Vec::new();
    println!("2. Encoding to a writer (streaming output)");
    let start = Instant::now();
    encode_to_writer(&value, &mut buffer)?;
    let elapsed = start.elapsed();

    println!("   Wrote {} bytes in {:?}", buffer.len(), elapsed);
    println!("   Content: {}", String::from_utf8_lossy(&buffer));

    // =========================================================================
    // Part 3: Processing Large Datasets in Chunks
    // =========================================================================
    println!("\n--- Part 3: Processing Large Datasets ---\n");

    println!("3. Generating and processing large key-value data");

    // Generate a large key-value dataset
    let item_count = 5_000;
    let start = Instant::now();

    // Build data as DX format string (key:value pairs)
    let mut kv_data = String::new();
    for i in 0..item_count {
        kv_data.push_str(&format!("item{}:{}\n", i, i * 10));
    }

    let gen_elapsed = start.elapsed();
    println!(
        "   Generated {} key-value pairs ({} bytes) in {:?}",
        item_count,
        kv_data.len(),
        gen_elapsed
    );

    // Parse the large dataset
    let start = Instant::now();
    let parsed = parse(kv_data.as_bytes())?;
    let parse_elapsed = start.elapsed();

    if let DxValue::Object(obj) = &parsed {
        println!("   Parsed {} fields in {:?}", obj.fields.len(), parse_elapsed);
        println!(
            "   Throughput: {:.2} MB/s",
            (kv_data.len() as f64 / 1_000_000.0) / parse_elapsed.as_secs_f64()
        );

        // Verify a sample
        if let Some(DxValue::Int(val)) = obj.get("item100") {
            println!("   Sample verification: item100 = {} (expected 1000)", val);
        }
    }

    // =========================================================================
    // Part 4: Memory-Efficient Document Building
    // =========================================================================
    println!("\n--- Part 4: Memory-Efficient Document Building ---\n");

    println!("4. Building large DxDocument incrementally");

    let start = Instant::now();
    let mut doc = DxDocument::new();

    // Add metadata
    doc.context.insert("type".to_string(), DxLlmValue::Str("batch".to_string()));
    doc.context.insert("version".to_string(), DxLlmValue::Str("1.0".to_string()));

    // Add many items incrementally
    let item_count = 1000;
    for i in 0..item_count {
        let key = format!("item_{}", i);
        doc.context.insert(key, DxLlmValue::Num(i as f64 * 1.5));
    }

    let build_elapsed = start.elapsed();
    println!("   Built document with {} fields in {:?}", doc.context.len(), build_elapsed);

    // Serialize the large document
    let start = Instant::now();
    let serialized = serializer::serialize(&doc);
    let serialize_elapsed = start.elapsed();

    println!("   Serialized to {} bytes in {:?}", serialized.len(), serialize_elapsed);

    // =========================================================================
    // Part 5: Buffered File I/O Pattern
    // =========================================================================
    println!("\n--- Part 5: Buffered File I/O Pattern ---\n");

    println!("5. Demonstrating buffered I/O pattern (in-memory simulation)");

    // Create sample data
    let mut data = DxObject::new();
    data.insert("config".to_string(), DxValue::String("production".to_string()));
    data.insert("workers".to_string(), DxValue::Int(8));
    data.insert("timeout".to_string(), DxValue::Int(30000));
    let value = DxValue::Object(data);

    // Write with buffering (simulated with Vec)
    let mut output_buffer: Vec<u8> = Vec::with_capacity(4096);
    {
        let mut writer = BufWriter::new(&mut output_buffer);
        encode_to_writer(&value, &mut writer)?;
        writer.flush()?;
    }

    println!("   Buffered write: {} bytes", output_buffer.len());

    // Read with buffering (simulated)
    let reader = BufReader::new(Cursor::new(&output_buffer));
    let reparsed = parse_stream(reader)?;

    println!("   Buffered read: round-trip successful = {}", value == reparsed);

    // =========================================================================
    // Part 6: Streaming with Real Files (Optional)
    // =========================================================================
    println!("\n--- Part 6: File I/O Example ---\n");

    // Create a temporary file path
    let temp_path = std::env::temp_dir().join("dx_streaming_example.dx");

    println!("6. Writing to and reading from file: {:?}", temp_path);

    // Write to file
    {
        let file = File::create(&temp_path)?;
        let mut writer = BufWriter::new(file);

        let mut obj = DxObject::new();
        obj.insert("source".to_string(), DxValue::String("file_example".to_string()));
        obj.insert("timestamp".to_string(), DxValue::Int(1234567890));
        let value = DxValue::Object(obj);

        encode_to_writer(&value, &mut writer)?;
        writer.flush()?;
    }

    // Read from file
    {
        let file = File::open(&temp_path)?;
        let reader = BufReader::new(file);
        let parsed = parse_stream(reader)?;

        if let DxValue::Object(obj) = parsed {
            println!("   Read {} fields from file", obj.fields.len());
            for (key, value) in obj.iter() {
                println!("   {} = {:?}", key, value);
            }
        }
    }

    // Clean up
    std::fs::remove_file(&temp_path)?;
    println!("   Cleaned up temporary file");

    // =========================================================================
    // Summary
    // =========================================================================
    println!("\n--- Summary: Streaming Best Practices ---\n");

    println!("Key patterns for large file handling:");
    println!("  1. Use parse_stream() for reading from any std::io::Read source");
    println!("  2. Use encode_to_writer() for streaming output to any std::io::Write");
    println!("  3. Wrap file handles with BufReader/BufWriter for better performance");
    println!("  4. Build DxDocument incrementally to manage memory");
    println!("  5. Process tables in chunks when dealing with millions of rows");

    println!("\nPerformance tips:");
    println!("  - DX format is designed for fast parsing (minimal allocations)");
    println!("  - Tables with schemas enable columnar access patterns");
    println!("  - Use Base62 encoding (%x) for compact integer representation");
    println!("  - Security limits prevent DoS: 100MB max input, 1000 recursion depth");

    println!("\n=== Example Complete ===");
    Ok(())
}
