//! Speech-to-text demo with file and microphone input

use dx_i18n::sts::{MicrophoneRecorder, SpeechToText, WhisperSTT};
use std::path::Path;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎤 Speech-to-Text Demo\n");

    let model_path =
        std::env::args().nth(1).unwrap_or_else(|| "models/ggml-base.en.bin".to_string());

    println!("Using model: {}\n", model_path);

    // Demo 1: Transcribe from audio file
    if let Some(audio_file) = std::env::args().nth(2) {
        println!("📁 Demo 1: Transcribe from file");
        println!("File: {}", audio_file);

        let stt = WhisperSTT::new(&model_path, Some("en".to_string()));

        let start = Instant::now();
        let transcript = stt.transcribe_file(Path::new(&audio_file)).await?;
        let duration = start.elapsed();

        println!("Transcript: {}", transcript);
        println!("Time: {:.2}s\n", duration.as_secs_f64());
    }

    // Demo 2: Record from microphone and transcribe
    println!("🎙️  Demo 2: Record from microphone");
    println!("Recording for 5 seconds...");

    let recorder = MicrophoneRecorder::new();
    let samples = recorder.record(5).await?;

    println!("✓ Recorded {} samples", samples.len());

    // Save recording
    let output_path = Path::new("playgrounds/audio_output/recording.wav");
    std::fs::create_dir_all("playgrounds/audio_output")?;
    recorder.save_wav(&samples, output_path)?;
    println!("✓ Saved to: {}", output_path.display());

    // Transcribe
    println!("Transcribing...");
    let stt = WhisperSTT::new(&model_path, Some("en".to_string()));

    let start = Instant::now();
    let transcript = stt.transcribe_samples(&samples).await?;
    let duration = start.elapsed();

    println!("Transcript: {}", transcript);
    println!("Time: {:.2}s\n", duration.as_secs_f64());

    println!("✅ Demo complete!");
    Ok(())
}
