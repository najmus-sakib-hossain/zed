# Quick Start Guide

## Installation

```toml
[dependencies]
dx-i18n = "0.1"
tokio = { version = "1.0", features = ["full"] }
```

## Speech-to-Text Examples

### Basic File Transcription

```rust
use dx_i18n::sts::{AutoSTT, SpeechToText};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Uses embedded tiny.en model
    let stt = AutoSTT::new("en-US", None);
    
    let transcript = stt.transcribe_file(Path::new("audio.wav")).await?;
    println!("Transcript: {}", transcript);
    
    Ok(())
}
```

### Microphone Recording

```rust
use dx_i18n::sts::{MicrophoneRecorder, WhisperSTT, SpeechToText};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Record 5 seconds
    let recorder = MicrophoneRecorder::new();
    let samples = recorder.record(5).await?;
    
    // Transcribe
    let stt = WhisperSTT::new("models/ggml-tiny.en.bin", Some("en".to_string()));
    let transcript = stt.transcribe_samples(&samples).await?;
    
    println!("You said: {}", transcript);
    
    Ok(())
}
```

### Custom Model

```rust
use dx_i18n::sts::{AutoSTT, SpeechToText};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use larger model for better accuracy
    let stt = AutoSTT::new("en-US", Some("models/ggml-large-v3.bin"));
    
    let transcript = stt.transcribe_file(Path::new("audio.wav")).await?;
    println!("Transcript: {}", transcript);
    
    Ok(())
}
```

## Text-to-Speech Examples

### Edge TTS

```rust
use dx_i18n::tts::{EdgeTTS, TextToSpeech};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tts = EdgeTTS::new("en-US-AriaNeural");
    
    // Save to file
    tts.save("Hello, world!", Path::new("output.mp3")).await?;
    
    // Or get audio bytes
    let audio = tts.synthesize("Hello, world!").await?;
    
    Ok(())
}
```

### Google TTS

```rust
use dx_i18n::tts::{GoogleTTS, TextToSpeech};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tts = GoogleTTS::new("en");
    
    tts.save("Hello, world!", Path::new("output.mp3")).await?;
    
    Ok(())
}
```

## Translation Examples

### Google Translate

```rust
use dx_i18n::locale::{GoogleTranslator, Translator};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let translator = GoogleTranslator::new();
    
    // Translate text
    let result = translator.translate("Hello", "en", "es").await?;
    println!("Spanish: {}", result);
    
    // Auto-detect source language
    let result = translator.translate("Bonjour", "auto", "en").await?;
    println!("English: {}", result);
    
    Ok(())
}
```

## Audio Format Requirements

For STT, audio must be:
- **Format**: WAV (16-bit PCM)
- **Sample Rate**: 16kHz
- **Channels**: Mono

Convert MP3 to WAV using ffmpeg:

```bash
ffmpeg -i audio.mp3 -ar 16000 -ac 1 audio.wav
```

## Performance Tips

1. **Use tiny.en for speed**: Embedded model transcribes in ~0.8s on CPU
2. **Use larger models for accuracy**: base.en, small.en, or large-v3
3. **Batch processing**: Process multiple files in parallel with tokio
4. **GPU acceleration**: Not currently supported in whisper-rs

## Error Handling

```rust
use dx_i18n::sts::{AutoSTT, SpeechToText};
use std::path::Path;

#[tokio::main]
async fn main() {
    let stt = AutoSTT::new("en-US", None);
    
    match stt.transcribe_file(Path::new("audio.wav")).await {
        Ok(transcript) => println!("Success: {}", transcript),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```
