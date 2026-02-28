use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use walkdir::WalkDir;

fn main() {
    println!("⚠ Batch markdown conversion is currently disabled.");
    println!("To re-enable, uncomment the code in crates/markdown/examples/batch_convert.rs");
    return;

    // COMMENTED OUT - Batch conversion disabled
    /*
    println!("Finding markdown files...");

    // Find all .md files
    let files: Vec<PathBuf> = WalkDir::new("../../")
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            if !path.extension().map(|e| e == "md").unwrap_or(false) {
                return false;
            }
            let path_str = path.to_string_lossy();
            !path_str.contains("/target/")
                && !path_str.contains("\\target\\")
                && !path_str.contains("/node_modules/")
                && !path_str.contains("\\node_modules\\")
                && !path_str.contains("/.git/")
                && !path_str.contains("\\.git\\")
                && !path_str.contains("/benchmarks/")
                && !path_str.contains("\\benchmarks\\")
                && !path_str.contains("/.venv/")
                && !path_str.contains("\\.venv\\")
                && !path_str.contains("/integrations/")
                && !path_str.contains("\\integrations\\")
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    println!("Found {} markdown files\n", files.len());
    println!("Processing in parallel...\n");

    let processed = Arc::new(AtomicUsize::new(0));
    let skipped = Arc::new(AtomicUsize::new(0));
    let errors = Arc::new(AtomicUsize::new(0));

    files.iter().for_each(|mdfile| {
        // Skip known problematic files
        let path_str = mdfile.to_string_lossy().replace('\\', "/");
        if path_str.contains("copilot-instructions.md") || path_str.contains(".kiro") {
            skipped.fetch_add(1, Ordering::Relaxed);
            return;
        }

        // Print which file we're processing
        print!("Processing {}... ", mdfile.display());
        std::io::Write::flush(&mut std::io::stdout()).ok();

        // Preserve directory structure: ../../crates/driven/README.md -> .dx/markdown/crates/driven/README.{human,machine}
        let path_str = mdfile.to_string_lossy();
        let relative_path = path_str.trim_start_matches("../../").replace('\\', "/"); // Normalize Windows paths

        // Remove .md extension and create paths
        let base_path = relative_path.trim_end_matches(".md");
        let humanfile = PathBuf::from(format!("../../.dx/markdown/{}.human", base_path));
        let machinefile = PathBuf::from(format!("../../.dx/markdown/{}.machine", base_path));

        // Create parent directories
        if let Some(parent) = humanfile.parent() {
            let _ = fs::create_dir_all(parent);
        }

        // Skip if both exist and are newer
        if humanfile.exists() && machinefile.exists() {
            if let (Ok(md_meta), Ok(h_meta), Ok(m_meta)) =
                (fs::metadata(mdfile), fs::metadata(&humanfile), fs::metadata(&machinefile))
            {
                if let (Ok(md_time), Ok(h_time), Ok(m_time)) =
                    (md_meta.modified(), h_meta.modified(), m_meta.modified())
                {
                    if h_time >= md_time && m_time >= md_time {
                        skipped.fetch_add(1, Ordering::Relaxed);
                        return;
                    }
                }
            }
        }

        // Read markdown
        let content = match fs::read_to_string(mdfile) {
            Ok(c) => c,
            Err(_) => {
                println!("✗ {} (read error)", mdfile.display());
                errors.fetch_add(1, Ordering::Relaxed);
                return;
            }
        };

        // Normalize line endings (CRLF -> LF)
        let content = content.replace("\r\n", "\n").replace('\r', "\n");

        // Skip files that are too large (likely to cause issues)
        if content.len() > 500_000 {
            println!("✗ {} (file too large: {} bytes)", mdfile.display(), content.len());
            errors.fetch_add(1, Ordering::Relaxed);
            return;
        }

        // Convert to LLM format
        use dx_markdown::convert::human_to_llm;
        let llm = match human_to_llm(&content) {
            Ok(l) => l,
            Err(e) => {
                println!("✗ {} (parse error: {})", mdfile.display(), e);
                errors.fetch_add(1, Ordering::Relaxed);
                return;
            }
        };

        // Sanity check LLM output
        if llm.len() > content.len() * 10 || llm.len() > 1_000_000 {
            println!("✗ {} (LLM too large: {} bytes)", mdfile.display(), llm.len());
            errors.fetch_add(1, Ordering::Relaxed);
            return;
        }

        // Save .human
        if fs::write(&humanfile, &content).is_err() {
            println!("✗ {} (write error)", mdfile.display());
            errors.fetch_add(1, Ordering::Relaxed);
            return;
        }

        // Skip .machine generation for now - binary serialization has bugs
        // TODO: Fix binary writer to handle all edge cases

        println!("✓");
        processed.fetch_add(1, Ordering::Relaxed);
    });

    println!("\n=== Summary ===");
    println!("Processed: {}", processed.load(Ordering::Relaxed));
    println!("Skipped: {}", skipped.load(Ordering::Relaxed));
    println!("Errors: {}", errors.load(Ordering::Relaxed));
    */
}
