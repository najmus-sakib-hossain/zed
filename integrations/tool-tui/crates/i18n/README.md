# dx-i18n

High-performance internationalization library with translation, text-to-speech (TTS), and speech-to-text (STT) capabilities.

## Features

- **Translation**: Multi-provider translation (Google, DeepL, etc.)
- **Text-to-Speech**: Edge TTS and Google TTS support
- **Speech-to-Text**: Whisper-based offline transcription with embedded tiny.en model (76MB)
- **Fast**: 0.8s transcription on CPU with embedded model
- **Offline**: No external dependencies required for STT

## Quick Start

### Speech-to-Text (STT)

```rust
use dx_i18n::sts::{AutoSTT, SpeechToText};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Uses embedded tiny.en model automatically
    let stt = AutoSTT::new("en-US", None);
    
    let transcript = stt.transcribe_file(Path::new("audio.wav")).await?;
    println!("Transcript: {}", transcript);
    
    Ok(())
}
```

### Text-to-Speech (TTS)

```rust
use dx_i18n::tts::{EdgeTTS, TextToSpeech};
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tts = EdgeTTS::new("en-US-AriaNeural");
    
    tts.save("Hello, world!", Path::new("output.mp3")).await?;
    
    Ok(())
}
```

### Translation

```rust
use dx_i18n::locale::{GoogleTranslator, Translator};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let translator = GoogleTranslator::new();
    
    let result = translator.translate("Hello", "en", "es").await?;
    println!("Translation: {}", result);
    
    Ok(())
}
```

## STT Models

The crate includes an embedded **tiny.en** model (76MB) for fast English transcription:

- **Speed**: ~0.8s per 13-second audio on CPU
- **Accuracy**: Suitable for most English transcription tasks
- **No Download**: Model is embedded in the binary

For higher accuracy, you can use custom models:

```rust
let stt = AutoSTT::new("en-US", Some("path/to/ggml-large-v3.bin"));
```

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
dx-i18n = "0.1"
tokio = { version = "1.0", features = ["full"] }
```

## Examples

See `playgrounds/` directory for more examples:

- `sts_demo.rs` - Speech-to-text with file and microphone input
- `auto_sts_demo.rs` - Auto STT with fallback
- `test_features.rs` - Translation and TTS examples

## License

Licensed under MIT or Apache-2.0.
