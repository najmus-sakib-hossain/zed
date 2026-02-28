use std::fs;

fn main() {
    let path = "../../crates/driven/inspirations/BMAD-METHOD/src/modules/bmb/workflows/create-agent/data/reference/agents/expert-examples/journal-keeper/journal-keeper-sidecar/breakthroughs.md";

    println!("Reading file: {}", path);
    let content = fs::read_to_string(path).expect("Failed to read file");
    println!("File size: {} bytes", content.len());

    // Normalize line endings
    let content = content.replace("\r\n", "\n").replace('\r', "\n");
    println!("After normalization: {} bytes", content.len());

    println!("\nConverting to LLM format...");
    use dx_markdown::convert::human_to_llm;
    match human_to_llm(&content) {
        Ok(llm) => {
            println!("✓ LLM conversion successful: {} bytes", llm.len());

            // Save LLM output for inspection
            fs::write("debug_breakthroughs.llm", &llm).expect("Failed to write LLM");
            println!("Saved to debug_breakthroughs.llm");

            println!("\nParsing LLM back to document...");
            println!("LLM input length: {}", llm.len());
            println!("LLM first 100 chars: {}", &llm[..llm.len().min(100)]);

            use dx_markdown::parser::DxmParser;
            println!("About to call DxmParser::parse...");
            match DxmParser::parse(&llm) {
                Ok(doc) => {
                    println!("✓ Parse successful: {} nodes", doc.nodes.len());
                    println!("Document has {} refs", doc.refs.len());

                    // Print node types
                    for (i, node) in doc.nodes.iter().enumerate() {
                        println!("Node {}: {:?}", i, std::mem::discriminant(node));
                    }

                    println!("\nBuilding binary...");
                    use dx_markdown::binary::BinaryBuilder;
                    match BinaryBuilder::build(&doc) {
                        Ok(binary) => {
                            println!("✓ Binary build successful: {} bytes", binary.len());
                        }
                        Err(e) => {
                            eprintln!("✗ Binary build failed: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("✗ Parse failed: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("✗ LLM conversion failed: {}", e);
        }
    }
}
