use markdown::{DxMarkdown, CompilerConfig};
use markdown::convert::llm_to_machine;

fn main() {
    let input = "# Hello World\n\nThis is a test.";
    let config = CompilerConfig::default();
    let compiler = DxMarkdown::new(config).unwrap();
    let result = compiler.compile(input).unwrap();
    
    println!("LLM output: {} bytes", result.output.len());
    
    match llm_to_machine(&result.output) {
        Ok(bytes) => println!("Machine format: {} bytes", bytes.len()),
        Err(e) => println!("Error: {}", e),
    }
}
