use std::fs;
use std::panic;
use std::path::PathBuf;
use walkdir::WalkDir;

fn main() {
    // Set panic hook to prevent abort
    panic::set_hook(Box::new(|_| {
        // Silent panic handler
    }));

    println!("Generating .human and .machine files for all markdown files...\n");

    let root = PathBuf::from("../../");
    let mut processed = 0;
    let mut skipped = 0;
    let mut errors = 0;

    // Find all .md files
    for entry in WalkDir::new(&root).follow_links(false).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        // Skip if not a .md file
        if !path.extension().map(|e| e == "md").unwrap_or(false) {
            continue;
        }

        // Skip if in target/, node_modules/, or .git/
        let path_str = path.to_string_lossy();
        if path_str.contains("/target/")
            || path_str.contains("\\target\\")
            || path_str.contains("/node_modules/")
            || path_str.contains("\\node_modules\\")
            || path_str.contains("/.git/")
            || path_str.contains("\\.git\\")
        {
            continue;
        }

        // Skip if .human or .machine already exist and are newer
        let human_path = path.with_extension("md.human");
        let machine_path = path.with_extension("md.machine");

        if let Ok(md_meta) = fs::metadata(path) {
            let md_modified = md_meta.modified().ok();

            let human_exists = human_path.exists();
            let machine_exists = machine_path.exists();

            if human_exists && machine_exists {
                if let (Some(md_time), Ok(human_meta), Ok(machine_meta)) =
                    (md_modified, fs::metadata(&human_path), fs::metadata(&machine_path))
                {
                    if let (Ok(human_time), Ok(machine_time)) =
                        (human_meta.modified(), machine_meta.modified())
                    {
                        if human_time >= md_time && machine_time >= md_time {
                            skipped += 1;
                            continue;
                        }
                    }
                }
            }
        }

        // Read the markdown file
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("✗ Failed to read {}: {}", path.display(), e);
                errors += 1;
                continue;
            }
        };

        // Convert to human format (with panic handling)
        use dx_markdown::convert::human_to_llm;
        let llm = match panic::catch_unwind(|| human_to_llm(&content)) {
            Ok(Ok(l)) => l,
            Ok(Err(e)) => {
                eprintln!("✗ Failed to convert {}: {} (skipping)", path.display(), e);
                errors += 1;
                continue;
            }
            Err(_) => {
                eprintln!("✗ Panic while converting {} (skipping)", path.display());
                errors += 1;
                continue;
            }
        };

        // Save .human file (just copy the original markdown)
        if let Err(e) = fs::write(&human_path, &content) {
            eprintln!("✗ Failed to write {}: {}", human_path.display(), e);
            errors += 1;
            continue;
        }

        // Convert to machine format
        use dx_markdown::convert::llm_to_machine;
        let machine = match llm_to_machine(&llm) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("✗ Failed to convert {} to machine: {}", path.display(), e);
                errors += 1;
                continue;
            }
        };

        // Save .machine file
        if let Err(e) = fs::write(&machine_path, &machine) {
            eprintln!("✗ Failed to write {}: {}", machine_path.display(), e);
            errors += 1;
            continue;
        }

        println!("✓ {}", path.display());
        processed += 1;
    }

    println!("\n=== Summary ===");
    println!("Processed: {}", processed);
    println!("Skipped (up-to-date): {}", skipped);
    println!("Errors: {}", errors);
}
