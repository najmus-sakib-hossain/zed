//! Example: Combined translation and TTS workflow

use i18n::locale::{GoogleTranslator, Translator};
use i18n::tts::{GoogleTTS, TextToSpeech};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
struct Announcement {
    id: String,
    english_text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct LocalizedAnnouncement {
    id: String,
    language: String,
    language_code: String,
    text: String,
    audio_file: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Original announcements in English
    let announcements_json = r#"[
        {
            "id": "announcement_1",
            "english_text": "Attention passengers, the train will depart in 5 minutes."
        },
        {
            "id": "announcement_2",
            "english_text": "Please stand behind the yellow line."
        },
        {
            "id": "announcement_3",
            "english_text": "Thank you for traveling with us."
        }
    ]"#;

    let announcements: Vec<Announcement> = serde_json::from_str(announcements_json)?;

    println!("Original announcements:");
    for ann in &announcements {
        println!("  - {}", ann.english_text);
    }
    println!();

    // Languages to translate to
    let languages = vec![
        ("es", "Spanish"),
        ("fr", "French"),
        ("de", "German"),
        ("ja", "Japanese"),
    ];

    std::fs::create_dir_all("announcements")?;
    let mut all_localized = Vec::new();

    for (lang_code, lang_name) in languages {
        println!("Processing {}...", lang_name);
        
        let translator = GoogleTranslator::new("en", lang_code)?;
        let tts = GoogleTTS::new(lang_code, "com", false);

        for ann in &announcements {
            // Translate
            let translated_text = translator.translate(&ann.english_text).await?;
            println!("  ✓ Translated: {}", translated_text);

            // Generate audio
            let audio_filename = format!(
                "announcements/{}_{}.mp3",
                ann.id, lang_code
            );
            
            tts.save(&translated_text, Path::new(&audio_filename)).await?;
            println!("  ✓ Audio saved: {}", audio_filename);

            // Store metadata
            all_localized.push(LocalizedAnnouncement {
                id: ann.id.clone(),
                language: lang_name.to_string(),
                language_code: lang_code.to_string(),
                text: translated_text,
                audio_file: audio_filename,
            });
        }
        println!();
    }

    // Save all metadata
    let metadata_file = "announcements/metadata.json";
    std::fs::write(
        metadata_file,
        serde_json::to_string_pretty(&all_localized)?,
    )?;

    println!("✅ Complete! Generated {} localized announcements", all_localized.len());
    println!("📄 Metadata saved to: {}", metadata_file);

    Ok(())
}
