//! Test Wispr Flow with CACHED Whisper model (fast!)

#[cfg(all(feature = "whisper", feature = "wisprflow"))]
use dx_i18n::{sts::CachedWhisperSTT, wisprflow::WisprFlow};
#[cfg(all(feature = "whisper", feature = "wisprflow"))]
use std::path::PathBuf;
#[cfg(all(feature = "whisper", feature = "wisprflow"))]
use std::time::Instant;

#[cfg(all(feature = "whisper", feature = "wisprflow"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎙️  Wispr Flow - CACHED Model (Ultra Fast!)\n");

    let audio_path = PathBuf::from("F:/Code/dx/crates/i18n/audio.wav");

    if !audio_path.exists() {
        eprintln!("❌ Audio file not found: {:?}", audio_path);
        return Ok(());
    }

    let model_path = format!("{}/models/ggml-tiny.en.bin", env!("CARGO_MANIFEST_DIR"));
    let stt = CachedWhisperSTT::new(model_path, Some("en".to_string()));
    let flow = WisprFlow::new()?;

    println!("🔥 First run (loads model into memory):");
    println!("📁 Testing with: {:?}\n", audio_path);

    // First run - loads model
    let start = Instant::now();
    let raw = stt.transcribe_file(&audio_path)?;
    let stt_time = start.elapsed();
    println!("  STT: {:.2}s (includes model loading)", stt_time.as_secs_f64());
    println!("  Raw: \"{}\"\n", raw);

    let result = flow.process_text(&raw)?;
    println!("  Enhanced: \"{}\"", result.enhanced_text);
    println!(
        "  Grammar issues: {}, Style: {:.1}%\n",
        result.grammar_issues, result.style_score
    );

    // Second run - model already loaded (FAST!)
    println!("⚡ Second run (model cached in memory):");
    let start = Instant::now();
    let raw2 = stt.transcribe_file(&audio_path)?;
    let stt_time2 = start.elapsed();
    println!("  STT: {:.2}s (CACHED!)", stt_time2.as_secs_f64());

    let result2 = flow.process_text(&raw2)?;
    let enhance_time = result2.enhancement_time_ms as f64 / 1000.0;
    println!("  Enhancement: {:.3}s", enhance_time);
    println!("  ─────────────────────");
    println!("  Total: {:.2}s\n", stt_time2.as_secs_f64() + enhance_time);

    // Third run - even faster!
    println!("🚀 Third run (fully optimized):");
    let start = Instant::now();
    let raw3 = stt.transcribe_file(&audio_path)?;
    let stt_time3 = start.elapsed();
    println!("  STT: {:.2}s", stt_time3.as_secs_f64());

    let result3 = flow.process_text(&raw3)?;
    let enhance_time3 = result3.enhancement_time_ms as f64 / 1000.0;
    println!("  Enhancement: {:.3}s", enhance_time3);
    println!("  ─────────────────────");
    println!("  Total: {:.2}s", stt_time3.as_secs_f64() + enhance_time3);

    println!("\n{}", "=".repeat(70));
    println!("💡 Model stays loaded in memory for subsequent calls!");
    println!("🎯 Perfect for real-time applications!");

    Ok(())
}

#[cfg(not(all(feature = "whisper", feature = "wisprflow")))]
fn main() {
    eprintln!("Error: This example requires both 'whisper' and 'wisprflow' features.");
    eprintln!(
        "Run with: cargo run --example wisprflow_cached --features whisper,wisprflow --release"
    );
    std::process::exit(1);
}
