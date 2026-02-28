//! Builder Pattern Demo
//!
//! This example demonstrates the SerializerBuilder fluent API
//! for configuring serialization options.

use serializer::{DxDocument, DxLlmValue, DxSection, SerializerBuilder};

fn main() {
    // Create a sample document
    let doc = create_sample_document();

    println!("=== Builder Pattern Demo ===\n");

    // 1. Default configuration
    println!("1. Default Configuration:");
    let default_serializer = SerializerBuilder::new().build();
    let default_output = default_serializer.format_human_unchecked(&doc);
    println!("{}\n", default_output);

    // 2. Human-friendly configuration
    println!("2. Human-Friendly Configuration:");
    let human_serializer = SerializerBuilder::new().for_humans().build();
    let human_output = human_serializer.format_human_unchecked(&doc);
    println!("{}\n", human_output);

    // 3. Compact configuration
    println!("3. Compact Configuration:");
    let compact_serializer = SerializerBuilder::new().for_compact().build();
    let compact_output = compact_serializer.format_human_unchecked(&doc);
    println!("{}\n", compact_output);

    // 4. Table configuration (for multi-row sections)
    println!("4. Table Configuration:");
    let table_serializer = SerializerBuilder::new().for_tables().build();
    let table_output = table_serializer.format_human_unchecked(&doc);
    println!("{}\n", table_output);

    // 5. Custom configuration
    println!("5. Custom Configuration:");
    let custom_serializer = SerializerBuilder::new()
        .indent_size(8)
        .expand_keys(false)
        .space_around_equals(false)
        .use_list_format(false)
        .build();
    let custom_output = custom_serializer.format_human_unchecked(&doc);
    println!("{}\n", custom_output);

    // 6. LLM format (token-efficient)
    println!("6. LLM Format (Token-Efficient):");
    let llm_output = default_serializer.serialize(&doc);
    println!("{}\n", llm_output);

    // 7. Round-trip test
    println!("7. Round-Trip Test:");
    match default_serializer.deserialize(&llm_output) {
        Ok(parsed_doc) => {
            println!("✓ Successfully parsed LLM format");
            println!("  Original context keys: {}", doc.context.len());
            println!("  Parsed context keys: {}", parsed_doc.context.len());
            println!("  Original sections: {}", doc.sections.len());
            println!("  Parsed sections: {}", parsed_doc.sections.len());
        }
        Err(e) => {
            println!("✗ Failed to parse LLM format: {}", e);
        }
    }
}

fn create_sample_document() -> DxDocument {
    let mut doc = DxDocument::new();

    // Add context (configuration)
    doc.context.insert("nm".to_string(), DxLlmValue::Str("MyApp".to_string()));
    doc.context.insert("v".to_string(), DxLlmValue::Str("1.0.0".to_string()));
    doc.context
        .insert("tt".to_string(), DxLlmValue::Str("Sample Application".to_string()));
    doc.context.insert("ac".to_string(), DxLlmValue::Bool(true));
    doc.context.insert("ct".to_string(), DxLlmValue::Num(42.0));

    // Add array
    doc.context.insert(
        "ed".to_string(),
        DxLlmValue::Arr(vec![
            DxLlmValue::Str("neovim".to_string()),
            DxLlmValue::Str("vscode".to_string()),
            DxLlmValue::Str("emacs".to_string()),
        ]),
    );

    // Add workspace array
    doc.context.insert(
        "ws".to_string(),
        DxLlmValue::Arr(vec![
            DxLlmValue::Str("@/frontend".to_string()),
            DxLlmValue::Str("@/backend".to_string()),
            DxLlmValue::Str("@/mobile".to_string()),
        ]),
    );

    // Add a section with multiple rows (like rules)
    let mut rules_section = DxSection::new(vec![
        "id".to_string(),
        "nm".to_string(),
        "ct".to_string(),
        "ac".to_string(),
    ]);

    rules_section.rows.push(vec![
        DxLlmValue::Num(1.0),
        DxLlmValue::Str("no-console".to_string()),
        DxLlmValue::Str("suspicious".to_string()),
        DxLlmValue::Bool(true),
    ]);

    rules_section.rows.push(vec![
        DxLlmValue::Num(2.0),
        DxLlmValue::Str("no-eval".to_string()),
        DxLlmValue::Str("security".to_string()),
        DxLlmValue::Bool(true),
    ]);

    rules_section.rows.push(vec![
        DxLlmValue::Num(3.0),
        DxLlmValue::Str("prefer-const".to_string()),
        DxLlmValue::Str("style".to_string()),
        DxLlmValue::Bool(false),
    ]);

    doc.sections.insert('r', rules_section);

    // Add a single-row section (like forge config)
    let mut forge_section = DxSection::new(vec![
        "repository".to_string(),
        "container".to_string(),
        "tools".to_string(),
    ]);

    forge_section.rows.push(vec![
        DxLlmValue::Str("https://github.com/user/myapp".to_string()),
        DxLlmValue::Null,
        DxLlmValue::Arr(vec![
            DxLlmValue::Str("cli".to_string()),
            DxLlmValue::Str("docs".to_string()),
            DxLlmValue::Str("test".to_string()),
        ]),
    ]);

    doc.sections.insert('f', forge_section);

    doc
}
