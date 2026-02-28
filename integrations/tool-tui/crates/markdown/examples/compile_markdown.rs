//! Compile MARKDOWN.md and output the result
//!
//! Run with: cargo run -p dx-markdown --example compile_markdown

#![allow(clippy::expect_used, clippy::unwrap_used)] // Examples can use expect/unwrap for clarity

use dx_markdown::compiler::DxMarkdown;
use dx_markdown::tokenizer::Tokenizer;
use dx_markdown::types::{CompilerConfig, TokenizerType};
use std::fs;

fn main() {
    // Read MARKDOWN.md
    let content = fs::read_to_string("MARKDOWN.md").expect("Failed to read MARKDOWN.md");

    println!("=== DX Markdown Compilation ===\n");
    println!("Input file: MARKDOWN.md");
    println!("Input size: {} bytes\n", content.len());

    // Create compiler with default config
    let config = CompilerConfig::default();
    let compiler = DxMarkdown::new(config).expect("Failed to create compiler");

    // Compile
    let result = compiler.compile(&content).expect("Compilation failed");

    // Save output
    fs::write("DX_MARKDOWN.md", &result.output).expect("Failed to write DX_MARKDOWN.md");
    println!("Output saved to: DX_MARKDOWN.md");
    println!("Output size: {} bytes\n", result.output.len());

    // Count tokens with different tokenizers
    println!("=== Token Counts ===\n");

    // cl100k (GPT-4, Claude 3.5/4)
    let cl100k = Tokenizer::new(TokenizerType::Cl100k).unwrap();
    let before_cl100k = cl100k.count(&content);
    let after_cl100k = cl100k.count(&result.output);

    // o200k (GPT-4o, GPT-5)
    let o200k = Tokenizer::new(TokenizerType::O200k).unwrap();
    let before_o200k = o200k.count(&content);
    let after_o200k = o200k.count(&result.output);

    // p50k (GPT-3.5)
    let p50k = Tokenizer::new(TokenizerType::P50k).unwrap();
    let before_p50k = p50k.count(&content);
    let after_p50k = p50k.count(&result.output);

    println!(
        "{:<35} {:>10} {:>10} {:>10} {:>10}",
        "Model/Tokenizer", "Before", "After", "Saved", "Savings %"
    );
    println!("{}", "=".repeat(85));

    // Claude 4.5 Opus (uses similar tokenization to cl100k)
    let saved = before_cl100k - after_cl100k;
    let pct = (saved as f64 / before_cl100k as f64) * 100.0;
    println!(
        "{:<35} {:>10} {:>10} {:>10} {:>9.1}%",
        "Claude 4.5 Opus (cl100k-like)", before_cl100k, after_cl100k, saved, pct
    );

    // Claude 3.5 Sonnet
    println!(
        "{:<35} {:>10} {:>10} {:>10} {:>9.1}%",
        "Claude 3.5 Sonnet (cl100k-like)", before_cl100k, after_cl100k, saved, pct
    );

    // Gemini models (use similar tokenization)
    println!(
        "{:<35} {:>10} {:>10} {:>10} {:>9.1}%",
        "Gemini 2.0 Flash (cl100k-like)", before_cl100k, after_cl100k, saved, pct
    );
    println!(
        "{:<35} {:>10} {:>10} {:>10} {:>9.1}%",
        "Gemini 1.5 Pro (cl100k-like)", before_cl100k, after_cl100k, saved, pct
    );

    // GPT-4o
    let saved_o200k = before_o200k - after_o200k;
    let pct_o200k = (saved_o200k as f64 / before_o200k as f64) * 100.0;
    println!(
        "{:<35} {:>10} {:>10} {:>10} {:>9.1}%",
        "GPT-4o (o200k)", before_o200k, after_o200k, saved_o200k, pct_o200k
    );

    // GPT-4
    println!(
        "{:<35} {:>10} {:>10} {:>10} {:>9.1}%",
        "GPT-4 Turbo (cl100k)", before_cl100k, after_cl100k, saved, pct
    );

    // GPT-3.5
    let saved_p50k = before_p50k - after_p50k;
    let pct_p50k = (saved_p50k as f64 / before_p50k as f64) * 100.0;
    println!(
        "{:<35} {:>10} {:>10} {:>10} {:>9.1}%",
        "GPT-3.5 Turbo (p50k)", before_p50k, after_p50k, saved_p50k, pct_p50k
    );

    println!("\n=== Summary ===");
    println!("Original: {} tokens (cl100k)", before_cl100k);
    println!("Optimized: {} tokens (cl100k)", after_cl100k);
    println!("Saved: {} tokens ({:.1}%)", saved, pct);
}
