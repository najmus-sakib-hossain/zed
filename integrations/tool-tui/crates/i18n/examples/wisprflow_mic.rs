//! Wispr Flow - Complete voice-to-text enhancement pipeline
//! Records 10s from microphone → Whisper STT → Rust_Grammar enhancement

#[cfg(all(feature = "whisper", feature = "wisprflow"))]
use dx_i18n::{sts::AutoSTT, wisprflow::WisprFlow};
#[cfg(all(feature = "whisper", feature = "wisprflow"))]
use std::fs;
#[cfg(all(feature = "whisper", feature = "wisprflow"))]
use std::path::PathBuf;
#[cfg(all(feature = "whisper", feature = "wisprflow"))]
use std::time::Instant;

#[cfg(all(feature = "whisper", feature = "wisprflow"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎙️  Wispr Flow - Complete Voice Enhancement Pipeline\n");
    println!("📋 Pipeline:");
    println!("  1. Record 10s from microphone");
    println!("  2. Whisper STT (tiny.en model)");
    println!("  3. Rust_Grammar text enhancement");
    println!("  4. Show timing breakdown\n");

    let start_total = Instant::now();

    // Step 1: Record audio from microphone
    println!("🎤 Recording 10 seconds from microphone...");
    let record_start = Instant::now();

    let temp_wav = PathBuf::from("temp_recording.wav");
    record_audio(&temp_wav, 10)?;

    let record_time = record_start.elapsed();
    println!("✅ Recording complete: {:.2}s\n", record_time.as_secs_f64());

    // Step 2: Speech-to-text with Whisper
    println!("🔊 Transcribing with Whisper tiny.en...");
    let stt_start = Instant::now();

    let stt = AutoSTT::new(None::<String>)?;
    let raw_transcript = stt.transcribe_file(&temp_wav)?;

    let stt_time = stt_start.elapsed();
    println!("✅ Transcription complete: {:.2}s", stt_time.as_secs_f64());
    println!("📝 Raw: \"{}\"\n", raw_transcript);

    // Step 3: Text enhancement with Rust_Grammar
    println!("✨ Enhancing with Rust_Grammar...");
    let enhance_start = Instant::now();

    let flow = WisprFlow::new()?;
    let result = flow.process_text(&raw_transcript)?;

    let enhance_time = enhance_start.elapsed();
    println!("✅ Enhancement complete: {:.2}s", enhance_time.as_secs_f64());
    println!("📝 Enhanced: \"{}\"\n", result.enhanced_text);

    // Step 4: Results
    let total_time = start_total.elapsed();

    println!("{}", "=".repeat(70));
    println!("🚀 RESULTS:");
    println!("{}", "=".repeat(70));
    println!("📊 Grammar Issues Fixed: {}", result.grammar_issues);
    println!("⭐ Style Score: {:.1}%", result.style_score);
    println!();
    println!("⏱️  TIMING BREAKDOWN:");
    println!("  Recording:     {:.2}s", record_time.as_secs_f64());
    println!("  STT (Whisper): {:.2}s", stt_time.as_secs_f64());
    println!("  Enhancement:   {:.2}s", enhance_time.as_secs_f64());
    println!("  ─────────────────────");
    println!("  Total:         {:.2}s", total_time.as_secs_f64());
    println!();
    println!("💡 100% offline processing!");
    println!("🎯 Faster than Wispr Flow with better grammar correction!");

    // Cleanup
    let _ = fs::remove_file(temp_wav);

    Ok(())
}

#[cfg(all(feature = "whisper", feature = "wisprflow"))]
fn record_audio(
    output_path: &PathBuf,
    duration_secs: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use hound::{WavSpec, WavWriter};
    use std::sync::{Arc, Mutex};

    let host = cpal::default_host();
    let device = host.default_input_device().ok_or("No input device available")?;

    println!("  Using device: {}", device.name()?);

    let config = device.default_input_config()?;
    let original_sample_rate = config.sample_rate().0;
    let channels = config.channels();

    println!("  Original: {}Hz, {} channels", original_sample_rate, channels);
    println!("  Converting to: 16000Hz, 1 channel (mono) for Whisper");

    // Whisper requires 16kHz mono
    let target_sample_rate = 16000;
    let spec = WavSpec {
        channels: 1, // Mono for Whisper
        sample_rate: target_sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let writer = Arc::new(Mutex::new(WavWriter::create(output_path, spec)?));
    let writer_clone = writer.clone();

    // Track audio levels
    let max_amplitude = Arc::new(Mutex::new(0.0f32));
    let max_amplitude_clone = max_amplitude.clone();

    let stream = device.build_input_stream(
        &config.into(),
        move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let mut writer = writer_clone.lock().unwrap();
            let mut max_amp = max_amplitude_clone.lock().unwrap();

            // Resample and convert to mono
            let resample_ratio = original_sample_rate as f32 / target_sample_rate as f32;
            let mut i = 0.0f32;

            while (i as usize) < data.len() {
                let idx = i as usize;

                // Convert stereo to mono by averaging channels
                let sample = if channels == 2 && idx + 1 < data.len() {
                    (data[idx] + data[idx + 1]) / 2.0
                } else if idx < data.len() {
                    data[idx]
                } else {
                    break;
                };

                // Track max amplitude
                let abs_sample = sample.abs();
                if abs_sample > *max_amp {
                    *max_amp = abs_sample;
                }

                // Convert to i16 and write
                let amplitude = (sample * i16::MAX as f32) as i16;
                writer.write_sample(amplitude).ok();

                i += resample_ratio * channels as f32;
            }
        },
        |err| eprintln!("Stream error: {}", err),
        None,
    )?;

    stream.play()?;

    println!("  🎤 SPEAK NOW! Say something clearly...");

    // Show progress with audio level indicator
    for i in 1..=duration_secs {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let max_amp = *max_amplitude.lock().unwrap();
        let level_bars = (max_amp * 20.0) as usize;
        let bars = "█".repeat(level_bars.min(20));
        print!(
            "\r  Recording: {}s / {}s  Level: [{}{}]",
            i,
            duration_secs,
            bars,
            " ".repeat(20 - level_bars.min(20))
        );
        std::io::Write::flush(&mut std::io::stdout())?;
    }
    println!();

    let final_max = *max_amplitude.lock().unwrap();
    if final_max < 0.01 {
        println!("  ⚠️  WARNING: Very low audio level detected ({:.4})", final_max);
        println!("      Make sure your microphone is working and not muted!");
    } else {
        println!("  ✓ Audio level OK (max: {:.2})", final_max);
    }

    drop(stream);

    // Take ownership from Arc<Mutex<>> to finalize
    let writer = Arc::try_unwrap(writer)
        .map_err(|_| "Failed to unwrap Arc")?
        .into_inner()
        .map_err(|_| "Failed to get inner value")?;
    writer.finalize()?;

    Ok(())
}

#[cfg(not(all(feature = "whisper", feature = "wisprflow")))]
fn main() {
    eprintln!("Error: This example requires both 'whisper' and 'wisprflow' features.");
    eprintln!("Run with: cargo run --example wisprflow_mic --features whisper,wisprflow");
    std::process::exit(1);
}
