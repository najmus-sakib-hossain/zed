
# i18n Playground Examples

This directory contains practical examples demonstrating how to use the i18n library for translation and text-to-speech operations with JSON data.

## 📂 Examples

### 1. `translate_locale.rs` - JSON Localization Translation

Demonstrates how to translate JSON localization files to multiple languages. What it does: -Loads a JSON structure with app strings (English) -Recursively translates all text values -Generates locale files for Spanish, French, and German -Saves translated JSONs to files Run it:
```bash
cargo run --example translate_locale ```
Output: -`locale_es.json` - Spanish translations -`locale_fr.json` - French translations -`locale_de.json` - German translations Use case: Perfect for internationalizing web apps, mobile apps, or any software with locale files.


### 2. `generate_audio.rs` - Audio from JSON Scripts


Shows how to generate audio files from JSON-formatted scripts with multiple speakers. What it does: -Reads a script from JSON (with speakers and lines) -Generates audio for each line using TTS -Supports custom voices per speaker -Saves audio files sequentially Run it:
```bash
cargo run --example generate_audio ```
Output: -`script_audio/line_01_*.mp3` -`script_audio/line_02_*.mp3` -etc. Use case: Great for creating voice-overs, audiobooks, podcast scripts, or video narration.

### 3. `translate_and_speak.rs` - Combined Translation & TTS

Demonstrates a complete workflow: translate text to multiple languages and generate audio for each. What it does: -Translates announcements from English to Spanish, French, German, Japanese -Generates audio files for each translation -Saves metadata with translations and audio file paths -Creates a complete localized audio library Run it:
```bash
cargo run --example translate_and_speak ```
Output: -`announcements/announcement_1_es.mp3` -`announcements/announcement_1_fr.mp3` -`announcements/metadata.json` - Complete metadata Use case: Ideal for creating multilingual announcements, IVR systems, public transportation announcements, or accessibility features.


## 🚀 Getting Started


- Make sure you have the i18n library built:
```bash
cargo build ```
- Run any example:
```bash
cargo run --example <example_name> ```
- Check the output files in the current directory


## 💡 Tips


- Rate Limiting: Google Translate has rate limits. If you hit them, wait a few seconds and try again.
- API Keys: For Microsoft Translator, set the `MICROSOFT_API_KEY` environment variable.
- Audio Quality: Edge TTS generally provides higher quality audio than Google TTS.
- File Formats: All audio files are generated in MP3 format.


## 🎯 Common Patterns



### Pattern 1: Batch Translation


```rust
let texts = vec!["Hello", "Goodbye", "Thank you"];
let results = translator.translate_batch(&texts).await?;
```


### Pattern 2: Error Handling


```rust
match translator.translate(text).await { Ok(result) => println!("Success: {}", result), Err(e) => eprintln!("Error: {}", e), }
```


### Pattern 3: Sequential Audio Generation


```rust
for (i, text) in texts.iter().enumerate() { let filename = format!("audio_{}.mp3", i);
tts.save(text, Path::new(&filename)).await?;
}
```


## 📚 Learn More


- Check the main README.md (../README.md) for full API documentation
- See src/bin/demo.rs (../src/bin/demo.rs) for the main demo application
- Browse the source code in src/ (../src/) for implementation details


## 🤝 Contributing


Feel free to add your own examples! Just create a new `.rs` file in this directory and document it here. This directory showcases i18n features to localize means translate text into different languages and formats and tts features to convert text into spoken words into different voices and styles.
