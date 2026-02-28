//! Basic Usage Examples for dx-style
//!
//! This example demonstrates common usage patterns for the dx-style crate,
//! including:
//! - Binary Dawn format for CSS storage
//! - HTML class extraction
//! - Auto-grouping configuration
//!
//! Run with: `cargo run --example basic_usage`

use style::binary::dawn::{BinaryDawnReader, BinaryDawnWriter};
use style::parser::extract_classes_fast;

fn main() {
    println!("=== dx-style Basic Usage Examples ===\n");

    // Example 1: Binary Dawn Format
    example_binary_dawn();

    // Example 2: HTML Class Extraction
    example_class_extraction();

    // Example 3: Auto-Grouping
    example_auto_grouping();

    println!("\n✅ All examples completed successfully!");
}

/// Example 1: Using Binary Dawn format for CSS storage
///
/// Binary Dawn is a zero-copy binary format for ultra-fast CSS loading.
/// It's ideal for production deployments where startup time matters.
fn example_binary_dawn() {
    println!("📦 Example 1: Binary Dawn Format");
    println!("─────────────────────────────────────");

    // Create a writer and add CSS rules
    let mut writer = BinaryDawnWriter::new();
    writer.add_style(1, ".flex { display: flex; }");
    writer.add_style(2, ".items-center { align-items: center; }");
    writer.add_style(3, ".p-4 { padding: 1rem; }");
    writer.add_style(4, ".bg-blue-500 { background-color: rgb(59, 130, 246); }");

    // Build the binary format
    let binary_data = writer.build();
    println!("Binary data size: {} bytes", binary_data.len());

    // Read back with zero-copy
    let reader = BinaryDawnReader::new(&binary_data).expect("Failed to parse Binary Dawn");
    println!("Entry count: {}", reader.entry_count());

    // Lookup CSS by ID (O(log n) binary search)
    for id in 1..=4 {
        if let Some(css) = reader.get_css(id) {
            println!("  ID {} → {}", id, css);
        }
    }

    // Non-existent ID returns None
    assert!(reader.get_css(999).is_none());
    println!("  ID 999 → None (not found)\n");
}

/// Example 2: Extracting CSS classes from HTML
///
/// The parser efficiently extracts class names from HTML content,
/// supporting both `class` and `className` attributes.
fn example_class_extraction() {
    println!("🔍 Example 2: HTML Class Extraction");
    println!("─────────────────────────────────────");

    let html = r#"
        <div class="flex items-center justify-between p-4">
            <span class="text-lg font-bold">Title</span>
            <button class="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded">
                Click me
            </button>
        </div>
    "#;

    let html_bytes = html.as_bytes();
    let extracted = extract_classes_fast(html_bytes, 128);

    println!("Extracted {} unique classes:", extracted.classes.len());
    let mut classes: Vec<_> = extracted.classes.iter().collect();
    classes.sort();
    for class in classes {
        println!("  - {}", class);
    }
    println!();
}

/// Example 3: Auto-Grouping Configuration
///
/// Auto-grouping detects frequently co-occurring class combinations
/// and groups them into single classnames for smaller HTML output.
fn example_auto_grouping() {
    use ahash::AHashSet;
    use style::grouping::{AutoGroupConfig, AutoGrouper};

    println!("🔄 Example 3: Auto-Grouping");
    println!("─────────────────────────────────────");

    // Configure auto-grouping
    let config = AutoGroupConfig {
        enabled: true,
        min_occurrences: 2,        // Group patterns that appear 2+ times
        similarity_threshold: 0.6, // 60% similarity threshold
        auto_rewrite: true,        // Automatically rewrite HTML
        excluded_patterns: vec![
            // Don't group these patterns
            "hover:".to_string(),
            "focus:".to_string(),
        ],
    };

    println!("Configuration:");
    println!("  enabled: {}", config.enabled);
    println!("  min_occurrences: {}", config.min_occurrences);
    println!("  similarity_threshold: {}", config.similarity_threshold);
    println!("  excluded_patterns: {:?}", config.excluded_patterns);

    // Create grouper with no existing classes
    let mut grouper = AutoGrouper::new(config, AHashSet::new());

    // HTML with repeated patterns
    let html = br#"
        <div class="flex items-center p-4">Item 1</div>
        <div class="flex items-center p-4">Item 2</div>
        <div class="flex items-center p-4">Item 3</div>
    "#;

    // Process HTML
    let result = grouper.process(html);

    match result {
        Some(rewrite) => {
            println!("\nDetected {} group(s):", rewrite.groups.len());
            for group in &rewrite.groups {
                println!("  {} → {:?}", group.alias, group.classes);
            }
        }
        None => {
            println!("\nNo patterns detected (threshold not met)");
        }
    }
    println!();
}
