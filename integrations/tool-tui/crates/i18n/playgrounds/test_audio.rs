//! Test transcription of audio.mp3 using both Google and Whisper

use dx_i18n::sts::{GoogleSTT, SpeechToText, WhisperSTT};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let audio_path = "audio.mp3";

    if !Path::new(audio_path).exists() {
        eprintln!("Error: audio.mp3 not found in crates/i18n/");
        return Ok(());
    }

    println!("🎵 Testing transcription of audio.mp3\n");

    // First convert MP3 to WAV (MP3 not directly supported)
    println!("📦 Converting MP3 to WAV...");
    convert_mp3_to_wav(audio_path, "audio.wav")?;
    println!("✓ Converted to audio.wav\n");

    let wav_path = Path::new("audio.wav");

    // Test 1: Google Free API (with timeout)
    println!("🌐 Test 1: Google Free API (10s timeout)");
    println!("─────────────────────────────────────────");
    let google_stt = GoogleSTT::new("en-US", None).with_timeout(std::time::Duration::from_secs(10));

    let start = std::time::Instant::now();
    match google_stt.transcribe_file(wav_path).await {
        Ok(transcript) => {
            println!("✓ Google Transcript ({}ms):", start.elapsed().as_millis());
            println!("{}\n", transcript);
        }
        Err(e) => {
            eprintln!("✗ Google API Error ({}ms): {}\n", start.elapsed().as_millis(), e);
        }
    }

    // Test 2: Whisper Offline (Medium model for better accuracy)
    println!("🤖 Test 2: Whisper Offline (Medium Model)");
    println!("──────────────────────────────────────────");
    let model_path = "models/ggml-medium.en.bin";

    // Download medium model if not exists
    if !Path::new(model_path).exists() {
        println!("📥 Downloading Whisper medium model (1.5GB)...");
        println!("This will take several minutes...");
        download_whisper_model(model_path).await?;
    }

    let whisper_stt = WhisperSTT::new(model_path, Some("en".to_string()));
    match whisper_stt.transcribe_file(wav_path).await {
        Ok(transcript) => {
            println!("✓ Whisper Transcript:");
            println!("{}\n", transcript);
        }
        Err(e) => {
            eprintln!("✗ Whisper Error: {}\n", e);
        }
    }

    println!("✅ Transcription tests complete!");
    Ok(())
}

async fn download_whisper_model(model_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    use dx_i18n::sts::AutoSTT;
    AutoSTT::ensure_whisper_model(model_path).await?;
    Ok(())
}

fn convert_mp3_to_wav(input: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;

    // Try using ffmpeg to convert
    let status = Command::new("ffmpeg")
        .args(&[
            "-i", input, "-ar", "16000", // 16kHz sample rate
            "-ac", "1",  // mono
            "-y", // overwrite
            output,
        ])
        .status();

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("ffmpeg failed with status: {}", s).into()),
        Err(e) => {
            eprintln!("Error: ffmpeg not found. Please install ffmpeg.");
            eprintln!("Windows: choco install ffmpeg");
            eprintln!("Or download from: https://ffmpeg.org/download.html");
            Err(e.into())
        }
    }
}
