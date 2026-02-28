//! Auto STS demo - Google API with Whisper fallback

use dx_i18n::sts::{AutoSTT, MicrophoneRecorder, SpeechToText};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎤 Auto STS Demo (Google API + Whisper Fallback)\n");

    // Create Auto STT with Whisper fallback
    let model_path = "models/ggml-base.en.bin";
    let stt = AutoSTT::new("en-US", Some(model_path));

    // Ensure Whisper model is downloaded
    if !Path::new(model_path).exists() {
        println!("📥 Downloading Whisper model...");
        AutoSTT::ensure_whisper_model(model_path).await?;
    }

    // Demo 1: Transcribe from file (if exists)
    if let Some(audio_file) = std::env::args().nth(1) {
        println!("📁 Transcribing file: {}", audio_file);
        match stt.transcribe_file(Path::new(&audio_file)).await {
            Ok(transcript) => println!("✓ Transcript: {}\n", transcript),
            Err(e) => eprintln!("✗ Error: {}\n", e),
        }
    }

    // Demo 2: Record from microphone
    println!("🎙️  Recording 5 seconds from microphone...");
    let recorder = MicrophoneRecorder::new();
    let samples = recorder.record(5).await?;
    println!("✓ Recorded {} samples", samples.len());

    // Save recording
    let output_path = Path::new("playgrounds/audio_output/auto_recording.wav");
    std::fs::create_dir_all("playgrounds/audio_output")?;
    recorder.save_wav(&samples, output_path)?;
    println!("✓ Saved to: {}", output_path.display());

    // Transcribe (will try Google first, fallback to Whisper)
    println!("\n🔄 Transcribing (Google API -> Whisper fallback)...");
    match stt.transcribe_samples(&samples).await {
        Ok(transcript) => println!("✓ Transcript: {}", transcript),
        Err(e) => eprintln!("✗ Error: {}", e),
    }

    println!("\n✅ Demo complete!");
    Ok(())
}
