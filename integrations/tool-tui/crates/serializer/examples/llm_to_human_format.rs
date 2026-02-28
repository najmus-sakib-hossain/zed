use std::fs;
use std::path::Path;

// Run with: cargo run -p dx-serializer --example llm_to_human_format

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use serializer::llm::llm_to_human;

    println!("Converting LLM files to human format...\n");

    // Process root dx file
    let dx_llm = Path::new(".dx/serializer/dx.llm");
    if dx_llm.exists() {
        let content = fs::read_to_string(dx_llm)?;
        println!("LLM content (first 100 chars): {}", &content[..100.min(content.len())]);
        let human_content = llm_to_human(&content)?;
        println!(
            "Human content (first 200 chars): {}",
            &human_content[..200.min(human_content.len())]
        );
        fs::write("dx", human_content)?;
        println!("Converted: dx");
    }

    // Process essence/*.llm files
    let essence_llm_dir = Path::new(".dx/serializer/essence");
    if essence_llm_dir.exists() {
        for entry in fs::read_dir(essence_llm_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("llm") {
                let content = fs::read_to_string(&path)?;
                let human_content = llm_to_human(&content)?;

                let stem = path.file_stem().unwrap().to_str().unwrap();
                let output_path = Path::new("essence").join(format!("{}.sr", stem));
                fs::write(&output_path, human_content)?;
                println!("Converted: {}", output_path.display());
            }
        }
    }

    println!("\nDone!");
    Ok(())
}
