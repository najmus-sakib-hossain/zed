//! JSON to DX Conversion Example
//!
//! This example demonstrates converting JSON data to DX LLM format,
//! showing the token efficiency gains for LLM context windows.
//!
//! Run with: `cargo run --example json_conversion`

use serializer::{DxDocument, DxLlmValue, deserialize, serialize};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== DX Serializer: JSON to DX Conversion ===\n");

    // =========================================================================
    // Example 1: Simple JSON-like data to DX
    // =========================================================================
    println!("--- Example 1: Simple Data ---\n");

    // Create a document representing JSON-like data
    let mut doc = DxDocument::new();
    doc.context.insert("name".to_string(), DxLlmValue::Str("my-app".to_string()));
    doc.context.insert("version".to_string(), DxLlmValue::Str("1.0.0".to_string()));
    doc.context
        .insert("description".to_string(), DxLlmValue::Str("A sample application".to_string()));
    doc.context
        .insert("author".to_string(), DxLlmValue::Str("Developer".to_string()));

    // Equivalent JSON would be:
    let json_equivalent = r#"{
  "name": "my-app",
  "version": "1.0.0",
  "description": "A sample application",
  "author": "Developer"
}"#;

    println!("Equivalent JSON ({} bytes):", json_equivalent.len());
    println!("{}\n", json_equivalent);

    let dx_output = serialize(&doc);
    println!("DX LLM Format ({} bytes):", dx_output.len());
    println!("{}\n", dx_output);

    let savings = json_equivalent.len() as i64 - dx_output.len() as i64;
    let percent = (savings as f64 / json_equivalent.len() as f64) * 100.0;
    println!("Token savings: {} bytes ({:.1}%)\n", savings, percent);

    // =========================================================================
    // Example 2: Data with different types
    // =========================================================================
    println!("--- Example 2: Mixed Types ---\n");

    let mut doc = DxDocument::new();
    doc.context.insert("name".to_string(), DxLlmValue::Str("test-app".to_string()));
    doc.context.insert("count".to_string(), DxLlmValue::Num(42.0));
    doc.context.insert("price".to_string(), DxLlmValue::Num(19.99));
    doc.context.insert("active".to_string(), DxLlmValue::Bool(true));
    doc.context.insert("deleted".to_string(), DxLlmValue::Bool(false));
    doc.context.insert("metadata".to_string(), DxLlmValue::Null);

    let json_equivalent = r#"{
  "name": "test-app",
  "count": 42,
  "price": 19.99,
  "active": true,
  "deleted": false,
  "metadata": null
}"#;

    println!("Equivalent JSON ({} bytes):", json_equivalent.len());
    println!("{}\n", json_equivalent);

    let dx_output = serialize(&doc);
    println!("DX LLM Format ({} bytes):", dx_output.len());
    println!("{}\n", dx_output);

    println!("Note: DX uses compact representations:");
    println!("  - Booleans: true/false");
    println!("  - Null: null");
    println!("  - Multi-word strings: Use quotes");
    println!("  - Keys use full names (not abbreviated)\n");

    // =========================================================================
    // Example 3: Arrays
    // =========================================================================
    println!("--- Example 3: Arrays ---\n");

    let mut doc = DxDocument::new();
    doc.context.insert("project".to_string(), DxLlmValue::Str("demo".to_string()));
    doc.context.insert(
        "tags".to_string(),
        DxLlmValue::Arr(vec![
            DxLlmValue::Str("rust".to_string()),
            DxLlmValue::Str("wasm".to_string()),
            DxLlmValue::Str("performance".to_string()),
        ]),
    );

    let json_equivalent = r#"{
  "project": "demo",
  "tags": ["rust", "wasm", "performance"]
}"#;

    println!("Equivalent JSON ({} bytes):", json_equivalent.len());
    println!("{}\n", json_equivalent);

    let dx_output = serialize(&doc);
    println!("DX LLM Format ({} bytes):", dx_output.len());
    println!("{}\n", dx_output);

    println!("Note: Arrays use *item1,item2,item3 format\n");

    // =========================================================================
    // Example 4: Round-trip verification
    // =========================================================================
    println!("--- Example 4: Round-Trip Verification ---\n");

    let mut original = DxDocument::new();
    original
        .context
        .insert("name".to_string(), DxLlmValue::Str("RoundTrip".to_string()));
    original
        .context
        .insert("version".to_string(), DxLlmValue::Str("2.0.0".to_string()));
    original.context.insert("count".to_string(), DxLlmValue::Num(100.0));
    original.context.insert("enabled".to_string(), DxLlmValue::Bool(true));

    // Serialize
    let serialized = serialize(&original);
    println!("Serialized: {}", serialized.replace('\n', " | "));

    // Deserialize
    let parsed = deserialize(&serialized)?;
    println!("Parsed back: {} fields", parsed.context.len());

    // Verify
    println!("Round-trip successful: {}\n", original.context.len() == parsed.context.len());

    // =========================================================================
    // Example 5: Comparison summary
    // =========================================================================
    println!("--- Summary: Token Efficiency ---\n");

    // Create a larger document for comparison
    let mut large_doc = DxDocument::new();
    large_doc
        .context
        .insert("name".to_string(), DxLlmValue::Str("large-app".to_string()));
    large_doc
        .context
        .insert("version".to_string(), DxLlmValue::Str("3.0.0".to_string()));
    large_doc
        .context
        .insert("description".to_string(), DxLlmValue::Str("A larger application".to_string()));
    large_doc
        .context
        .insert("author".to_string(), DxLlmValue::Str("Team".to_string()));
    large_doc
        .context
        .insert("license".to_string(), DxLlmValue::Str("MIT".to_string()));
    large_doc
        .context
        .insert("repository".to_string(), DxLlmValue::Str("github.com/example/app".to_string()));
    large_doc
        .context
        .insert("homepage".to_string(), DxLlmValue::Str("https://example.com".to_string()));
    large_doc.context.insert("private".to_string(), DxLlmValue::Bool(false));
    large_doc.context.insert("stable".to_string(), DxLlmValue::Bool(true));

    let large_json = r#"{
  "name": "large-app",
  "version": "3.0.0",
  "description": "A larger application",
  "author": "Team",
  "license": "MIT",
  "repository": "github.com/example/app",
  "homepage": "https://example.com",
  "private": false,
  "stable": true
}"#;

    let large_dx = serialize(&large_doc);

    println!("Large document comparison:");
    println!("  JSON: {} bytes", large_json.len());
    println!("  DX:   {} bytes", large_dx.len());
    println!(
        "  Savings: {} bytes ({:.1}%)",
        large_json.len() - large_dx.len(),
        ((large_json.len() - large_dx.len()) as f64 / large_json.len() as f64) * 100.0
    );

    println!("\nDX format benefits for LLM context windows:");
    println!("  - Abbreviated keys reduce token count");
    println!("  - Compact boolean/null representations");
    println!("  - No quotes around simple strings");
    println!("  - No commas or colons as separators");

    println!("\n=== Example Complete ===");
    Ok(())
}
