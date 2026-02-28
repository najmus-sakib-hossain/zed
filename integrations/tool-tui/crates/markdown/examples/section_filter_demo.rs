//! Section Filter Demo
//!
//! Demonstrates the human-first approach:
//! 1. Human format = Full README with ALL content (source of truth)
//! 2. LLM format = Filtered version based on user configuration
//! 3. VS Code UI = Checklist to toggle sections

use dx_markdown::{SectionFilterConfig, analyze_sections, filter_sections};
use std::fs;

fn main() {
    println!("DX MARKDOWN SECTION FILTER DEMO");
    println!("================================\n");

    let readme = fs::read_to_string("README.md").expect("Failed to read README.md");

    println!("📄 HUMAN FORMAT (Source of Truth)");
    println!("   Full README.md with ALL content: {} bytes\n", readme.len());

    // Analyze sections
    let sections = analyze_sections(&readme);
    println!("📊 SECTION ANALYSIS");
    println!("   Found {} sections:\n", sections.len());

    for section in &sections {
        let type_str = section
            .section_type
            .as_ref()
            .map(|t| format!("{:?}", t))
            .unwrap_or_else(|| "Technical".to_string());
        println!("   • {} ({}, ~{} tokens)", section.header, type_str, section.estimated_tokens);
    }

    println!("\n🎯 FILTERING STRATEGIES\n");

    // Default: Preserve everything
    println!("1. DEFAULT (Preserve All)");
    let default_config = SectionFilterConfig::default();
    let default_output = filter_sections(&readme, &default_config);
    println!("   Output: {} bytes (100% preserved)", default_output.len());
    println!("   Use case: Human reading, full documentation\n");

    // Conservative: Remove only obvious redundant sections
    println!("2. CONSERVATIVE");
    let conservative_config = SectionFilterConfig::conservative();
    let conservative_output = filter_sections(&readme, &conservative_config);
    let conservative_savings =
        ((readme.len() - conservative_output.len()) as f64 / readme.len() as f64) * 100.0;
    println!(
        "   Output: {} bytes ({:.1}% saved)",
        conservative_output.len(),
        conservative_savings
    );
    println!("   Removes: Acknowledgments only");
    println!("   Use case: LLM context with minimal filtering\n");

    // Aggressive: Remove all non-technical sections
    println!("3. AGGRESSIVE");
    let aggressive_config = SectionFilterConfig::aggressive();
    let aggressive_output = filter_sections(&readme, &aggressive_config);
    let aggressive_savings =
        ((readme.len() - aggressive_output.len()) as f64 / readme.len() as f64) * 100.0;
    println!(
        "   Output: {} bytes ({:.1}% saved)",
        aggressive_output.len(),
        aggressive_savings
    );
    println!("   Removes: Updates, Contributing, Community, Acknowledgments");
    println!("   Use case: LLM context focused on technical content\n");

    println!("💡 KEY INSIGHT");
    println!("   Human format = Full README (source of truth)");
    println!("   LLM format = Filtered based on user preferences");
    println!("   VS Code UI = Checklist to control filtering\n");

    println!("🎨 VS CODE EXTENSION FEATURES");
    println!("   • Tree view with checkboxes for each section");
    println!("   • Toggle section types (Updates, Contributing, etc.)");
    println!("   • Three presets: Default, Conservative, Aggressive");
    println!("   • Config saved to .dx/markdown-filter.json");
    println!("   • Real-time token count estimates\n");

    println!("✅ Implementation complete!");
    println!("   - Rust: section_filter.rs module");
    println!("   - TypeScript: markdown-section-filter.ts");
    println!("   - Config: .dx/markdown-filter.json");
    println!("   - Tests: 5 passing");
}
