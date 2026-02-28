use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {} <input.md> <output.machine>", args[0]);
        process::exit(1);
    }

    let input_path = &args[1];
    let output_path = &args[2];

    // Read input file
    let content = match fs::read_to_string(input_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read {}: {}", input_path, e);
            process::exit(1);
        }
    };

    // Convert to LLM format
    use dx_markdown::convert::human_to_llm;
    let llm = match human_to_llm(&content) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to parse {}: {}", input_path, e);
            process::exit(1);
        }
    };

    // Convert to machine format
    use dx_markdown::convert::llm_to_machine;
    let machine = match llm_to_machine(&llm) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to convert to machine format: {}", e);
            process::exit(1);
        }
    };

    // Write output file
    if let Err(e) = fs::write(output_path, &machine) {
        eprintln!("Failed to write {}: {}", output_path, e);
        process::exit(1);
    }
}
