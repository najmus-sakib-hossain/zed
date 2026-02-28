//! Demo application showing i18n library usage with JSON data

use dx_i18n::locale::{GoogleTranslator, Translator};
use dx_i18n::tts::{EdgeTTS, GoogleTTS, TextToSpeech};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    id: String,
    text: String,
    language: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TranslatedMessage {
    id: String,
    original_text: String,
    translated_text: String,
    source_language: String,
    target_language: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AudioMessage {
    id: String,
    text: String,
    language: String,
    audio_file: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== i18n Library Demo ===\n");

    // Demo 1: Translation with JSON data
    demo_translation().await?;

    println!("\n{}\n", "=".repeat(60));

    // Demo 2: Text-to-Speech with JSON data
    demo_tts().await?;

    Ok(())
}

async fn demo_translation() -> Result<(), Box<dyn std::error::Error>> {
    println!("📝 Demo 1: Translation with JSON Data\n");

    // Sample JSON data with messages in English
    let messages_json = r#"[
        {
            "id": "msg_1",
            "text": "Hello, world!",
            "language": "en"
        },
        {
            "id": "msg_2",
            "text": "Good morning, how are you?",
            "language": "en"
        },
        {
            "id": "msg_3",
            "text": "Thank you for using our service.",
            "language": "en"
        },
        {
            "id": "msg_4",
            "text": "Welcome to the internationalization library.",
            "language": "en"
        }
    ]"#;

    let messages: Vec<Message> = serde_json::from_str(messages_json)?;
    println!("Original messages (JSON):");
    println!("{}\n", serde_json::to_string_pretty(&messages)?);

    // Translate to Spanish
    println!("Translating to Spanish...\n");
    let translator = GoogleTranslator::new("en", "es")?;

    let mut translated_messages = Vec::new();
    for msg in &messages {
        match translator.translate(&msg.text).await {
            Ok(translated) => {
                let translated_msg = TranslatedMessage {
                    id: msg.id.clone(),
                    original_text: msg.text.clone(),
                    translated_text: translated.clone(),
                    source_language: "en".to_string(),
                    target_language: "es".to_string(),
                };
                translated_messages.push(translated_msg);
                println!("✓ {}: {} → {}", msg.id, msg.text, translated);
            }
            Err(e) => {
                eprintln!("✗ Error translating {}: {}", msg.id, e);
            }
        }
    }

    println!("\nTranslated messages (JSON):");
    println!("{}", serde_json::to_string_pretty(&translated_messages)?);

    // Save to file
    let output_path = "playgrounds/translations.json";
    std::fs::write(output_path, serde_json::to_string_pretty(&translated_messages)?)?;
    println!("\n💾 Saved translations to: {}", output_path);

    Ok(())
}

async fn demo_tts() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔊 Demo 2: Text-to-Speech with JSON Data\n");

    // Sample JSON data with messages for TTS
    let tts_messages_json = r#"[
        {
            "id": "tts_1",
            "text": "Hello, this is a test of the text to speech system.",
            "language": "en"
        },
        {
            "id": "tts_2",
            "text": "Welcome to our application!",
            "language": "en"
        },
        {
            "id": "tts_3",
            "text": "Thank you for listening.",
            "language": "en"
        }
    ]"#;

    let messages: Vec<Message> = serde_json::from_str(tts_messages_json)?;
    println!("Messages for TTS (JSON):");
    println!("{}\n", serde_json::to_string_pretty(&messages)?);

    println!("Generating audio files...\n");

    // Create output directory if it doesn't exist
    std::fs::create_dir_all("playgrounds/audio_output")?;

    let mut audio_messages = Vec::new();

    // Demo with Google TTS
    println!("Using Google TTS:");
    let google_tts = GoogleTTS::new("en");

    for msg in &messages {
        let filename = format!("playgrounds/audio_output/{}_google.mp3", msg.id);
        match google_tts.save(&msg.text, Path::new(&filename)).await {
            Ok(_) => {
                println!("✓ Generated: {}", filename);
                audio_messages.push(AudioMessage {
                    id: msg.id.clone(),
                    text: msg.text.clone(),
                    language: msg.language.clone(),
                    audio_file: filename,
                });
            }
            Err(e) => {
                eprintln!("✗ Error generating {}: {}", filename, e);
            }
        }
    }

    // Demo with Edge TTS (commented out as it requires WebSocket support)
    println!("\nUsing Edge TTS:");
    let edge_tts = EdgeTTS::new("en-US-AriaNeural");

    for msg in &messages {
        let filename = format!("playgrounds/audio_output/{}_edge.mp3", msg.id);
        match edge_tts.save(&msg.text, Path::new(&filename)).await {
            Ok(_) => {
                println!("✓ Generated: {}", filename);
                audio_messages.push(AudioMessage {
                    id: format!("{}_edge", msg.id),
                    text: msg.text.clone(),
                    language: msg.language.clone(),
                    audio_file: filename,
                });
            }
            Err(e) => {
                eprintln!("✗ Error generating {}: {}", filename, e);
            }
        }
    }

    // Save metadata
    let metadata_path = "playgrounds/audio_metadata.json";
    std::fs::write(metadata_path, serde_json::to_string_pretty(&audio_messages)?)?;
    println!("\n💾 Saved audio metadata to: {}", metadata_path);

    Ok(())
}
