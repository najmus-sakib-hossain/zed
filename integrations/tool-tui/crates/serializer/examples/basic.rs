//! Basic DX Serializer Usage
//!
//! This example demonstrates the fundamental serialize/deserialize operations
//! using the simplified public API.
//!
//! Run with: `cargo run --example basic`

use serializer::{
    DxDocument, DxLlmValue, DxValue, deserialize, encode, format_human, parse, serialize,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== DX Serializer: Basic Usage ===\n");

    // =========================================================================
    // Part 1: Simplified API (DxDocument)
    // =========================================================================
    println!("--- Part 1: Simplified API (DxDocument) ---\n");

    // Create a document with the simplified API
    println!("1. Creating a DxDocument");
    let mut doc = DxDocument::new();
    doc.context.insert("name".to_string(), DxLlmValue::Str("MyApp".to_string()));
    doc.context.insert("version".to_string(), DxLlmValue::Str("1.0.0".to_string()));
    doc.context.insert("count".to_string(), DxLlmValue::Num(42.0));
    doc.context.insert("active".to_string(), DxLlmValue::Bool(true));
    println!("   Created document with {} fields", doc.context.len());

    // Serialize to LLM format (text)
    println!("\n2. Serializing to LLM format");
    let text = serialize(&doc);
    println!("   Output ({} bytes):", text.len());
    for line in text.lines() {
        println!("   {}", line);
    }

    // Deserialize back
    println!("\n3. Deserializing from LLM format");
    let parsed = deserialize(&text)?;
    println!("   Parsed {} fields:", parsed.context.len());
    for (key, value) in &parsed.context {
        println!("   {} = {:?}", key, value);
    }

    // =========================================================================
    // Part 2: Low-Level API (DxValue)
    // =========================================================================
    println!("\n--- Part 2: Low-Level API (DxValue) ---\n");

    // Parse simple key-value pairs
    println!("4. Parsing key-value pairs");
    let input = b"name:Alice\nage:30\nactive:+";
    let data = parse(input)?;

    if let DxValue::Object(obj) = &data {
        println!("   Name: {}", obj.get("name").unwrap().as_str().unwrap());
        println!("   Age: {}", obj.get("age").unwrap().as_int().unwrap());
        println!("   Active: {}", obj.get("active").unwrap().as_bool().unwrap());
    }

    // Parse arrays with stream operator
    println!("\n5. Parsing arrays");
    let input = b"colors>red|blue|green|yellow";
    let data = parse(input)?;
    if let DxValue::Object(obj) = &data {
        if let Some(DxValue::Array(arr)) = obj.get("colors") {
            print!("   Colors: ");
            for elem in &arr.values {
                print!("{} ", elem.as_str().unwrap());
            }
            println!();
        }
    }

    // Encode data back to DX format
    println!("\n6. Encoding data");
    let input = b"project:DX\ncount:42\nstatus:+";
    let data = parse(input)?;
    let encoded = encode(&data)?;
    println!("   Original: {} bytes", input.len());
    println!("   Encoded:  {} bytes", encoded.len());
    println!("   Content:  {}", String::from_utf8_lossy(&encoded));

    // Human-readable formatting
    println!("\n7. Human-readable formatting");
    let human = format_human(&data)?;
    println!("{}", human);

    // =========================================================================
    // Part 3: Tables
    // =========================================================================
    println!("--- Part 3: Tables ---\n");

    println!("8. Parsing tables with schema");
    let table_input = b"users=id%i name%s score%f active%b
1 Alice 95.5 +
2 Bob 87.3 -
3 Charlie 92.0 +";

    let table_data = parse(table_input)?;
    let table_human = format_human(&table_data)?;
    println!("{}", table_human);

    // Round-trip verification
    println!("9. Round-trip verification");
    let encoded = encode(&table_data)?;
    let reparsed = parse(&encoded)?;
    println!("   Round-trip successful: {}", table_data == reparsed);

    println!("\n=== Example Complete ===");
    Ok(())
}
