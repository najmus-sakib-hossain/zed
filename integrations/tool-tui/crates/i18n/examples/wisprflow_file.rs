//! Wispr Flow file-based test
//! Tests text enhancement without Whisper (avoids symbol conflicts)

#[cfg(feature = "wisprflow")]
use dx_i18n::wisprflow::WisprFlow;
#[cfg(feature = "wisprflow")]
use std::time::Instant;

#[cfg(feature = "wisprflow")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎙️  Wispr Flow - Offline Text Enhancement\n");
    println!("📋 Features:");
    println!("  ✓ Remove filler words (um, uh, like, you know)");
    println!("  ✓ Fix grammar and punctuation");
    println!("  ✓ Format for LLM prompting");
    println!("  ✓ 100% offline processing\n");

    println!("🔧 Loading Qwen 0.5B model...");

    let mut flow = WisprFlow::new(None)?;

    // Add custom sound mappings
    flow.add_sound_mapping(" um ", " ");
    flow.add_sound_mapping(" uh ", " ");
    flow.add_sound_mapping(" like ", " ");
    flow.add_sound_mapping(" you know ", " ");
    flow.add_sound_mapping(" so ", " ");
    flow.add_sound_mapping(" well ", " ");
    flow.add_sound_mapping(" actually ", " ");
    flow.add_sound_mapping(" basically ", " ");

    println!("✅ Model loaded!\n");

    // Test cases simulating raw STT output
    let test_cases = vec![
        "um so like I want you to uh create a function that um takes two numbers and uh returns the sum you know",
        "can you uh help me with uh writing a a blog post about um machine learning and uh neural networks",
        "so basically I need like a script that uh reads a file and um processes the data you know",
        "um I'm thinking we should uh implement a cache system that like stores frequently accessed data",
        "well actually I want to uh build a web app that um connects to an API and displays the results",
    ];

    println!("{}", "=".repeat(70));

    let mut total_time = 0u128;

    for (i, raw_text) in test_cases.iter().enumerate() {
        println!("\n📝 TEST {}:", i + 1);
        println!("Raw: {}", raw_text);

        let result = flow.process_text(raw_text)?;
        total_time += result.enhancement_time_ms;

        println!("✨ Enhanced: {}", result.enhanced_text);
        println!("⏱️  Time: {}ms", result.enhancement_time_ms);
        println!("{}", "-".repeat(70));
    }

    println!("\n🚀 RESULTS:");
    println!("  Total tests: {}", test_cases.len());
    println!("  Total time: {}ms", total_time);
    println!("  Average: {}ms per enhancement", total_time / test_cases.len() as u128);
    println!("\n💡 This is text enhancement only (no STT)");
    println!("   For full voice-to-text, use Whisper STT separately");

    Ok(())
}

#[cfg(not(feature = "wisprflow"))]
fn main() {
    eprintln!("Error: This example requires the 'wisprflow' feature.");
    eprintln!(
        "Run with: cargo run --example wisprflow_file --features wisprflow --no-default-features"
    );
    std::process::exit(1);
}
