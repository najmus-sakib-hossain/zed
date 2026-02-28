//! Regenerate .human files from .sr files in essence folder

use serializer::llm::{HumanFormatter, LlmParser};
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let essence_files = vec![
        "../../essence/example1_mixed.sr",
        "../../essence/example2_nested.sr",
        "../../essence/example3_deep.sr",
        "../../essence/example4_config.sr",
        "../../essence/example5_leaf.sr",
    ];

    for sr_file in essence_files {
        println!("Processing: {}", sr_file);

        // Read the .sr file
        let content = fs::read_to_string(sr_file)?;

        // Parse it
        let doc = match LlmParser::parse(&content) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("  ❌ Failed to parse: {}", e);
                continue;
            }
        };

        // Format as human
        let formatter = HumanFormatter::new();
        let human_content = formatter.format(&doc);

        // Determine output path
        let filename = Path::new(sr_file).file_stem().unwrap().to_str().unwrap();
        let output_path = format!("../../.dx/serializer/essence/{}.human", filename);

        // Create directory if needed
        fs::create_dir_all("../../.dx/serializer/essence")?;

        // Write the .human file
        fs::write(&output_path, human_content)?;

        println!("  ✅ Generated: {}", output_path);
    }

    println!("\n✅ All .human files regenerated!");
    Ok(())
}
