//! DX Serializer CLI
//!
//! Processes .sr/.dx files and generates .llm and .machine outputs in .dx/serializer/

use serializer::llm::convert::CompressionAlgorithm;
use serializer::{SerializerOutput, SerializerOutputConfig};
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: dx-serialize <file.sr> [options]");
        eprintln!("       dx-serialize --dir <directory> [options]");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --output-dir <dir>    Output directory (default: .dx/serializer)");
        eprintln!("  --lz4                 Use LZ4 compression (fastest, default)");
        eprintln!("  --zstd                Use Zstd compression (better ratio)");
        eprintln!("  --speed               Alias for --lz4");
        eprintln!("  --size                Alias for --zstd");
        eprintln!("  --no-compression      Disable compression");
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  dx-serialize crates/check/rules/javascript-lint.sr");
        eprintln!("  dx-serialize --dir crates/check/rules --zstd");
        eprintln!("  dx-serialize file.sr --speed --output-dir build/");
        std::process::exit(1);
    }

    let mut output_dir = ".dx/serializer".to_string();
    let mut compression = CompressionAlgorithm::default();
    let mut input_path: Option<String> = None;
    let mut is_dir = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--dir" => {
                is_dir = true;
                if i + 1 < args.len() {
                    input_path = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--output-dir" => {
                if i + 1 < args.len() {
                    output_dir = args[i + 1].clone();
                    i += 1;
                }
            }
            "--lz4" | "--speed" => {
                compression = CompressionAlgorithm::Lz4;
            }
            "--zstd" | "--size" => {
                compression = CompressionAlgorithm::Zstd;
            }
            "--no-compression" => {
                compression = CompressionAlgorithm::None;
            }
            arg if !arg.starts_with("--") && input_path.is_none() => {
                input_path = Some(arg.to_string());
            }
            _ => {}
        }
        i += 1;
    }

    let input_path = match input_path {
        Some(p) => p,
        None => {
            eprintln!("Error: No input file or directory specified");
            std::process::exit(1);
        }
    };

    let config = SerializerOutputConfig::new()
        .with_output_dir(&output_dir)
        .with_compression(compression);
    let serializer = SerializerOutput::with_config(config);

    let compression_name = match compression {
        CompressionAlgorithm::Lz4 => "LZ4",
        CompressionAlgorithm::Zstd => "Zstd",
        CompressionAlgorithm::None => "None",
    };

    if is_dir {
        let dir = Path::new(&input_path);
        match serializer.process_directory(dir) {
            Ok(results) => {
                println!("Processed {} files (compression: {}):", results.len(), compression_name);
                for result in results {
                    println!(
                        "  {} -> {}",
                        result.paths.source.display(),
                        result.paths.llm.display()
                    );
                    println!("    LLM: {} bytes", result.llm_size);
                    println!("    Machine: {} bytes", result.machine_size);
                }
            }
            Err(e) => {
                eprintln!("Error processing directory: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        let source = Path::new(&input_path);
        match serializer.process_file(source) {
            Ok(result) => {
                println!(
                    "Generated outputs for {} (compression: {}):",
                    source.display(),
                    compression_name
                );
                println!("  LLM:     {} ({} bytes)", result.paths.llm.display(), result.llm_size);
                println!(
                    "  Machine: {} ({} bytes)",
                    result.paths.machine.display(),
                    result.machine_size
                );
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}
