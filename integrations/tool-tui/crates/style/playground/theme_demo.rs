#!/usr/bin/env rust-script
//! Theme Generation Demo
//! 
//! Run with: cargo run --example theme_demo

use style::theme::ThemeGenerator;
use style::core::color::color::Argb;

fn main() {
    println!("=== DX Style Theme Generation Demo ===\n");

    // 1. Generate from hex color
    println!("1. Generate theme from hex color (#6750A4):");
    let generator = ThemeGenerator::new();
    let theme = generator.from_hex("#6750A4").unwrap();
    
    println!("   Light mode primary: {}", theme.light.primary);
    println!("   Dark mode primary: {}", theme.dark.primary);
    println!();

    // 2. Generate from RGB
    println!("2. Generate theme from RGB (103, 80, 164):");
    let color = Argb::new(255, 103, 80, 164);
    let theme = generator.from_color(color);
    
    println!("   Light background: {}", theme.light.background);
    println!("   Dark background: {}", theme.dark.background);
    println!();

    // 3. Generate CSS output
    println!("3. Generate CSS custom properties:");
    let css = generator.to_css(&theme);
    println!("{}", &css[..500.min(css.len())]);
    println!("   ... (truncated)\n");

    // 4. Generate DX Serializer format
    println!("4. Generate DX Serializer format:");
    let sr = generator.to_dxs(&theme);
    println!("{}", &sr[..300.min(sr.len())]);
    println!("   ... (truncated)\n");

    // 5. Verify WCAG contrast
    println!("5. Verify WCAG AA contrast (≥4.5:1):");
    let issues = generator.verify_contrast(&theme);
    if issues.is_empty() {
        println!("   ✓ All color pairs meet WCAG AA standards");
    } else {
        println!("   ⚠ Found {} contrast issues:", issues.len());
        for issue in issues.iter().take(3) {
            println!("     - {}/{}: {:.2}:1 (required: {:.1}:1)", 
                issue.bg, issue.fg, issue.ratio, issue.required);
        }
    }
    println!();

    // 6. Custom overrides
    println!("6. Theme with custom overrides:");
    let generator = ThemeGenerator::new()
        .with_override("primary", "oklch(0.5 0.2 280)")
        .with_override("light.accent", "oklch(0.7 0.15 120)");
    let theme = generator.from_color(color);
    
    println!("   Custom primary: {}", theme.light.primary);
    println!("   Custom accent: {}", theme.light.accent);
    println!();

    println!("=== All theme features working! ===");
}
