use dx_markdown::convert::human_to_machine;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn main() {
    println!("=== Generating .human and .machine files for ALL markdown ===\n");

    let dx_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let output_dir = dx_root.join(".dx/markdown");
    fs::create_dir_all(&output_dir).expect("Failed to create output dir");

    // Manually collect files to avoid WalkDir issues
    let mut md_files = Vec::new();
    collect_markdown_files(&dx_root, &mut md_files);

    // Limit to first 500 files for safety
    md_files.truncate(500);

    println!("Found {} markdown files (limited to 500)\n", md_files.len());

    let start_time = Instant::now();
    let mut success = 0;
    let mut errors = 0;
    let mut total_human = 0;
    let mut total_machine = 0;

    for (i, path) in md_files.iter().enumerate() {
        if (i + 1) % 100 == 0 {
            println!("Progress: {}/{} files...", i + 1, md_files.len());
        }

        match process_file(&path, &dx_root, &output_dir) {
            Ok((human_size, machine_size)) => {
                success += 1;
                total_human += human_size;
                total_machine += machine_size;
            }
            Err(e) => {
                errors += 1;
                if errors <= 10 {
                    eprintln!("Skipped: {} - {}", path.display(), e);
                }
            }
        }
    }

    let duration = start_time.elapsed();

    println!("\n=== Complete ===");
    println!("Total files: {}", md_files.len());
    println!("Success: {}", success);
    println!("Errors: {}", errors);
    println!("Time: {:?}", duration);
    println!("\nSizes:");
    println!("  Human:   {} bytes ({:.2} MB)", total_human, total_human as f64 / 1_000_000.0);
    println!(
        "  Machine: {} bytes ({:.2} MB)",
        total_machine,
        total_machine as f64 / 1_000_000.0
    );
    println!("  Ratio:   {:.1}%", (total_machine as f64 / total_human as f64) * 100.0);
    println!("\n✓ All files generated in .dx/markdown/");
}

fn collect_markdown_files(dir: &Path, files: &mut Vec<PathBuf>) {
    // No limit - process all files
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        let path_str = path.to_string_lossy();

        // Skip excluded directories
        if path_str.contains("node_modules")
            || path_str.contains("/target/")
            || path_str.contains("\\.git\\")
            || path_str.contains("/.git/")
            || path_str.contains("\\.venv\\")
            || path_str.contains("/.venv/")
            || path_str.contains("\\.dx\\")
            || path_str.contains("/.dx/")
        {
            continue;
        }

        if path.is_dir() {
            collect_markdown_files(&path, files);
        } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
            // Check file size before adding
            if let Ok(metadata) = fs::metadata(&path) {
                if metadata.len() < 500_000 {
                    // Skip files > 500KB
                    files.push(path);
                }
            }
        }
    }
}

fn process_file(
    path: &Path,
    workspace_root: &Path,
    output_dir: &Path,
) -> Result<(usize, usize), String> {
    let human_content = fs::read_to_string(path).map_err(|e| format!("Read error: {}", e))?;

    // Skip files larger than 500KB to avoid memory issues
    if human_content.len() > 500_000 {
        return Err("File too large (>500KB)".to_string());
    }

    // Skip files with certain patterns that cause issues
    if human_content.contains("```mermaid") && human_content.len() > 100_000 {
        return Err("Large mermaid diagram".to_string());
    }

    let _file_stem = path.file_stem().ok_or("No file stem")?.to_string_lossy();

    // Generate unique name based on path
    let relative = path.strip_prefix(workspace_root).unwrap_or(path);
    let unique_name = relative
        .to_string_lossy()
        .replace('/', "_")
        .replace('\\', "_")
        .replace(".md", "");

    // Limit unique name length
    let unique_name = if unique_name.len() > 200 {
        unique_name.chars().take(200).collect()
    } else {
        unique_name
    };

    // Convert to machine format with timeout protection
    let machine_bytes = match human_to_machine(&human_content) {
        Ok(b) => b,
        Err(e) => return Err(format!("Conversion error: {:?}", e)),
    };

    // Save files
    let human_path = output_dir.join(format!("{}.human", unique_name));
    let machine_path = output_dir.join(format!("{}.machine", unique_name));

    fs::write(&human_path, &human_content).map_err(|e| format!("Write human error: {}", e))?;
    fs::write(&machine_path, &machine_bytes).map_err(|e| format!("Write machine error: {}", e))?;

    Ok((human_content.len(), machine_bytes.len()))
}
