//! Animation Engine Demo
//!
//! Demonstrates programmatic animation generation from class syntax.
//!
//! Run with: cargo run --example animation_demo

use style::animation::AnimationEngine;

fn main() {
    println!("=== DX Style Animation Engine ===\n");

    let engine = AnimationEngine::new();

    // 1. Basic animations
    println!("1. Basic Animation Presets:");
    let animations = vec![
        "animate-fade-in",
        "animate-fade-out",
        "animate-slide-up",
        "animate-slide-down",
        "animate-bounce",
        "animate-pulse",
        "animate-spin",
    ];

    for anim in animations {
        if let Some(css) = engine.generate_css(anim) {
            println!("   {} → {} bytes of CSS", anim, css.len());
        }
    }
    println!();

    // 2. With timing parameters
    println!("2. Animations with Timing:");
    let timed = vec![
        "animate-fade-in-500ms",
        "animate-slide-up-1s",
        "animate-bounce-2s-ease-in-out",
        "animate-pulse-1s-infinite",
    ];

    for anim in timed {
        if let Some(css) = engine.generate_css(anim) {
            println!("   {}", anim);
            let lines: Vec<&str> = css.lines().take(3).collect();
            for line in lines {
                println!("     {}", line.trim());
            }
        }
    }
    println!();

    // 3. Composed animations
    println!("3. Composed Animations:");
    let composed = vec![
        "animate-fade-in+slide-up",
        "animate-fade-in+scale-in",
        "animate-bounce+pulse",
    ];

    for anim in composed {
        if let Some(css) = engine.generate_css(anim) {
            println!("   {} → {} bytes", anim, css.len());
        }
    }
    println!();

    // 4. Full control
    println!("4. Full Animation Control:");
    let full = "animate-pulse-1s-delay-200ms-infinite-alternate-both";
    if let Some(css) = engine.generate_css(full) {
        println!("   Class: {}", full);
        println!("   Generated CSS:");
        for line in css.lines().take(8) {
            println!("     {}", line.trim());
        }
    }
    println!();

    // 5. Available presets
    println!("5. Available Presets:");
    let presets = engine.list_presets();
    println!("   Total: {} presets", presets.len());
    for (i, preset) in presets.iter().enumerate().take(10) {
        println!("   {}. {}", i + 1, preset);
    }
    if presets.len() > 10 {
        println!("   ... and {} more", presets.len() - 10);
    }
    println!();

    // 6. Custom keyframes
    println!("6. Custom Keyframes:");
    println!("   You can add custom animations via config:");
    println!("   ```");
    println!("   [animations.custom]");
    println!("   wobble = [");
    println!("     { percent = 0, transform = \"rotate(0deg)\" },");
    println!("     { percent = 25, transform = \"rotate(-5deg)\" },");
    println!("     { percent = 75, transform = \"rotate(5deg)\" },");
    println!("     { percent = 100, transform = \"rotate(0deg)\" }");
    println!("   ]");
    println!("   ```");
    println!();

    println!("=== Animation engine ready! ===");
}
