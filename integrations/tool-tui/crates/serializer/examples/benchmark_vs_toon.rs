//! DX vs TOON Token Efficiency Benchmark
//! Run with: cargo run --example benchmark_vs_toon -p dx-serializer --features tiktoken

use serializer::llm::tokens::{ModelType, TokenCounter};
use std::fs;

fn main() {
    let counter = TokenCounter::new();

    println!("DX Serializer vs TOON - Token Efficiency Benchmark");
    println!("===================================================\n");

    let examples = [
        ("example1_mixed", "Mixed (hikes)"),
        ("example2_nested", "Logs (tabular)"),
        ("example3_deep", "Orders (nested)"),
        ("example4_config", "Config (objects)"),
        ("example5_leaf", "Leaf (dotted)"),
    ];

    let models = [
        (ModelType::Gpt4o, "GPT-4o"),
        (ModelType::ClaudeOpus45, "Claude"),
        (ModelType::Gemini3, "Gemini"),
    ];

    // Benchmark each model
    for (model, model_name) in &models {
        println!("=== {} ===", model_name);
        println!("{:<18} {:>8} {:>8} {:>10}", "Test Case", "TOON", "DX", "Savings");
        println!("{}", "-".repeat(46));

        let mut total_toon = 0usize;
        let mut total_dx = 0usize;

        for (filename, label) in examples.iter() {
            let toon_path = format!("essence/{}.toon", filename);
            let dx_path = format!("essence/{}.sr", filename);

            let toon_content = match fs::read_to_string(&toon_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let dx_content = match fs::read_to_string(&dx_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let toon_tokens = counter.count(&toon_content, *model).count;
            let dx_tokens = counter.count(&dx_content, *model).count;

            let savings = ((toon_tokens as f64 - dx_tokens as f64) / toon_tokens as f64) * 100.0;

            println!("{:<18} {:>8} {:>8} {:>9.1}%", label, toon_tokens, dx_tokens, savings);

            total_toon += toon_tokens;
            total_dx += dx_tokens;
        }

        println!("{}", "-".repeat(46));
        let total_savings = ((total_toon as f64 - total_dx as f64) / total_toon as f64) * 100.0;
        println!("{:<18} {:>8} {:>8} {:>9.1}%", "TOTAL", total_toon, total_dx, total_savings);
        println!();
    }

    // Summary table
    println!("=== SUMMARY (All Models) ===\n");
    println!("{:<12} {:>10} {:>10} {:>10}", "Model", "TOON", "DX", "Savings");
    println!("{}", "-".repeat(44));

    for (model, model_name) in &models {
        let mut total_toon = 0usize;
        let mut total_dx = 0usize;

        for (filename, _) in examples.iter() {
            let toon_path = format!("essence/{}.toon", filename);
            let dx_path = format!("essence/{}.sr", filename);

            if let (Ok(toon), Ok(dx)) =
                (fs::read_to_string(&toon_path), fs::read_to_string(&dx_path))
            {
                total_toon += counter.count(&toon, *model).count;
                total_dx += counter.count(&dx, *model).count;
            }
        }

        let savings = ((total_toon as f64 - total_dx as f64) / total_toon as f64) * 100.0;
        println!("{:<12} {:>10} {:>10} {:>9.1}%", model_name, total_toon, total_dx, savings);
    }

    // Calculate average across all models
    let mut grand_total_toon = 0usize;
    let mut grand_total_dx = 0usize;

    for (model, _) in &models {
        for (filename, _) in examples.iter() {
            let toon_path = format!("essence/{}.toon", filename);
            let dx_path = format!("essence/{}.sr", filename);

            if let (Ok(toon), Ok(dx)) =
                (fs::read_to_string(&toon_path), fs::read_to_string(&dx_path))
            {
                grand_total_toon += counter.count(&toon, *model).count;
                grand_total_dx += counter.count(&dx, *model).count;
            }
        }
    }

    let avg_savings =
        ((grand_total_toon as f64 - grand_total_dx as f64) / grand_total_toon as f64) * 100.0;
    println!("Average across all models: {:.1}% savings\n", avg_savings);

    println!("=== WHY DX BEATS TOON ===");
    println!("1. No indentation (TOON requires 2 spaces per row)");
    println!("2. Inline tables (no newlines between rows)");
    println!("3. Prefix elimination (@/api/ @2025-01-15T)");
    println!("4. Compact headers: name:N(schema) vs name[N]{{schema}}:");
}
