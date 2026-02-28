//! Regenerate all .human files from .md files with FIGlet metadata

use dx_markdown::human_formatter::{FormatterConfig, HumanFormatter};
use std::env;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

fn main() {
    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();
    let mut use_figlet_headers = false;
    let mut use_serializer_tables = true;

    for arg in &args[1..] {
        match arg.as_str() {
            "--figlet" => use_figlet_headers = true,
            "--markdown" => use_figlet_headers = false,
            "--serializer-tables" => use_serializer_tables = true,
            "--ascii-tables" => use_serializer_tables = false,
            _ => {}
        }
    }

    println!("🔄 Regenerating .human files...");
    println!(
        "   Headers: {}",
        if use_figlet_headers {
            "FIGlet"
        } else {
            "Markdown"
        }
    );
    println!(
        "   Tables: {}\n",
        if use_serializer_tables {
            "Serializer"
        } else {
            "ASCII"
        }
    );

    let workspace_root = Path::new("../../");
    let mut count = 0;

    // Find all .md files
    for entry in WalkDir::new(workspace_root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip non-.md files
        if !path.extension().map_or(false, |ext| ext == "md") {
            continue;
        }

        // Skip node_modules, target, .git directories
        let path_str = path.to_string_lossy();
        if path_str.contains("node_modules")
            || path_str.contains("/target/")
            || path_str.contains("\\.git\\")
            || path_str.contains("/.git/")
        {
            continue;
        }

        // Read the .md file
        let md_content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(_) => continue,
        };

        // Parse as Markdown document
        let doc = match dx_markdown::markdown::MarkdownParser::parse(&md_content) {
            Ok(doc) => doc,
            Err(_) => {
                eprintln!("⚠️  Failed to parse: {}", path.display());
                continue;
            }
        };

        // Format to human format with configured settings
        let config = FormatterConfig {
            use_figlet_headers,
            use_serializer_tables,
            ..Default::default()
        };
        let mut formatter = HumanFormatter::with_config(config);
        let human_content = formatter.format(&doc);

        // Determine output path: .dx/markdown/{relative-path}/{filename}.human
        let relative_path = path.strip_prefix(workspace_root).unwrap();
        let relative_dir = relative_path.parent().unwrap();
        let filename = path.file_stem().unwrap();

        let output_dir = if relative_dir.as_os_str().is_empty() {
            workspace_root.join(".dx/markdown")
        } else {
            workspace_root.join(".dx/markdown").join(relative_dir)
        };

        let output_path = output_dir.join(format!("{}.human", filename.to_string_lossy()));

        // Create directory if needed
        if let Err(e) = fs::create_dir_all(&output_dir) {
            eprintln!("⚠️  Failed to create dir {}: {}", output_dir.display(), e);
            continue;
        }

        // Write .human file
        if let Err(e) = fs::write(&output_path, human_content) {
            eprintln!("⚠️  Failed to write {}: {}", output_path.display(), e);
            continue;
        }

        count += 1;
        if count % 10 == 0 {
            print!(".");
            use std::io::Write;
            std::io::stdout().flush().unwrap();
        }
    }

    println!("\n\n✅ Regenerated {} .human files!", count);
}
