//! Aggressive Token Optimization for README.md
//!
//! This implements REAL table conversion to DX Serializer format
//! and other aggressive optimizations.

use serializer::llm::tokens::{ModelType, TokenCounter};
use std::fs;

fn main() {
    println!("🔥 AGGRESSIVE TOKEN OPTIMIZATION FOR README.md\n");

    let original = fs::read_to_string("README.md").expect("Failed to read README.md");
    let counter = TokenCounter::new();

    let original_tokens = counter.count(&original, ModelType::Gpt4o).count;
    println!("📊 Original: {} tokens\n", original_tokens);

    // Test each optimization individually
    println!("🧪 TESTING INDIVIDUAL OPTIMIZATIONS:\n");

    let opt1 = aggressive_table_conversion(&original);
    let opt1_tokens = counter.count(&opt1, ModelType::Gpt4o).count;
    let opt1_savings = ((original_tokens - opt1_tokens) as f64 / original_tokens as f64) * 100.0;
    println!(
        "1. Aggressive Table Conversion: {} tokens ({:.1}% saved)",
        opt1_tokens, opt1_savings
    );

    let opt2 = remove_all_decorative_elements(&original);
    let opt2_tokens = counter.count(&opt2, ModelType::Gpt4o).count;
    let opt2_savings = ((original_tokens - opt2_tokens) as f64 / original_tokens as f64) * 100.0;
    println!(
        "2. Remove Decorative Elements: {} tokens ({:.1}% saved)",
        opt2_tokens, opt2_savings
    );

    let opt3 = aggressive_abbreviations(&original);
    let opt3_tokens = counter.count(&opt3, ModelType::Gpt4o).count;
    let opt3_savings = ((original_tokens - opt3_tokens) as f64 / original_tokens as f64) * 100.0;
    println!(
        "3. Aggressive Abbreviations: {} tokens ({:.1}% saved)",
        opt3_tokens, opt3_savings
    );

    let opt4 = remove_redundant_sections(&original);
    let opt4_tokens = counter.count(&opt4, ModelType::Gpt4o).count;
    let opt4_savings = ((original_tokens - opt4_tokens) as f64 / original_tokens as f64) * 100.0;
    println!(
        "4. Remove Redundant Sections: {} tokens ({:.1}% saved)",
        opt4_tokens, opt4_savings
    );

    let opt5 = compact_code_examples(&original);
    let opt5_tokens = counter.count(&opt5, ModelType::Gpt4o).count;
    let opt5_savings = ((original_tokens - opt5_tokens) as f64 / original_tokens as f64) * 100.0;
    println!("5. Compact Code Examples: {} tokens ({:.1}% saved)", opt5_tokens, opt5_savings);

    // Apply ALL optimizations
    println!("\n🚀 APPLYING ALL OPTIMIZATIONS COMBINED:\n");
    let mut optimized = original.clone();
    optimized = aggressive_table_conversion(&optimized);
    optimized = remove_all_decorative_elements(&optimized);
    optimized = aggressive_abbreviations(&optimized);
    optimized = remove_redundant_sections(&optimized);
    optimized = compact_code_examples(&optimized);
    optimized = final_cleanup(&optimized);

    let final_tokens = counter.count(&optimized, ModelType::Gpt4o).count;
    let final_savings = ((original_tokens - final_tokens) as f64 / original_tokens as f64) * 100.0;

    println!("📊 FINAL RESULTS:");
    println!("   Original:  {} tokens", original_tokens);
    println!("   Optimized: {} tokens", final_tokens);
    println!("   Saved:     {} tokens", original_tokens - final_tokens);
    println!("   Reduction: {:.1}%", final_savings);

    if final_savings > 30.0 {
        println!("\n   🏆 GAME-CHANGING SUCCESS!");
    } else if final_savings > 15.0 {
        println!("\n   ✅ SIGNIFICANT IMPROVEMENT");
    } else {
        println!("\n   ⚠️  MODEST IMPROVEMENT");
    }

    fs::write("README.aggressive.md", &optimized).expect("Failed to save");
    println!("\n💾 Saved to: README.aggressive.md");

    // Show all model results
    println!("\n📊 TOKEN COUNTS ACROSS ALL MODELS:");
    let models = [
        (ModelType::Gpt4o, "GPT-4o"),
        (ModelType::ClaudeSonnet4, "Claude Sonnet 4"),
        (ModelType::Gemini3, "Gemini 3"),
        (ModelType::Other, "Other"),
    ];

    for (model, name) in &models {
        let orig = counter.count(&original, *model).count;
        let opt = counter.count(&optimized, *model).count;
        let saved = orig - opt;
        let pct = (saved as f64 / orig as f64) * 100.0;
        println!("   {:20} {} → {} ({} saved, {:.1}%)", name, orig, opt, saved, pct);
    }
}

fn aggressive_table_conversion(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Detect table start
        if line.starts_with('|') && line.ends_with('|') && i + 2 < lines.len() {
            // Check if next line is separator
            let next = lines[i + 1].trim();
            if next.contains("---") || next.contains("===") {
                // Found a table! Collect all rows
                let mut table_lines = vec![lines[i]];
                let mut j = i + 1;

                while j < lines.len() {
                    let tline = lines[j].trim();
                    if tline.starts_with('|') && tline.ends_with('|') {
                        table_lines.push(lines[j]);
                        j += 1;
                    } else {
                        break;
                    }
                }

                // Convert to DX format
                if table_lines.len() >= 3 {
                    result.push_str(&convert_table_to_dx_compact(&table_lines));
                    result.push('\n');
                    i = j;
                    continue;
                }
            }
        }

        result.push_str(lines[i]);
        result.push('\n');
        i += 1;
    }

    result
}

fn convert_table_to_dx_compact(lines: &[&str]) -> String {
    if lines.len() < 3 {
        return lines.join("\n");
    }

    // Parse header
    let header_cells: Vec<&str> = lines[0]
        .trim_matches('|')
        .split('|')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if header_cells.is_empty() {
        return lines.join("\n");
    }

    // Skip separator (lines[1])

    // Parse data rows
    let mut data_rows = Vec::new();
    for line in &lines[2..] {
        let cells: Vec<&str> = line
            .trim_matches('|')
            .split('|')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        if !cells.is_empty() {
            data_rows.push(cells);
        }
    }

    if data_rows.is_empty() {
        return lines.join("\n");
    }

    // Build DX format: t:N(h1,h2,h3)[r1c1,r1c2,r1c3 r2c1,r2c2,r2c3]
    let mut result = format!("t:{}(", header_cells.len());
    result.push_str(&header_cells.join(","));
    result.push_str(")[");

    for (i, row) in data_rows.iter().enumerate() {
        if i > 0 {
            result.push(' ');
        }
        result.push_str(&row.join(","));
    }
    result.push(']');

    result
}

fn remove_all_decorative_elements(content: &str) -> String {
    let mut result = content.to_string();

    // Remove ALL emojis
    let emojis = [
        "🚀", "🔥", "⚡", "🏆", "🌟", "🎯", "✅", "🎉", "💰", "📊", "🛠️", "🔧", "🌍", "🛡️", "📦",
        "🎨", "🗄️", "🔒", "🌐", "📚", "📋", "🚧", "💾", "🧪", "1️⃣", "2️⃣", "3️⃣", "4️⃣", "5️⃣", "6️⃣",
        "7️⃣", "8️⃣", "9️⃣", "🔟", "⚠️", "❌", "📈", "🎭",
    ];

    for emoji in &emojis {
        result = result.replace(emoji, "");
    }

    // Remove decorative separators
    result = result.replace("═════════════════════════════════════════════════════════════", "");
    result = result.replace("─────────────────────────────────────────────────────────────", "");
    result =
        result.replace("╔═══════════════════════════════════════════════════════════════╗", "");
    result =
        result.replace("╚═══════════════════════════════════════════════════════════════╝", "");
    result = result.replace("║", "");

    // Remove badge images
    let re = regex::Regex::new(r"!\[.*?\]\(.*?\)").unwrap();
    result = re.replace_all(&result, "").to_string();

    result
}

fn aggressive_abbreviations(content: &str) -> String {
    let abbrevs = vec![
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
        ("Framework", "FW"),
        ("Component", "Comp"),
        ("Function", "Fn"),
        ("Variable", "Var"),
        ("Parameter", "Param"),
        ("Benchmark", "Bench"),
        ("Compilation", "Compile"),
        ("Serialization", "Serial"),
        ("Authentication", "Auth"),
        ("Authorization", "Authz"),
        ("Database", "DB"),
        ("Accessibility", "A11y"),
        ("Internationalization", "I18n"),
        ("Compatibility", "Compat"),
        ("Progressive", "Prog"),
        ("Enhancement", "Enhance"),
        ("Management", "Mgmt"),
        ("Orchestration", "Orch"),
    ];

    let mut result = content.to_string();
    for (long, short) in abbrevs {
        result = result.replace(long, short);
    }
    result
}

fn remove_redundant_sections(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let lines: Vec<&str> = content.lines().collect();
    let mut skip_until_next_section = false;

    for line in lines {
        // Skip redundant "Previous Updates" sections
        if line.contains("## Previous Updates") || line.contains("## Latest Updates") {
            skip_until_next_section = true;
            continue;
        }

        // Skip "Contributing" section (redundant for LLM context)
        if line.contains("## Contributing") || line.contains("## Community & Support") {
            skip_until_next_section = true;
            continue;
        }

        // Skip "Acknowledgments" section
        if line.contains("## Acknowledgments") {
            skip_until_next_section = true;
            continue;
        }

        // Resume when we hit a new major section
        if line.starts_with("## ")
            && !line.contains("Previous Updates")
            && !line.contains("Latest Updates")
        {
            skip_until_next_section = false;
        }

        if !skip_until_next_section {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

fn compact_code_examples(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let lines: Vec<&str> = content.lines().collect();
    let mut in_code_block = false;
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        if line.trim().starts_with("```") {
            in_code_block = !in_code_block;
            result.push_str(line);
            result.push('\n');
        } else if in_code_block {
            // Remove comments and excessive whitespace in code blocks
            let trimmed = line.trim();
            if !trimmed.starts_with('#') && !trimmed.starts_with("//") && !trimmed.is_empty() {
                result.push_str(line);
                result.push('\n');
            }
        } else {
            result.push_str(line);
            result.push('\n');
        }

        i += 1;
    }

    result
}

fn final_cleanup(content: &str) -> String {
    let mut result = content.to_string();

    // Remove excessive blank lines (more than 2)
    let re = regex::Regex::new(r"\n{3,}").unwrap();
    result = re.replace_all(&result, "\n\n").to_string();

    // Remove trailing whitespace
    let re = regex::Regex::new(r"[ \t]+\n").unwrap();
    result = re.replace_all(&result, "\n").to_string();

    // Compress large numbers
    result = result.replace("10,000,000", "10M");
    result = result.replace("5,000,000", "5M");
    result = result.replace("2,500,000", "2.5M");
    result = result.replace("1,000,000", "1M");
    result = result.replace("100,000", "100K");
    result = result.replace("10,000", "10K");

    result
}
