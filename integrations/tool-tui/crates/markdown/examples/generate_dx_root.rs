use dx_markdown::convert::llm_to_machine;
use dx_markdown::{CompilerConfig, DxMarkdown};
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("DX Markdown: Generating .human and .machine files for root markdown files...\n");

    // Get dx workspace root (CARGO_MANIFEST_DIR is crates/markdown, go up 2 levels)
    let dx_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    println!("Processing: {}\n", dx_root.display());

    // Test if we can read a simple file first
    let test_path = dx_root.join("README.md");
    if test_path.exists() {
        println!("README.md exists, size: {} bytes", std::fs::metadata(&test_path).unwrap().len());
    } else {
        println!("README.md not found!");
        return;
    }

    // Configuration for LLM format compilation
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

    let mut total_files = 0;
    let mut success_count = 0;
    let mut total_tokens_before = 0;
    let mut total_tokens_after = 0;

    // Process only root-level .md files
    let root_files = [
        "README.md",
        "HUMAN_FORMAT.md",
        "LLM_FORMAT.md",
        "MACHINE_FORMAT.md",
        "MARKDOWN.md",
        "MARKDOWN_NOISE.md",
        "MARKDOWN_VISUAL.md",
        "ZED.md",
    ];

    for filename in &root_files {
        let path = dx_root.join(filename);
        if !path.exists() {
            continue;
        }

        total_files += 1;

        println!("Processing {}...", filename);

        match process_markdown_file(&path, &dx_root, &config) {
            Ok((tokens_before, tokens_after)) => {
                success_count += 1;
                total_tokens_before += tokens_before;
                total_tokens_after += tokens_after;
            }
            Err(e) => {
                eprintln!("Error processing {}: {}", filename, e);
            }
        }
    }

    println!("\n=== Token Savings Summary ===");
    println!("Files processed: {}/{}", success_count, total_files);
    println!("Tokens before: {}", total_tokens_before);
    println!("Tokens after: {}", total_tokens_after);

    if total_tokens_before > 0 {
        let saved = total_tokens_before - total_tokens_after;
        let savings_pct = (saved as f64 / total_tokens_before as f64) * 100.0;
        println!("Tokens saved: {} ({:.1}%)", saved, savings_pct);
        println!("\n🎉 DX Markdown saved {} tokens across {} files!", saved, success_count);
    }
}

fn process_markdown_file(
    path: &Path,
    workspace_root: &Path,
    config: &CompilerConfig,
) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    // Read original markdown file (human-readable format)
    let original_content = fs::read_to_string(path)?;

    // Compile to LLM format (token-optimized)
    let compiler = DxMarkdown::new(config.clone())?;
    let result = compiler.compile(&original_content)?;

    // Get filename without extension
    let file_stem = path.file_stem().unwrap().to_string_lossy();

    // Create output directory: .dx/markdown/
    let output_dir = workspace_root.join(".dx").join("markdown");
    fs::create_dir_all(&output_dir)?;

    // 1. Write .human file (beautiful human-readable format - original content)
    let human_path = output_dir.join(format!("{}.human", file_stem));
    fs::write(&human_path, &original_content)?;

    // 2. Write .machine file (binary format - using rkyv for fastest serialization)
    let machine_path = output_dir.join(format!("{}.machine", file_stem));
    let machine_bytes = llm_to_machine(&result.output)?;
    fs::write(&machine_path, machine_bytes)?;

    // 3. Overwrite the original .md file with LLM-optimized format
    fs::write(path, &result.output)?;

    println!(
        "✓ {} → {} tokens (saved {:.1}%)",
        path.file_name().unwrap().to_string_lossy(),
        result.tokens_after,
        result.savings_percent()
    );

    Ok((result.tokens_before, result.tokens_after))
}
