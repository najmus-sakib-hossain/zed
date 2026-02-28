//! Token Optimization Analysis for README.md
//!
//! This example integrates dx-serializer's TokenCounter into dx-markdown
//! to analyze and propose token-efficient optimizations for README.md.
//!
//! Run with:
//! ```bash
//! cargo run --example token_optimization_analysis -p dx-markdown --features tiktoken
//! ```

use dx_markdown::{CompilerConfig, DxMarkdown};
use serializer::llm::tokens::{ModelType, TokenCounter};
use std::fs;

fn main() {
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║  DX Markdown + Serializer Token Optimization Analysis        ║");
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // Read README.md
    let readme_path = "README.md";
    let original = match fs::read_to_string(readme_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("❌ Failed to read {}: {}", readme_path, e);
            return;
        }
    };

    let counter = TokenCounter::new();

    // Analyze original README
    println!("📊 ORIGINAL README.md TOKEN ANALYSIS");
    println!("─────────────────────────────────────────────────────────────");
    let original_counts = counter.count_primary_models(&original);

    for (model, info) in &original_counts {
        println!("  {:20} {:>6} tokens", format!("{}", model), info.count);
    }
    println!();

    // Apply dx-markdown compilation
    println!("🔧 APPLYING DX-MARKDOWN COMPILATION");
    println!("─────────────────────────────────────────────────────────────");

    let config = CompilerConfig::default();
    let compiler = match DxMarkdown::new(config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Failed to create compiler: {}", e);
            return;
        }
    };

    let compiled = match compiler.compile(&original) {
        Ok(result) => result.output,
        Err(e) => {
            eprintln!("❌ Compilation failed: {}", e);
            return;
        }
    };

    let compiled_counts = counter.count_primary_models(&compiled);

    println!("✅ Compilation complete\n");

    // Show compiled results
    println!("📊 COMPILED README.md TOKEN ANALYSIS");
    println!("─────────────────────────────────────────────────────────────");
    for (model, info) in &compiled_counts {
        println!("  {:20} {:>6} tokens", format!("{}", model), info.count);
    }
    println!();

    // Calculate savings
    println!("💰 TOKEN SAVINGS ANALYSIS");
    println!("─────────────────────────────────────────────────────────────");
    for (model, original_info) in &original_counts {
        if let Some(compiled_info) = compiled_counts.get(model) {
            let savings = original_info.count.saturating_sub(compiled_info.count);
            let percent = if original_info.count > 0 {
                (savings as f64 / original_info.count as f64) * 100.0
            } else {
                0.0
            };
            println!(
                "  {:20} {:>6} → {:>6} ({:>6} saved, {:>5.1}%)",
                format!("{}", model),
                original_info.count,
                compiled_info.count,
                savings,
                percent
            );
        }
    }
    println!();

    // Now propose game-changing optimizations
    println!("🚀 GAME-CHANGING OPTIMIZATION IDEAS");
    println!("═════════════════════════════════════════════════════════════");

    let ideas = vec![
        OptimizationIdea {
            name: "Table Compression via DX Serializer",
            description: "Convert all markdown tables to DX Serializer format (t:N syntax)",
            estimated_savings: 40.0,
            rationale: "Tables are 50-65% of README tokens. DX format saves 40-60% on tables.",
        },
        OptimizationIdea {
            name: "Emoji Removal",
            description: "Remove decorative emojis (🚀, 🔥, ⚡, etc.) - keep semantic content only",
            estimated_savings: 5.0,
            rationale: "Emojis use 2-4 tokens each. ~50 emojis = 100-200 tokens wasted.",
        },
        OptimizationIdea {
            name: "Abbreviate Common Terms",
            description: "Create abbreviation dictionary: 'JavaScript' → 'JS', 'TypeScript' → 'TS', etc.",
            estimated_savings: 8.0,
            rationale: "Repeated long words waste tokens. 'JavaScript' = 2 tokens, 'JS' = 1 token.",
        },
        OptimizationIdea {
            name: "Remove Redundant Headers",
            description: "Flatten nested headers and remove decorative separators",
            estimated_savings: 3.0,
            rationale: "Headers use 2-5 tokens each. Many are redundant with context.",
        },
        OptimizationIdea {
            name: "Compact Code Blocks",
            description: "Remove comments and whitespace from code examples",
            estimated_savings: 10.0,
            rationale: "Code blocks have excessive whitespace and comments for LLM context.",
        },
        OptimizationIdea {
            name: "URL Stripping",
            description: "Remove URLs from link text, keep only anchor text",
            estimated_savings: 4.0,
            rationale: "URLs are long and redundant. '[text](url)' → 'text' saves 5-10 tokens each.",
        },
        OptimizationIdea {
            name: "Merge Duplicate Content",
            description: "Remove repeated information across sections",
            estimated_savings: 6.0,
            rationale: "Performance numbers repeated 3-4 times. Consolidate to single section.",
        },
        OptimizationIdea {
            name: "Numeric Compression",
            description: "Use scientific notation for large numbers: 10,000,000 → 10M",
            estimated_savings: 2.0,
            rationale: "Large numbers with commas use 3-5 tokens. Abbreviations use 1-2.",
        },
    ];

    for (i, idea) in ideas.iter().enumerate() {
        println!("\n{}. {}", i + 1, idea.name);
        println!("   Description: {}", idea.description);
        println!("   Estimated Savings: {:.1}%", idea.estimated_savings);
        println!("   Rationale: {}", idea.rationale);
    }

    let total_estimated_savings: f64 = ideas.iter().map(|i| i.estimated_savings).sum();
    println!("\n─────────────────────────────────────────────────────────────");
    println!("📈 TOTAL ESTIMATED SAVINGS: {:.1}%", total_estimated_savings);
    println!("═════════════════════════════════════════════════════════════\n");

    // Now test each optimization
    println!("🧪 TESTING OPTIMIZATIONS");
    println!("═════════════════════════════════════════════════════════════");

    test_table_compression(&original, &counter);
    test_emoji_removal(&original, &counter);
    test_abbreviations(&original, &counter);
    test_url_stripping(&original, &counter);
    test_combined_optimizations(&original, &counter);
}

struct OptimizationIdea {
    name: &'static str,
    description: &'static str,
    estimated_savings: f64,
    rationale: &'static str,
}

fn test_table_compression(original: &str, counter: &TokenCounter) {
    println!("\n1️⃣  Testing: Table Compression via DX Serializer");
    println!("─────────────────────────────────────────────────────────────");

    // Find and convert tables to DX format
    let optimized = convert_tables_to_dx_format(original);

    let original_tokens = counter.count(original, ModelType::Gpt4o).count;
    let optimized_tokens = counter.count(&optimized, ModelType::Gpt4o).count;
    let savings = original_tokens.saturating_sub(optimized_tokens);
    let percent = (savings as f64 / original_tokens as f64) * 100.0;

    println!("  Original:  {:>6} tokens", original_tokens);
    println!("  Optimized: {:>6} tokens", optimized_tokens);
    println!("  Savings:   {:>6} tokens ({:.1}%)", savings, percent);
    println!(
        "  ✅ Result: {}",
        if percent > 5.0 {
            "SIGNIFICANT SAVINGS"
        } else {
            "MINOR SAVINGS"
        }
    );
}

fn test_emoji_removal(original: &str, counter: &TokenCounter) {
    println!("\n2️⃣  Testing: Emoji Removal");
    println!("─────────────────────────────────────────────────────────────");

    let optimized = remove_emojis(original);

    let original_tokens = counter.count(original, ModelType::Gpt4o).count;
    let optimized_tokens = counter.count(&optimized, ModelType::Gpt4o).count;
    let savings = original_tokens.saturating_sub(optimized_tokens);
    let percent = (savings as f64 / original_tokens as f64) * 100.0;

    println!("  Original:  {:>6} tokens", original_tokens);
    println!("  Optimized: {:>6} tokens", optimized_tokens);
    println!("  Savings:   {:>6} tokens ({:.1}%)", savings, percent);
    println!(
        "  ✅ Result: {}",
        if percent > 2.0 {
            "SIGNIFICANT SAVINGS"
        } else {
            "MINOR SAVINGS"
        }
    );
}

fn test_abbreviations(original: &str, counter: &TokenCounter) {
    println!("\n3️⃣  Testing: Common Term Abbreviations");
    println!("─────────────────────────────────────────────────────────────");

    let optimized = apply_abbreviations(original);

    let original_tokens = counter.count(original, ModelType::Gpt4o).count;
    let optimized_tokens = counter.count(&optimized, ModelType::Gpt4o).count;
    let savings = original_tokens.saturating_sub(optimized_tokens);
    let percent = (savings as f64 / original_tokens as f64) * 100.0;

    println!("  Original:  {:>6} tokens", original_tokens);
    println!("  Optimized: {:>6} tokens", optimized_tokens);
    println!("  Savings:   {:>6} tokens ({:.1}%)", savings, percent);
    println!(
        "  ✅ Result: {}",
        if percent > 3.0 {
            "SIGNIFICANT SAVINGS"
        } else {
            "MINOR SAVINGS"
        }
    );
}

fn test_url_stripping(original: &str, counter: &TokenCounter) {
    println!("\n4️⃣  Testing: URL Stripping");
    println!("─────────────────────────────────────────────────────────────");

    let optimized = strip_urls(original);

    let original_tokens = counter.count(original, ModelType::Gpt4o).count;
    let optimized_tokens = counter.count(&optimized, ModelType::Gpt4o).count;
    let savings = original_tokens.saturating_sub(optimized_tokens);
    let percent = (savings as f64 / original_tokens as f64) * 100.0;

    println!("  Original:  {:>6} tokens", original_tokens);
    println!("  Optimized: {:>6} tokens", optimized_tokens);
    println!("  Savings:   {:>6} tokens ({:.1}%)", savings, percent);
    println!(
        "  ✅ Result: {}",
        if percent > 2.0 {
            "SIGNIFICANT SAVINGS"
        } else {
            "MINOR SAVINGS"
        }
    );
}

fn test_combined_optimizations(original: &str, counter: &TokenCounter) {
    println!("\n🎯  Testing: COMBINED OPTIMIZATIONS");
    println!("═════════════════════════════════════════════════════════════");

    // Apply all optimizations in sequence
    let mut optimized = original.to_string();
    optimized = convert_tables_to_dx_format(&optimized);
    optimized = remove_emojis(&optimized);
    optimized = apply_abbreviations(&optimized);
    optimized = strip_urls(&optimized);
    optimized = remove_redundant_whitespace(&optimized);

    let original_tokens = counter.count(original, ModelType::Gpt4o).count;
    let optimized_tokens = counter.count(&optimized, ModelType::Gpt4o).count;
    let savings = original_tokens.saturating_sub(optimized_tokens);
    let percent = (savings as f64 / original_tokens as f64) * 100.0;

    println!("  Original:  {:>6} tokens", original_tokens);
    println!("  Optimized: {:>6} tokens", optimized_tokens);
    println!("  Savings:   {:>6} tokens ({:.1}%)", savings, percent);
    println!("\n  🎉 FINAL RESULT: {:.1}% TOKEN REDUCTION", percent);

    if percent > 30.0 {
        println!("  🏆 GAME-CHANGING SUCCESS!");
    } else if percent > 15.0 {
        println!("  ✅ SIGNIFICANT IMPROVEMENT");
    } else {
        println!("  ⚠️  MODEST IMPROVEMENT");
    }

    // Save optimized version
    if let Err(e) = fs::write("README.optimized.md", &optimized) {
        eprintln!("\n  ⚠️  Failed to save optimized version: {}", e);
    } else {
        println!("\n  💾 Saved optimized version to: README.optimized.md");
    }
}

// Optimization implementations

fn convert_tables_to_dx_format(content: &str) -> String {
    // Simple table detection and conversion
    // This is a basic implementation - real version would use proper markdown parsing
    let mut result = String::with_capacity(content.len());
    let mut in_table = false;
    let mut table_lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Detect table start (line with |)
        if trimmed.starts_with('|') && trimmed.ends_with('|') {
            in_table = true;
            table_lines.push(line.to_string());
        } else if in_table {
            if trimmed.is_empty() || !trimmed.contains('|') {
                // End of table - convert it
                if table_lines.len() > 2 {
                    result.push_str(&convert_table_to_dx(&table_lines));
                    result.push('\n');
                } else {
                    // Not a valid table, keep original
                    for tline in &table_lines {
                        result.push_str(tline);
                        result.push('\n');
                    }
                }
                table_lines.clear();
                in_table = false;
                result.push_str(line);
                result.push('\n');
            } else {
                table_lines.push(line.to_string());
            }
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    // Handle table at end of file
    if !table_lines.is_empty() && table_lines.len() > 2 {
        result.push_str(&convert_table_to_dx(&table_lines));
    }

    result
}

fn convert_table_to_dx(lines: &[String]) -> String {
    if lines.len() < 3 {
        return lines.join("\n");
    }

    // Parse header
    let header = &lines[0];
    let cols: Vec<&str> = header.trim_matches('|').split('|').map(|s| s.trim()).collect();

    // Skip separator line (lines[1])

    // Parse data rows
    let mut rows = Vec::new();
    for line in &lines[2..] {
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(|s| s.trim()).collect();
        if !cells.is_empty() && cells.iter().any(|c| !c.is_empty()) {
            rows.push(cells);
        }
    }

    if rows.is_empty() {
        return lines.join("\n");
    }

    // Convert to DX format: t:N(col1,col2,col3)[val1,val2,val3 ...]
    let mut result = format!("t:{}(", cols.len());
    result.push_str(&cols.join(","));
    result.push_str(")[");

    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            result.push(' ');
        }
        result.push_str(&row.join(","));
    }
    result.push(']');

    result
}

fn remove_emojis(content: &str) -> String {
    // Remove common emojis used in README
    content
        .replace("🚀", "")
        .replace("🔥", "")
        .replace("⚡", "")
        .replace("🏆", "")
        .replace("🌟", "")
        .replace("🎯", "")
        .replace("✅", "")
        .replace("🎉", "")
        .replace("💰", "")
        .replace("📊", "")
        .replace("🛠️", "")
        .replace("🔧", "")
        .replace("🌍", "")
        .replace("🛡️", "")
        .replace("📦", "")
        .replace("🎨", "")
        .replace("🗄️", "")
        .replace("🔒", "")
        .replace("🌐", "")
        .replace("📚", "")
        .replace("📋", "")
        .replace("🚧", "")
        .replace("  ", " ") // Clean up double spaces
}

fn apply_abbreviations(content: &str) -> String {
    let abbreviations = vec![
        ("JavaScript", "JS"),
        ("TypeScript", "TS"),
        ("WebAssembly", "WASM"),
        ("Application", "App"),
        ("Performance", "Perf"),
        ("Configuration", "Config"),
        ("Documentation", "Docs"),
        ("Repository", "Repo"),
        ("Development", "Dev"),
        ("Production", "Prod"),
        ("Implementation", "Impl"),
        ("Optimization", "Opt"),
        ("Architecture", "Arch"),
    ];

    let mut result = content.to_string();
    for (long, short) in abbreviations {
        result = result.replace(long, short);
    }
    result
}

fn strip_urls(content: &str) -> String {
    // Remove URLs from markdown links: [text](url) → text
    let re = regex::Regex::new(r"\[([^\]]+)\]\([^\)]+\)").unwrap();
    re.replace_all(content, "$1").to_string()
}

fn remove_redundant_whitespace(content: &str) -> String {
    // Remove excessive blank lines (more than 2 consecutive)
    let re = regex::Regex::new(r"\n{3,}").unwrap();
    re.replace_all(content, "\n\n").to_string()
}
