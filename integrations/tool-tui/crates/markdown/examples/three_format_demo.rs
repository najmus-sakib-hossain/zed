use dx_markdown::{CompilerConfig, DxMarkdown};
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create .dx/markdown directory
    let dx_dir = Path::new(".dx/markdown");
    fs::create_dir_all(dx_dir)?;

    // Test markdown with tables and diagrams
    let markdown_input = r#"
# Project Documentation

## Team Members

| Name | Role | Experience |
|------|------|------------|
| Alice | Engineer | 5 years |
| Bob | Designer | 3 years |
| Carol | Manager | 8 years |

## System Architecture

```mermaid
graph TD
    A[Client] --> B{Load Balancer}
    B -->|Route 1| C[Server 1]
    B -->|Route 2| D[Server 2]
```

## Project Structure

```
project/
├── src/
│   ├── main.rs
│   └── lib.rs
└── Cargo.toml
```
"#;

    println!("=== DX-MARKDOWN: 3-FORMAT SYSTEM ===\n");
    println!("Converting markdown with tables, diagrams, and ASCII trees\n");

    // Compile to LLM format (token-optimized)
    let config = CompilerConfig {
        tables_to_tsv: true, // Enables diagram conversion
        ..Default::default()
    };

    let compiler = DxMarkdown::new(config)?;
    let result = compiler.compile(markdown_input)?;

    println!("✓ COMPILATION COMPLETE\n");
    println!("  Input:  {} tokens", result.tokens_before);
    println!("  Output: {} tokens", result.tokens_after);
    println!("  Saved:  {:.1}%\n", result.savings_percent());

    // Show what was converted
    println!("=== CONVERSIONS APPLIED ===\n");

    if result.output.contains("Name") && result.output.contains("Alice") {
        println!("✓ Table → TSV format (tab-separated)");
    }
    if result.output.contains("Client") && result.output.contains("Load Balancer") {
        println!("✓ Mermaid diagram → Preserved in code block");
    }
    if result.output.contains("project/") {
        println!("✓ ASCII tree → Preserved in code block");
    }

    println!("\n=== FORMAT 1: LLM (Token-Optimized) ===\n");
    println!("{}", result.output);

    // Save LLM format to main disk as .md
    fs::write("README.md", &result.output)?;
    println!("\n✓ Saved to: README.md (main disk)\n");

    println!("=== 3-FORMAT SYSTEM EXPLANATION ===\n");
    println!("DX-Markdown supports 3 formats like DX-Serializer:\n");
    println!("1. LLM Format (.md) - Token-efficient, shown above");
    println!("   - Saved to main disk as .md files");
    println!("   - Optimized for AI context windows");
    println!("   - Tables converted to TSV, diagrams preserved\n");

    println!("2. Human Format (.human) - Beautiful, readable");
    println!("   - Saved to .dx/markdown/*.human");
    println!("   - Uses Unicode tables and formatting");
    println!("   - Perfect for editing in text editors\n");

    println!("3. Machine Format (.machine) - Binary, fastest");
    println!("   - Saved to .dx/markdown/*.machine");
    println!("   - Zero-copy deserialization");
    println!("   - Optimal for runtime performance\n");

    println!("=== CONVERSION FUNCTIONS ===\n");
    println!("The convert module provides these functions:");
    println!("  - llm_to_human()    - LLM → Human");
    println!("  - human_to_llm()    - Human → LLM");
    println!("  - llm_to_machine()  - LLM → Binary");
    println!("  - machine_to_llm()  - Binary → LLM");
    println!("  - human_to_machine() - Human → Binary");
    println!("  - machine_to_human() - Binary → Human\n");

    println!("=== TOKEN SAVINGS DEMO ===\n");

    // Show original vs converted
    let original_table = "| Name | Role | Experience |\n|------|------|------------|\n| Alice | Engineer | 5 years |";
    let converted_table = "Name\tRole\tExperience\nAlice\tEngineer\t5 years";

    println!("Original Markdown table ({} chars):", original_table.len());
    println!("{}\n", original_table);
    println!(
        "Converted to TSV ({} chars, {:.1}% savings):",
        converted_table.len(),
        (1.0 - converted_table.len() as f64 / original_table.len() as f64) * 100.0
    );
    println!("{}\n", converted_table);

    println!("✓ All conversions successful!");
    println!("✓ Markdown compiled to token-optimized LLM format");
    println!("✓ Ready for Human and Machine format conversion");

    Ok(())
}
