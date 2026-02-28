# dx-i18n Documentation

## Overview

dx-i18n provides internationalization capabilities including translation, text-to-speech, and speech-to-text.

## Modules

### Speech-to-Text (STT)

Located in `src/sts/`:

- **WhisperSTT**: Offline Whisper-based transcription
- **GoogleSTT**: Google Speech Recognition API (disabled by default)
- **AutoSTT**: Automatic fallback (Whisper primary, Google fallback)
- **MicrophoneRecorder**: Record audio from microphone

The default configuration uses an embedded **tiny.en** model (76MB) for fast CPU transcription.

### Text-to-Speech (TTS)

Located in `src/tts/`:

- **EdgeTTS**: Microsoft Edge TTS (free, no API key)
- **GoogleTTS**: Google Translate TTS (free, no API key)

### Translation

Located in `src/locale/`:

- **GoogleTranslator**: Google Translate API
- **DeepLTranslator**: DeepL API (requires API key)

## Performance

### STT Benchmarks (13-second audio on CPU)

- **tiny.en** (76MB): 0.8s - Embedded, fastest
- **base.en** (141MB): 1.5s - Better accuracy
- **small.en** (465MB): 4.2s - High accuracy
- **large-v3** (1.6GB): 23s - Best accuracy

## Architecture

```
dx-i18n/
├── src/
│   ├── sts/          # Speech-to-text
│   │   ├── whisper.rs
│   │   ├── google.rs
│   │   └── auto.rs
│   ├── tts/          # Text-to-speech
│   │   ├── edge.rs
│   │   └── google.rs
│   └── locale/       # Translation
│       └── google.rs
├── models/
│   └── ggml-tiny.en.bin  # Embedded model
└── playgrounds/      # Examples
```

## Usage Examples

See [QUICKSTART.md](QUICKSTART.md) for detailed examples.
