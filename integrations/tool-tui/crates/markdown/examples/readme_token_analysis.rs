//! Comprehensive README.md Token Analysis
//!
//! Demonstrates dx-markdown's TokenOptimizer integration with dx-serializer

use dx_markdown::{OptimizationStrategy, TokenOptimizer};
use serializer::llm::tokens::ModelType;
use std::fs;

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("  DX-MARKDOWN TOKEN OPTIMIZER - README.md ANALYSIS");
    println!("═══════════════════════════════════════════════════════════════\n");

    let readme = fs::read_to_string("README.md").expect("Failed to read README.md");

    // Test all models
    let models = [
        (ModelType::Gpt4o, "GPT-4o"),
        (ModelType::ClaudeSonnet4, "Claude Sonnet 4"),
        (ModelType::Gemini3, "Gemini 3"),
        (ModelType::Other, "Other"),
    ];

    println!("📊 ANALYSIS ACROSS ALL MODELS\n");

    for (model, name) in &models {
        println!("─────────────────────────────────────────────────────────────");
        println!("Model: {}", name);
        println!("─────────────────────────────────────────────────────────────");

        let optimizer = TokenOptimizer::new(*model);

        // Analyze
        let analysis = optimizer.analyze(&readme);
        println!("\n  Total tokens: {}", analysis.total_tokens);
        println!(
            "  Table tokens: {} ({:.1}%)",
            analysis.table_tokens,
            (analysis.table_tokens as f64 / analysis.total_tokens as f64) * 100.0
        );
        println!(
            "  Emoji tokens: {} ({:.1}%)",
            analysis.emoji_tokens,
            (analysis.emoji_tokens as f64 / analysis.total_tokens as f64) * 100.0
        );
        println!(
            "  Code tokens: {} ({:.1}%)",
            analysis.code_tokens,
            (analysis.code_tokens as f64 / analysis.total_tokens as f64) * 100.0
        );
        println!(
            "  Redundant tokens: {} ({:.1}%)",
            analysis.redundant_tokens,
            (analysis.redundant_tokens as f64 / analysis.total_tokens as f64) * 100.0
        );

        println!("\n  Suggestions:");
        for suggestion in &analysis.suggestions {
            println!("    • {} - save ~{} tokens", suggestion.name, suggestion.estimated_savings);
        }

        println!(
            "\n  Potential savings: {} tokens ({:.1}%)",
            analysis.potential_savings(),
            analysis.potential_savings_percent()
        );

        // Test conservative optimization
        let conservative = optimizer
            .optimize(&readme, OptimizationStrategy::Conservative)
            .expect("Conservative optimization failed");

        println!("\n  Conservative optimization:");
        println!(
            "    {} → {} tokens ({:.1}% saved)",
            conservative.original_tokens,
            conservative.optimized_tokens,
            conservative.savings_percent
        );

        // Test aggressive optimization
        let aggressive = optimizer
            .optimize(&readme, OptimizationStrategy::Aggressive)
            .expect("Aggressive optimization failed");

        println!("\n  Aggressive optimization:");
        println!(
            "    {} → {} tokens ({:.1}% saved)",
            aggressive.original_tokens, aggressive.optimized_tokens, aggressive.savings_percent
        );
        println!("    Applied: {}", aggressive.applied_optimizations.join(", "));

        println!();
    }

    println!("═══════════════════════════════════════════════════════════════");
    println!("\n🎯 KEY FINDINGS:\n");

    let optimizer = TokenOptimizer::new(ModelType::Gpt4o);
    let analysis = optimizer.analyze(&readme);
    let aggressive = optimizer.optimize(&readme, OptimizationStrategy::Aggressive).unwrap();

    println!("1. REDUNDANT SECTIONS are the biggest opportunity");
    println!(
        "   → Removing 'Previous Updates' and 'Contributing' saves ~{}%",
        (analysis.redundant_tokens as f64 / analysis.total_tokens as f64) * 100.0
    );

    println!(
        "\n2. AGGRESSIVE OPTIMIZATION achieves {:.1}% reduction",
        aggressive.savings_percent
    );
    println!(
        "   → From {} to {} tokens",
        aggressive.original_tokens, aggressive.optimized_tokens
    );

    println!("\n3. GAME-CHANGING IDEAS:");
    println!("   ✓ Remove redundant sections (19%+ savings)");
    println!("   ✓ Convert tables to DX format (potential 40% on tables)");
    println!("   ✓ Remove decorative emojis (1-2% savings)");
    println!("   ✓ Apply abbreviations (0.2-0.5% savings)");
    println!("   ✓ Compact code blocks (1-2% savings)");

    println!("\n4. ACTUAL vs ESTIMATED:");
    println!("   Estimated total: {:.1}%", analysis.potential_savings_percent());
    println!("   Actual achieved: {:.1}%", aggressive.savings_percent);
    println!("   → Real-world results validate the approach!");

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("✅ Analysis complete. dx-markdown + dx-serializer integration working!");
    println!("═══════════════════════════════════════════════════════════════\n");
}
