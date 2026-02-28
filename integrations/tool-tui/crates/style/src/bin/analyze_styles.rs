use std::env;
use std::path::PathBuf;
/// Build-time style analyzer tool
///
/// Usage: cargo run --bin analyze_styles -- [directory]
use style::binary::StyleAnalyzer;

fn main() {
    let args: Vec<String> = env::args().collect();

    let scan_dir = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        env::current_dir().expect("Failed to get current directory")
    };

    println!("🔍 Scanning directory: {}", scan_dir.display());
    println!();

    let mut analyzer = StyleAnalyzer::new();

    match analyzer.scan_directory(&scan_dir) {
        Ok(_) => {
            // Print analysis report
            analyzer.print_report(20, 3);

            // Generate code
            println!("\n=== Generated Code ===\n");
            let code = analyzer.generate_combo_code(20, 3);
            println!("{}", code);

            // Save to file
            let output_path = scan_dir.join("detected_combos.rs");
            if let Err(e) = std::fs::write(&output_path, &code) {
                eprintln!("Error writing output: {}", e);
            } else {
                println!("\n✅ Saved to: {}", output_path.display());
            }
        }
        Err(e) => {
            eprintln!("Error scanning directory: {}", e);
            std::process::exit(1);
        }
    }
}
