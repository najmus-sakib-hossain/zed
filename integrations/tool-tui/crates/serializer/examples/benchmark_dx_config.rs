//! Benchmark: DX Config Format Comparison
//!
//! Compares token efficiency of DX LLM format vs TOON, JSON, TOML, YAML, CSV
//!
//! Run with: cargo run --example benchmark_dx_config -p dx-serializer --features tiktoken

use std::fs;
use std::path::Path;

#[cfg(feature = "tiktoken")]
use tiktoken_rs::{cl100k_base, o200k_base};

fn main() {
    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║           DX Config Format Token Efficiency Benchmark                        ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    let base_path = Path::new("essence/datasets");

    let formats = vec![
        ("DX (LLM)", "dx-config.sr"),
        ("TOON", "dx-config.toon"),
        ("JSON", "dx-config.json"),
        ("JSON (minified)", "dx-config.min.json"),
        ("TOML", "dx-config.toml"),
        ("YAML", "dx-config.yaml"),
        ("CSV", "dx-config.csv"),
    ];

    let mut results: Vec<(String, usize, usize, usize, usize)> = Vec::new();

    for (name, file) in &formats {
        let path = base_path.join(file);
        match fs::read_to_string(&path) {
            Ok(content) => {
                let bytes = content.len();
                let chars = content.chars().count();

                #[cfg(feature = "tiktoken")]
                let (gpt4o, claude) = {
                    let gpt4o_bpe = o200k_base().expect("Failed to load o200k tokenizer");
                    let claude_bpe = cl100k_base().expect("Failed to load cl100k tokenizer");
                    let gpt4o_tokens = gpt4o_bpe.encode_with_special_tokens(&content).len();
                    let claude_tokens = claude_bpe.encode_with_special_tokens(&content).len();
                    (gpt4o_tokens, claude_tokens)
                };

                #[cfg(not(feature = "tiktoken"))]
                let (gpt4o, claude) = {
                    let estimated = chars / 4;
                    (estimated, estimated)
                };

                results.push((name.to_string(), bytes, chars, gpt4o, claude));
            }
            Err(e) => {
                eprintln!("Failed to read {}: {}", file, e);
            }
        }
    }

    if results.is_empty() {
        eprintln!("No files found. Run from workspace root.");
        return;
    }

    // Print size comparison
    println!("┌─────────────────────┬──────────┬──────────┬──────────┬──────────┐");
    println!("│ Format              │ Bytes    │ Chars    │ GPT-4o   │ Claude   │");
    println!("├─────────────────────┼──────────┼──────────┼──────────┼──────────┤");

    for (name, bytes, chars, gpt4o, claude) in &results {
        println!("│ {:<19} │ {:>8} │ {:>8} │ {:>8} │ {:>8} │", name, bytes, chars, gpt4o, claude);
    }
    println!("└─────────────────────┴──────────┴──────────┴──────────┴──────────┘\n");

    // Calculate savings vs DX
    let dx_result = &results[0];
    let dx_gpt4o = dx_result.3;
    let dx_claude = dx_result.4;

    println!("┌─────────────────────┬────────────────┬────────────────┐");
    println!("│ Format              │ GPT-4o Savings │ Claude Savings │");
    println!("├─────────────────────┼────────────────┼────────────────┤");

    for (name, _, _, gpt4o, claude) in &results {
        let gpt4o_savings = if *gpt4o > dx_gpt4o {
            let pct = ((*gpt4o - dx_gpt4o) as f64 / *gpt4o as f64) * 100.0;
            format!("DX saves {:.1}%", pct)
        } else if *gpt4o < dx_gpt4o {
            let pct = ((dx_gpt4o - *gpt4o) as f64 / dx_gpt4o as f64) * 100.0;
            format!("{} saves {:.1}%", name, pct)
        } else {
            "Equal".to_string()
        };

        let claude_savings = if *claude > dx_claude {
            let pct = ((*claude - dx_claude) as f64 / *claude as f64) * 100.0;
            format!("DX saves {:.1}%", pct)
        } else if *claude < dx_claude {
            let pct = ((dx_claude - *claude) as f64 / dx_claude as f64) * 100.0;
            format!("{} saves {:.1}%", name, pct)
        } else {
            "Equal".to_string()
        };

        println!("│ {:<19} │ {:>14} │ {:>14} │", name, gpt4o_savings, claude_savings);
    }
    println!("└─────────────────────┴────────────────┴────────────────┘\n");

    // Summary
    println!("═══════════════════════════════════════════════════════════════════════════════");
    println!("SUMMARY: DX LLM Format Token Efficiency");
    println!("═══════════════════════════════════════════════════════════════════════════════\n");

    let json_result = results.iter().find(|r| r.0 == "JSON").unwrap();
    let toon_result = results.iter().find(|r| r.0 == "TOON").unwrap();
    let yaml_result = results.iter().find(|r| r.0 == "YAML").unwrap();
    let toml_result = results.iter().find(|r| r.0 == "TOML").unwrap();

    let json_gpt4o_savings = ((json_result.3 - dx_gpt4o) as f64 / json_result.3 as f64) * 100.0;
    let toon_gpt4o_savings = ((toon_result.3 - dx_gpt4o) as f64 / toon_result.3 as f64) * 100.0;
    let yaml_gpt4o_savings = ((yaml_result.3 - dx_gpt4o) as f64 / yaml_result.3 as f64) * 100.0;
    let toml_gpt4o_savings = ((toml_result.3 - dx_gpt4o) as f64 / toml_result.3 as f64) * 100.0;

    println!("DX vs JSON:     {:.1}% fewer tokens (GPT-4o)", json_gpt4o_savings);
    println!("DX vs TOON:     {:.1}% fewer tokens (GPT-4o)", toon_gpt4o_savings);
    println!("DX vs YAML:     {:.1}% fewer tokens (GPT-4o)", yaml_gpt4o_savings);
    println!("DX vs TOML:     {:.1}% fewer tokens (GPT-4o)", toml_gpt4o_savings);
    println!();

    // Winner announcement
    let min_tokens = results.iter().map(|r| r.3).min().unwrap();
    let winner = results.iter().find(|r| r.3 == min_tokens).unwrap();
    println!("🏆 WINNER: {} with {} GPT-4o tokens!", winner.0, winner.3);
}
