//! Theme Generation Example
//!
//! Demonstrates dynamic theme generation from colors using Material Color Utilities.
//!
//! Run with: cargo run --example theme_generation

use style::core::color::color::Argb;
use style::theme::ThemeGenerator;

fn main() {
    println!("=== DX Style Theme Generation ===\n");

    // Create theme generator
    let generator = ThemeGenerator::new();

    // 1. Generate from hex color
    println!("1. From Hex Color (#6750A4):");
    let theme = generator.from_hex("#6750A4").expect("Valid hex");
    println!("   Source: {:?}", theme.source);
    println!("   Light Primary: {}", theme.light.primary);
    println!("   Dark Primary: {}", theme.dark.primary);
    println!();

    // 2. Generate from ARGB
    println!("2. From ARGB (255, 103, 80, 164):");
    let color = Argb::new(255, 103, 80, 164);
    let theme = generator.from_color(color);
    println!("   Light Background: {}", theme.light.background);
    println!("   Light Foreground: {}", theme.light.foreground);
    println!("   Dark Background: {}", theme.dark.background);
    println!("   Dark Foreground: {}", theme.dark.foreground);
    println!();

    // 3. Export as CSS
    println!("3. Export as CSS Custom Properties:");
    let css = generator.to_css(&theme);
    let lines: Vec<&str> = css.lines().take(10).collect();
    for line in lines {
        println!("   {}", line);
    }
    println!("   ... ({} total lines)", css.lines().count());
    println!();

    // 4. Export as DX Serializer format
    println!("4. Export as DX Serializer (.sr):");
    let sr = generator.to_dxs(&theme);
    let lines: Vec<&str> = sr.lines().take(8).collect();
    for line in lines {
        println!("   {}", line);
    }
    println!("   ... ({} total lines)", sr.lines().count());
    println!();

    // 5. Verify WCAG contrast
    println!("5. WCAG AA Contrast Verification:");
    let issues = generator.verify_contrast(&theme);
    if issues.is_empty() {
        println!("   ✓ All {} color pairs meet WCAG AA (≥4.5:1)", count_color_pairs());
    } else {
        println!("   ⚠ {} contrast issues found:", issues.len());
        for (i, issue) in issues.iter().enumerate().take(5) {
            println!(
                "     {}. {}/{} in {} mode: {:.2}:1 (need {:.1}:1)",
                i + 1,
                issue.bg,
                issue.fg,
                issue.mode,
                issue.ratio,
                issue.required
            );
        }
        if issues.len() > 5 {
            println!("     ... and {} more", issues.len() - 5);
        }
    }
    println!();

    // 6. Custom overrides
    println!("6. With Custom Overrides:");
    let generator = ThemeGenerator::new()
        .with_override("primary", "oklch(0.5 0.2 280)")
        .with_override("light.accent", "oklch(0.7 0.15 120)")
        .with_override("dark.accent", "oklch(0.5 0.15 120)");

    let theme = generator.from_color(color);
    println!("   Custom Primary: {}", theme.light.primary);
    println!("   Custom Light Accent: {}", theme.light.accent);
    println!("   Custom Dark Accent: {}", theme.dark.accent);
    println!("   Overrides Applied: {}", theme.overrides.len());
    println!();

    // 7. All tokens
    println!("7. All Generated Tokens:");
    let tokens = vec![
        ("background", &theme.light.background),
        ("foreground", &theme.light.foreground),
        ("primary", &theme.light.primary),
        ("secondary", &theme.light.secondary),
        ("accent", &theme.light.accent),
        ("muted", &theme.light.muted),
        ("border", &theme.light.border),
        ("ring", &theme.light.ring),
    ];

    for (name, value) in tokens {
        println!("   {:<15} {}", name, value);
    }
    println!();

    println!("=== Theme generation complete! ===");
}

fn count_color_pairs() -> usize {
    // Each mode has: background/foreground, card/card-foreground, primary/primary-foreground,
    // secondary/secondary-foreground, muted/muted-foreground, accent/accent-foreground,
    // destructive/destructive-foreground = 7 pairs per mode = 14 total
    14
}
