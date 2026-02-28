use std::fs;

fn main() {
    println!("Debugging crates/check/README.md\n");

    let content = fs::read_to_string("../../crates/check/README.md").expect("Failed to read file");

    println!("File size: {} bytes", content.len());
    println!("Lines: {}", content.lines().count());
    println!("Characters: {}", content.chars().count());

    // Check for problematic patterns
    let has_nested_lists = content.matches("  -").count();
    let has_tables = content.matches('|').count();
    let has_code_blocks = content.matches("```").count();

    println!("\nContent analysis:");
    println!("  Nested list items: {}", has_nested_lists);
    println!("  Table cells (|): {}", has_tables);
    println!("  Code blocks: {}", has_code_blocks / 2);

    // Try to parse with HumanParser
    println!("\nAttempting to parse with human_to_machine...");

    use dx_markdown::convert::human_to_machine;

    match human_to_machine(&content) {
        Ok(bytes) => {
            println!("✓ Success! Generated {} bytes", bytes.len());
        }
        Err(e) => {
            println!("✗ Failed: {:?}", e);
        }
    }
}
