//! Sound Effects for Animations
//!
//! Add audio feedback to CLI animations.

use owo_colors::OwoColorize;
use std::io::{self, Read};
use std::time::Duration;

/// Play a beep sound
pub fn play_beep() {
    // Use notification click sound
    let sound_path = std::path::Path::new("notification-click-sound-455421.mp3");

    if sound_path.exists() {
        eprintln!("🔔 BEEP!");
        let _ = play_audio_file(sound_path);
    } else {
        // Fallback to system beep
        eprintln!("🔔 BEEP!");
        for _ in 0..3 {
            print!("\x07");
            std::io::Write::flush(&mut std::io::stdout()).ok();
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

/// Play train whistle sound
pub fn play_train_whistle() {
    // Use menu open sound
    let sound_path = std::path::Path::new("menu-open-sound-effect-432999.mp3");

    if sound_path.exists() {
        eprintln!("🚂 CHOO CHOO!");
        let _ = play_audio_file(sound_path);
    } else {
        // Fallback
        eprintln!("🚂 CHOO CHOO!");
        play_beep();
        std::thread::sleep(Duration::from_millis(100));
        play_beep();
    }
}

/// Play success sound
pub fn play_success() {
    // Use success sound
    let sound_path =
        std::path::Path::new("soft-treble-win-fade-out-ending-sound-effect-416829.mp3");

    if sound_path.exists() {
        eprintln!("✨ SUCCESS!");
        let _ = play_audio_file(sound_path);
    } else {
        // Fallback
        eprintln!("✨ SUCCESS!");
        for _ in 0..3 {
            print!("\x07");
            std::io::Write::flush(&mut std::io::stdout()).ok();
            std::thread::sleep(Duration::from_millis(80));
        }
    }
}

/// Play error sound
pub fn play_error() {
    // Use warning sound
    let sound_path = std::path::Path::new("ui-3-sound-effect-warn-242229.mp3");

    if sound_path.exists() {
        eprintln!("❌ ERROR!");
        let _ = play_audio_file(sound_path);
    } else {
        // Fallback
        eprintln!("❌ ERROR!");
        print!("\x07");
        std::io::Write::flush(&mut std::io::stdout()).ok();
        std::thread::sleep(Duration::from_millis(100));
        print!("\x07");
        std::io::Write::flush(&mut std::io::stdout()).ok();
    }
}

/// Generate and play a tone (advanced)
pub fn play_tone(_frequency: f32, _duration_ms: u64) {
    // Simplified - just use beep for now
    // Full audio implementation would require rodio setup
    play_beep();
}

/// Play audio file from local path with visualizer
pub fn play_audio_file(path: &std::path::Path) -> io::Result<()> {
    play_audio_file_with_visualizer(path, true)
}

/// Play audio file with optional inline visualizer
pub fn play_audio_file_with_visualizer(
    path: &std::path::Path,
    show_visualizer: bool,
) -> io::Result<()> {
    use rodio::{Decoder, OutputStream, Sink, Source};
    use std::fs::File;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    eprintln!("   {} {}", "▶".bright_cyan(), path.display().to_string().white());

    // Create output stream
    let (_stream, stream_handle) =
        OutputStream::try_default().map_err(|e| io::Error::other(e.to_string()))?;

    // Create sink
    let sink = Sink::try_new(&stream_handle).map_err(|e| io::Error::other(e.to_string()))?;

    // Load audio file
    let file = File::open(path)?;
    let source = Decoder::new(file).map_err(|e| io::Error::other(e.to_string()))?;

    // Get audio info
    let channels = source.channels();
    let sample_rate = source.sample_rate();

    eprintln!(
        "   {} {} ch • {} Hz",
        "│".bright_black(),
        channels.to_string().bright_black(),
        sample_rate.to_string().bright_black()
    );
    eprintln!();

    // Play audio
    sink.append(source);

    if show_visualizer {
        // Inline visualizer - just a few lines
        let playing = Arc::new(Mutex::new(true));
        let playing_clone = Arc::clone(&playing);

        let viz_thread = thread::spawn(move || {
            use crossterm::{
                cursor, execute,
                style::{Color, Print, SetForegroundColor},
            };

            let bar_chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
            let mut frame = 0;
            let start = Instant::now();

            while *playing_clone.lock().unwrap() {
                // Generate animated samples
                let width = 50; // Fixed width for inline viz
                let mut bars = String::new();

                for i in 0..width {
                    let t = frame as f32 * 0.12 + i as f32 * 0.1;
                    let amplitude =
                        (t.sin() * 0.5 + (t * 1.8).sin() * 0.3 + (t * 2.3).sin() * 0.2).abs();
                    let bar_idx = (amplitude * (bar_chars.len() - 1) as f32) as usize;
                    bars.push(bar_chars[bar_idx.min(bar_chars.len() - 1)]);
                }

                // Print inline visualizer (overwrite same line)
                let elapsed = start.elapsed().as_secs();
                let mins = elapsed / 60;
                let secs = elapsed % 60;
                let time_str = if mins > 0 {
                    format!("{}:{:02}", mins, secs)
                } else {
                    format!("0:{:02}", secs)
                };

                let _ = execute!(
                    std::io::stderr(),
                    cursor::MoveToColumn(0),
                    Print("\r   "),
                    SetForegroundColor(Color::DarkGrey),
                    Print("│ "),
                    SetForegroundColor(Color::Cyan),
                    Print(&bars),
                    SetForegroundColor(Color::DarkGrey),
                    Print(format!("  {}", time_str)),
                    SetForegroundColor(Color::Reset)
                );

                thread::sleep(Duration::from_millis(50)); // 20 FPS for smooth animation
                frame += 1;
            }

            // Don't clear - leave final state
            let _ = execute!(std::io::stderr(), Print("\n"));
        });

        // Wait for playback to finish
        sink.sleep_until_end();

        // Stop visualizer
        *playing.lock().unwrap() = false;
        let _ = viz_thread.join();

        eprintln!("   {} {}", "✓".green(), "Complete".bright_black());
    } else {
        // Simple playback without visualizer
        eprintln!("   {} Playing...", "│".bright_black());
        sink.sleep_until_end();
        eprintln!("   {} {}", "✓".green(), "Complete".bright_black());
    }

    Ok(())
}

/// Download and play audio from URL with visualizer
pub fn play_audio_from_url(url: &str) -> io::Result<()> {
    play_audio_from_url_with_visualizer(url, true)
}

/// Download and play audio from URL with optional visualizer
pub fn play_audio_from_url_with_visualizer(url: &str, show_visualizer: bool) -> io::Result<()> {
    eprintln!("📥 Downloading from {}...", url);

    // Download using ureq (no tokio conflicts)
    let response = ureq::get(url).call().map_err(|e| io::Error::other(e.to_string()))?;

    if response.status() != 200 {
        return Err(io::Error::other(format!("Failed to download: HTTP {}", response.status())));
    }

    // Read response body
    let mut bytes = Vec::new();
    response.into_reader().read_to_end(&mut bytes)?;

    // Detect audio format from content
    let ext = if bytes.starts_with(b"ID3")
        || bytes.starts_with(&[0xFF, 0xFB])
        || bytes.starts_with(&[0xFF, 0xF3])
        || bytes.starts_with(&[0xFF, 0xF2])
    {
        "mp3"
    } else if bytes.starts_with(b"RIFF") && bytes.len() > 12 && &bytes[8..12] == b"WAVE" {
        "wav"
    } else if bytes.starts_with(b"OggS") {
        "ogg"
    } else if bytes.starts_with(b"fLaC") {
        "flac"
    } else {
        "mp3" // default
    };

    // Save to temporary file
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(format!("dx_temp_audio.{}", ext));

    std::fs::write(&temp_path, &bytes)?;

    eprintln!("   ✓ Downloaded {} KB", bytes.len() / 1024);
    eprintln!();

    // Play the audio with visualizer
    play_audio_file_with_visualizer(&temp_path, show_visualizer)?;

    // Clean up
    let _ = std::fs::remove_file(temp_path);

    Ok(())
}

/// Display audio waveform visualization in terminal
pub fn visualize_audio(path: &std::path::Path) -> io::Result<()> {
    use rodio::{Decoder, Source};
    use std::fs::File;

    eprintln!("Visualizing audio: {}", path.display());

    // Load audio file
    let file = File::open(path)?;
    let source = Decoder::new(file).map_err(|e| io::Error::other(e.to_string()))?;

    // Get audio info
    let channels = source.channels();
    let sample_rate = source.sample_rate();

    eprintln!("📊 Audio Info:");
    eprintln!("  Channels: {}", channels);
    eprintln!("  Sample Rate: {} Hz", sample_rate);
    eprintln!();
    eprintln!("🎵 Waveform:");
    eprintln!("  ▁▂▃▄▅▆▇█▇▆▅▄▃▂▁ ▁▂▃▄▅▆▇█▇▆▅▄▃▂▁ ▁▂▃▄▅▆▇█▇▆▅▄▃▂▁");
    eprintln!("  │                                              │");
    eprintln!("  └──────────────────────────────────────────────┘");
    eprintln!("  0s                                          End");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beep() {
        // Just verify it doesn't panic
        play_beep();
    }
}
