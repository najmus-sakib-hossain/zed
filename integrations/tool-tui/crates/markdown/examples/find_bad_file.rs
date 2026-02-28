use dx_markdown::convert::human_to_machine;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("Finding the problematic file...\n");

    let dx_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let mut files = Vec::new();
    collect_files(&dx_root, &mut files);

    println!("Collected {} files\n", files.len());

    for (i, path) in files.iter().enumerate() {
        print!("{}. Testing {} ... ", i + 1, path.display());
        std::io::Write::flush(&mut std::io::stdout()).ok();

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                println!("read error: {}", e);
                continue;
            }
        };

        match human_to_machine(&content) {
            Ok(_) => println!("OK"),
            Err(e) => {
                println!("FAILED: {:?}", e);
                println!("\nProblematic file found!");
                println!("Path: {}", path.display());
                println!("Size: {} bytes", content.len());
                println!("First 500 chars:\n{}", content.chars().take(500).collect::<String>());
                break;
            }
        }
    }
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) {
    if files.len() >= 100 {
        return;
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        if files.len() >= 100 {
            return;
        }

        let path = entry.path();
        let path_str = path.to_string_lossy();

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
            collect_files(&path, files);
        } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
            if let Ok(metadata) = fs::metadata(&path) {
                if metadata.len() < 500_000 {
                    files.push(path);
                }
            }
        }
    }
}
