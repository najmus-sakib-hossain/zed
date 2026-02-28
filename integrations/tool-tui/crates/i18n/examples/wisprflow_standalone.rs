//! Standalone Wispr Flow example
//!
//! This must be a separate binary due to symbol conflicts between
//! whisper-rs and llama-cpp-2 (both statically link ggml).
//!
//! Build with: cargo build --example wisprflow_standalone --features wisprflow
//! Run with: cargo run --example wisprflow_standalone --features wisprflow

#[cfg(feature = "wisprflow")]
use dx_i18n::wisprflow::WisprFlow;

#[cfg(feature = "wisprflow")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎙️  Wispr Flow - Offline Voice-to-Text Enhancement\n");
    println!("📋 Features:");
    println!("  ✓ Remove filler words (um, uh, like, you know)");
    println!("  ✓ Fix grammar and punctuation");
    println!("  ✓ Format for LLM prompting");
    println!("  ✓ 100% offline processing\n");

    println!("🔧 Initializing models...");

    // Create Wispr Flow processor
    let mut flow = WisprFlow::new(None, None)?;

    // Add custom sound mappings for common filler words
    flow.add_sound_mapping(" um ", " ");
    flow.add_sound_mapping(" uh ", " ");
    flow.add_sound_mapping(" like ", " ");
    flow.add_sound_mapping(" you know ", " ");
    flow.add_sound_mapping(" so ", " ");
    flow.add_sound_mapping(" well ", " ");
    flow.add_sound_mapping(" actually ", " ");
    flow.add_sound_mapping(" basically ", " ");

    println!("✅ Models loaded!\n");
    println!("🎤 Recording for 10 seconds... Speak naturally!\n");

    // Process microphone input
    let result = flow.process_microphone(10).await?;

    // Display results
    println!("\n{}", "=".repeat(60));
    println!("📝 RAW TRANSCRIPT:");
    println!("{}\n", result.raw_transcript);

    println!("{}", "=".repeat(60));
    println!("✨ ENHANCED TEXT:");
    println!("{}\n", result.enhanced_text);

    println!("{}", "=".repeat(60));
    println!("⏱️  PERFORMANCE:");
    println!("  STT (Whisper tiny.en):  {}ms", result.stt_time_ms);
    println!("  Enhancement (Qwen 0.5B): {}ms", result.enhancement_time_ms);
    println!("  Total Processing:        {}ms", result.total_time_ms);
    println!(
        "\n🚀 Speed: {:.1}x faster than typing (avg 40 WPM)",
        (result.enhanced_text.split_whitespace().count() as f64
            / (result.total_time_ms as f64 / 60000.0))
            / 40.0
    );

    Ok(())
}

#[cfg(not(feature = "wisprflow"))]
fn main() {
    eprintln!("Error: This example requires the 'wisprflow' feature.");
    eprintln!("Build with: cargo build --example wisprflow_standalone --features wisprflow");
    std::process::exit(1);
}
