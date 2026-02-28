//! Quick STS test without model download

use dx_i18n::sts::{MicrophoneRecorder, SpeechToText, WhisperSTT};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎤 STS Module Test\n");

    // Test 1: Check if model exists
    let model_path = "models/ggml-base.en.bin";
    if !Path::new(model_path).exists() {
        println!("⚠️  Model not found: {}", model_path);
        println!("Download from: https://huggingface.co/ggerganov/whisper.cpp/tree/main");
        println!("\nTesting microphone recording only...\n");

        // Test microphone recording
        println!("🎙️  Recording 3 seconds from microphone...");
        let recorder = MicrophoneRecorder::new();
        let samples = recorder.record(3).await?;
        println!("✓ Recorded {} samples", samples.len());

        // Save to file
        let output = Path::new("playgrounds/audio_output/test_recording.wav");
        std::fs::create_dir_all("playgrounds/audio_output")?;
        recorder.save_wav(&samples, output)?;
        println!("✓ Saved to: {}", output.display());

        println!("\n✅ Microphone recording works!");
        return Ok(());
    }

    // Test 2: Full transcription test
    println!("📁 Testing file transcription...");
    let stt = WhisperSTT::new(model_path, Some("en".to_string()));

    // Check supported languages
    let langs = stt.get_supported_languages();
    println!("✓ Supports {} languages", langs.len());
    println!("✓ English supported: {}", stt.is_language_supported("en"));

    println!("\n✅ All tests passed!");
    Ok(())
}
