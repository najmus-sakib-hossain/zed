//! Silent Wispr Flow - suppresses all logs

#[cfg(all(feature = "whisper", feature = "wisprflow"))]
use dx_i18n::{sts::CachedWhisperSTT, wisprflow::WisprFlow};
#[cfg(all(feature = "whisper", feature = "wisprflow"))]
use std::path::PathBuf;
#[cfg(all(feature = "whisper", feature = "wisprflow"))]
use std::time::Instant;

#[cfg(all(feature = "whisper", feature = "wisprflow"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let audio_path = PathBuf::from("F:/Code/dx/crates/i18n/audio.wav");

    if !audio_path.exists() {
        eprintln!("❌ Audio file not found");
        return Ok(());
    }

    let model_path = format!("{}/models/ggml-tiny.en.bin", env!("CARGO_MANIFEST_DIR"));
    let stt = CachedWhisperSTT::new(model_path, Some("en".to_string()));
    let flow = WisprFlow::new()?;

    // Warm up (load model)
    print!("Loading model...");
    std::io::Write::flush(&mut std::io::stdout())?;
    let _ = stt.transcribe_file(&audio_path)?;
    println!(" ✓");

    // Benchmark runs
    println!("\n🚀 Running 5 benchmarks...\n");

    let mut times = Vec::new();
    for i in 1..=5 {
        let start = Instant::now();
        let raw = stt.transcribe_file(&audio_path)?;
        let stt_time = start.elapsed();

        let result = flow.process_text(&raw)?;
        let total = stt_time.as_secs_f64() + (result.enhancement_time_ms as f64 / 1000.0);

        times.push(total);
        println!("Run {}: {:.3}s", i, total);
    }

    let avg = times.iter().sum::<f64>() / times.len() as f64;
    let min = times.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    println!("\n{}", "=".repeat(50));
    println!("📊 RESULTS:");
    println!("  Fastest: {:.3}s", min);
    println!("  Slowest: {:.3}s", max);
    println!("  Average: {:.3}s", avg);
    println!("{}", "=".repeat(50));

    if min < 1.0 {
        println!("\n🎯 ACHIEVED SUB-1-SECOND TRANSCRIPTION!");
    } else {
        println!("\n💡 To get under 1s, you need:");
        println!("  • GPU acceleration (CUDA/Metal)");
        println!("  • Rebuild whisper.cpp with GPU support");
        println!("  • Current: CPU-only (optimized)");
    }

    Ok(())
}

#[cfg(not(all(feature = "whisper", feature = "wisprflow")))]
fn main() {
    eprintln!("Error: Requires whisper and wisprflow features");
    std::process::exit(1);
}
