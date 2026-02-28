use std::fs;

fn main() {
    println!("Parsing crates/check/README.md with HumanParser\n");

    let content = fs::read_to_string("../../crates/check/README.md").expect("Failed to read file");

    println!("File size: {} bytes\n", content.len());

    // Use the internal parser directly
    use dx_markdown::convert::human_to_llm;

    println!("Attempting human_to_llm conversion...");
    match human_to_llm(&content) {
        Ok(llm) => {
            println!("✓ Parsed successfully!");
            println!("LLM format size: {} bytes", llm.len());

            // Now try to serialize to machine
            println!("\nAttempting llm_to_machine conversion...");
            use dx_markdown::convert::llm_to_machine;
            match llm_to_machine(&llm) {
                Ok(machine) => {
                    println!("✓ Serialized successfully!");
                    println!("Machine format size: {} bytes", machine.len());
                }
                Err(e) => {
                    println!("✗ Serialization failed: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("✗ Parsing failed: {:?}", e);
        }
    }
}
