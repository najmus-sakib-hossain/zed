use dx_markdown::convert::{llm_to_human, llm_to_machine};
use dx_markdown::{CompilerConfig, DxMarkdown};
use std::env;
use std::fs;
use walkdir::WalkDir;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("⚠ Markdown .llm and .machine file generation is currently disabled.");
    println!("To re-enable, uncomment the code in crates/markdown/examples/generate_dx_markdown.rs");
    return Ok(());

    // COMMENTED OUT - Markdown generation disabled
    /*
    // Get workspace root (go up from current dir if we're in target)
    let current_dir = env::current_dir()?;
    let workspace_root =
        if current_dir.ends_with("target/debug") || current_dir.ends_with("target\\debug") {
            current_dir.parent().unwrap().parent().unwrap()
        } else {
            &current_dir
        };

    println!("Workspace root: {}\n", workspace_root.display());

    // Create .dx/markdown directory
    let dx_dir = workspace_root.join(".dx/markdown");
    fs::create_dir_all(&dx_dir)?;

    // Find all markdown files in the workspace
    let mut markdown_files = Vec::new();

    for entry in WalkDir::new(workspace_root).follow_links(false).into_iter().filter_entry(|e| {
        let path = e.path();
        // Skip hidden directories, target, node_modules, .git
        !path.components().any(|c| {
            let s = c.as_os_str().to_string_lossy();
            s.starts_with('.') || s == "target" || s == "node_modules"
        })
    }) {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "md" {
                    markdown_files.push(path.to_path_buf());
                }
            }
        }
    }

    println!("Found {} markdown files\n", markdown_files.len());

    // Create compiler with optimal settings
    let config = CompilerConfig {
        tables_to_tsv: true,
        strip_urls: true,
        strip_images: true,
        strip_badges: true,
        minify_code: false,
        collapse_whitespace: true,
        strip_filler: true,
        dictionary: false,
        ..Default::default()
    };

    let compiler = DxMarkdown::new(config)?;

    let mut total_tokens_before = 0;
    let mut total_tokens_after = 0;
    let mut processed = 0;
    let mut failed = 0;

    for md_file in &markdown_files {
        // Read the markdown file
        let content = match fs::read_to_string(md_file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("✗ Failed to read {}: {}", md_file.display(), e);
                failed += 1;
                continue;
            }
        };

        // Skip empty files
        if content.trim().is_empty() {
            continue;
        }

        // Compile to LLM format
        let result = match compiler.compile(&content) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("✗ Failed to compile {}: {}", md_file.display(), e);
                failed += 1;
                continue;
            }
        };

        // Generate output filename (remove .md extension)
        let relative_path = md_file.strip_prefix(workspace_root).unwrap_or(md_file);
        let filename = relative_path
            .to_string_lossy()
            .replace('/', "_")
            .replace('\\', "_")
            .trim_end_matches(".md")
            .to_string();

        // Save LLM format (overwrite original .md file)
        if let Err(e) = fs::write(md_file, &result.output) {
            eprintln!("✗ Failed to write LLM format to {}: {}", md_file.display(), e);
            failed += 1;
            continue;
        }

        // Convert to Human format
        let human_content = match llm_to_human(&result.output) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("✗ Failed to convert to human format {}: {}", md_file.display(), e);
                failed += 1;
                continue;
            }
        };

        // Save Human format to .dx/markdown/*.human
        let human_path = dx_dir.join(format!("{}.human", filename));
        if let Err(e) = fs::write(&human_path, &human_content) {
            eprintln!("✗ Failed to write Human format to {}: {}", human_path.display(), e);
            failed += 1;
            continue;
        }

        // Convert to Machine format (binary)
        let machine_content = match llm_to_machine(&result.output) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("✗ Failed to convert to machine format {}: {}", md_file.display(), e);
                failed += 1;
                continue;
            }
        };

        // Save Machine format to .dx/markdown/*.machine
        let machine_path = dx_dir.join(format!("{}.machine", filename));
        if let Err(e) = fs::write(&machine_path, &machine_content) {
            eprintln!("✗ Failed to write Machine format to {}: {}", machine_path.display(), e);
            failed += 1;
            continue;
        }

        total_tokens_before += result.tokens_before;
        total_tokens_after += result.tokens_after;
        processed += 1;

        let savings = result.savings_percent();
        println!(
            "✓ {} ({} → {} tokens, {:.1}% saved)",
            relative_path.display(),
            result.tokens_before,
            result.tokens_after,
            savings
        );
    }

    println!("\n=== SUMMARY ===");
    println!("Processed: {} files", processed);
    println!("Failed: {} files", failed);
    println!("Total tokens before: {}", total_tokens_before);
    println!("Total tokens after: {}", total_tokens_after);

    if total_tokens_before > 0 {
        let total_savings = ((total_tokens_before - total_tokens_after) as f64
            / total_tokens_before as f64)
            * 100.0;
        println!("Total savings: {:.1}%", total_savings);
    }

    println!("\n✓ All markdown files processed!");
    println!("✓ LLM format saved to original .md files");
    println!("✓ Human format saved to .dx/markdown/*.human");
    println!("✓ Machine format (binary) saved to .dx/markdown/*.machine");

    Ok(())
    */
}
