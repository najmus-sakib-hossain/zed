use dx_markdown::convert::llm_to_machine;
use dx_markdown::{CompilerConfig, DxMarkdown};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use walkdir::WalkDir;

fn main() {
    println!("=== Generating .human and .machine files with directory structure ===\n");

    let dx_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    println!("Workspace root: {}\n", dx_root.display());

    let config = CompilerConfig {
        strip_urls: true,
        strip_images: true,
        strip_badges: true,
        tables_to_tsv: true,
        minify_code: false,
        collapse_whitespace: true,
        strip_filler: true,
        dictionary: false,
        ..CompilerConfig::default()
    };

    let start_time = Instant::now();
    let mut success = 0;
    let mut errors = 0;

    // Find all .md files
    println!("Scanning for markdown files...");
    let md_files: Vec<PathBuf> = WalkDir::new(&dx_root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
        .filter(|e| {
            let path_str = e.path().to_string_lossy();
            !path_str.contains("node_modules")
                && !path_str.contains("/target/")
                && !path_str.contains("\\.git\\")
                && !path_str.contains("/.git/")
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    println!("Found {} markdown files\n", md_files.len());

    for (idx, path) in md_files.iter().enumerate() {
        if (idx + 1) % 100 == 0 {
            println!("Progress: {}/{} files...", idx + 1, md_files.len());
        }

        match process_file(&path, &dx_root, &config) {
            Ok(_) => success += 1,
            Err(e) => {
                errors += 1;
                if errors <= 10 {
                    eprintln!("Error processing {}: {}", path.display(), e);
                }
            }
        }
    }

    let elapsed = start_time.elapsed();
    println!("\n=== Complete ===");
    println!("Success: {}", success);
    println!("Errors: {}", errors);
    println!("Time: {:?}", elapsed);
}

fn process_file(
    path: &Path,
    workspace_root: &Path,
    config: &CompilerConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get relative path from workspace root
    let relative_path = path.strip_prefix(workspace_root)?;

    // Create output path maintaining directory structure
    let output_base = workspace_root.join(".dx").join("markdown");
    let output_dir = if let Some(parent) = relative_path.parent() {
        output_base.join(parent)
    } else {
        output_base.clone()
    };

    // Create directory structure
    fs::create_dir_all(&output_dir)?;

    // Get filename without extension
    let file_stem = path.file_stem().unwrap().to_string_lossy();

    // Read original content
    let original_content = fs::read_to_string(path)?;

    // Compile to LLM format
    let compiler = DxMarkdown::new(config.clone())?;
    let result = compiler.compile(&original_content)?;

    // Write .human file
    let human_path = output_dir.join(format!("{}.human", file_stem));
    fs::write(&human_path, &original_content)?;

    // Write .machine file
    let machine_path = output_dir.join(format!("{}.machine", file_stem));
    let machine_bytes = llm_to_machine(&result.output)?;
    fs::write(&machine_path, &machine_bytes)?;

    Ok(())
}
