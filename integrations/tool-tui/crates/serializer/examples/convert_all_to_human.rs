use std::fs;
use std::path::Path;

// Run with: cargo run -p dx-serializer --example convert_all_to_human

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use serializer::llm::llm_to_human;

    println!("Converting all DX files to new architecture...\n");
    println!("Architecture:");
    println!("  - Front-facing files (.md, .sr, dx): Human format");
    println!("  - .dx folder: .llm + .machine files\n");

    // Step 1: Convert DX Serializer files
    println!("=== Converting DX Serializer files ===\n");

    // Convert root dx file
    let dx_llm = Path::new(".dx/serializer/dx.llm");
    if dx_llm.exists() {
        let content = fs::read_to_string(dx_llm)?;
        let human_content = llm_to_human(&content)?;
        fs::write("dx", human_content)?;
        println!("✓ Converted: dx");
    }

    // Convert essence/*.sr files
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
                println!("✓ Converted: {}", output_path.display());
            }
        }
    }

    // Step 2: Delete old .human files from .dx/serializer
    println!("\n=== Cleaning up old .human files ===\n");
    let mut count = 0;

    let serializer_dx_dir = Path::new(".dx/serializer");
    if serializer_dx_dir.exists() {
        delete_human_files_recursive(serializer_dx_dir, &mut count)?;
    }

    println!("✓ Deleted {} .human files", count);

    println!("\n✓ Conversion complete!");
    println!("\nFront-facing files now have human format with proper spacing.");
    println!(".dx folder contains .llm and .machine files.");

    Ok(())
}

fn delete_human_files_recursive(
    dir: &Path,
    count: &mut usize,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            delete_human_files_recursive(&path, count)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("human") {
            fs::remove_file(&path)?;
            *count += 1;
        }
    }
    Ok(())
}
