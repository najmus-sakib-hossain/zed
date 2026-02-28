//! Example: Generating audio from JSON scripts

use i18n::tts::{GoogleTTS, EdgeTTS, TextToSpeech};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
struct ScriptLine {
    speaker: String,
    text: String,
    voice: Option<String>,
    language: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Script {
    title: String,
    lines: Vec<ScriptLine>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Sample script in JSON format
    let script_json = r#"{
        "title": "Product Demo",
        "lines": [
            {
                "speaker": "Narrator",
                "text": "Welcome to our revolutionary new product.",
                "language": "en"
            },
            {
                "speaker": "Narrator",
                "text": "This product will change the way you work.",
                "language": "en"
            },
            {
                "speaker": "Customer",
                "text": "This is exactly what I needed!",
                "voice": "en-US-JennyNeural",
                "language": "en"
            },
            {
                "speaker": "Narrator",
                "text": "Try it today and see the difference.",
                "language": "en"
            }
        ]
    }"#;

    let script: Script = serde_json::from_str(script_json)?;
    
    println!("Processing script: {}", script.title);
    println!("Total lines: {}\n", script.lines.len());

    // Create output directory
    std::fs::create_dir_all("script_audio")?;

    // Generate audio for each line
    for (index, line) in script.lines.iter().enumerate() {
        println!("[{}] {}: \"{}\"", index + 1, line.speaker, line.text);
        
        // Use Edge TTS with custom voice if specified
        if let Some(ref voice) = line.voice {
            let tts = EdgeTTS::new(voice);
            let filename = format!("script_audio/line_{:02}_edge.mp3", index + 1);
            
            match tts.save(&line.text, Path::new(&filename)).await {
                Ok(_) => println!("   ✓ Saved: {}", filename),
                Err(e) => eprintln!("   ✗ Error: {}", e),
            }
        } else {
            // Use Google TTS as fallback
            let tts = GoogleTTS::new(&line.language, "com", false);
            let filename = format!("script_audio/line_{:02}_google.mp3", index + 1);
            
            match tts.save(&line.text, Path::new(&filename)).await {
                Ok(_) => println!("   ✓ Saved: {}", filename),
                Err(e) => eprintln!("   ✗ Error: {}", e),
            }
        }
    }

    println!("\n✅ Script processing complete!");

    Ok(())
}
