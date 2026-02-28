# DX Media

Universal media processing toolkit built in Rust with 56 production-ready tools.

## Status

**Production Ready** | 421 Tests Passing | 56/76 Tools Verified (74%)

## Features

### ✅ Video Processing (11 tools)
- Format transcoding (MP4, WebM, MKV, AVI, MOV)
- Audio extraction
- Trimming, scaling, concatenation
- GIF creation, thumbnail extraction
- Muting, watermarking, speed adjustment
- Subtitle burning

### ✅ Audio Processing (9 tools)
- Format conversion (MP3, WAV, FLAC, OGG, AAC)
- Trimming, merging, normalization
- Silence removal, splitting
- Effects (echo, reverb), spectrum visualization
- Metadata reading

### ✅ Image Processing (7 tools)
- Format conversion (PNG, JPEG, WebP, GIF, BMP, TIFF)
- Resizing with aspect ratio preservation
- JPEG quality control
- **SVG to PNG** (native Rust, no ImageMagick)
- **Favicon generation** (web, iOS, Android)

### ✅ Archive Tools (7 tools)
- ZIP/TAR creation and extraction
- TAR.GZ compression
- Gzip compression/decompression
- Archive listing

### ✅ Document Tools (2 tools)
- Markdown to HTML conversion
- Text extraction

### ✅ Utility Tools (14 tools)
- Hash calculation (MD5, SHA256)
- Base64/URL encoding/decoding
- JSON/CSV/YAML conversion
- UUID generation/validation
- Timestamp conversion
- File duplicate detection
- Checksum verification
- File watching

### ✅ Native Processing (6 tools)
- Native audio decoding (symphonia)
- Native PDF reading (lopdf)
- Native markdown parsing (pulldown-cmark)

## Installation

```toml
[dependencies]
dx-media = "1.0"
```

### Feature Flags

```toml
# Core features (enabled by default)
default = ["cli", "image-core", "archive-core", "utility-core"]

# Optional features
image-svg = []        # SVG support (resvg)
audio-core = []       # Native audio (symphonia)
document-core = []    # Native PDF (lopdf)
```

## Quick Start

### Generate Favicons from SVG

```rust
use dx_media::tools::image::svg::generate_web_icons;

generate_web_icons("logo.svg", "public/icons")?;
// Generates: 16x16, 32x32, 48x48, 64x64, 96x96, 128x128, 192x192, 256x256, 384x384, 512x512
```

### Convert Video

```rust
use dx_media::tools::video::{transcode_video, TranscodeOptions, VideoFormat};

transcode_video("input.mkv", "output.mp4", TranscodeOptions::new(VideoFormat::Mp4))?;
```

### Extract Audio

```rust
use dx_media::tools::video::{extract_audio, AudioFormat};

extract_audio("video.mp4", "audio.mp3", AudioFormat::Mp3)?;
```

### Create Archive

```rust
use dx_media::tools::archive::create_zip;

create_zip(&["file1.txt", "file2.txt"], "archive.zip")?;
```

## Dependencies

### Required
- **FFmpeg** - Video/audio processing (11 video + 9 audio tools)

### Optional (for additional tools)
- **Ghostscript** - PDF operations (11 tools)
- **7-Zip** - Archive encryption (1 tool)
- **Tesseract** - OCR (1 tool)
- **wkhtmltopdf** - HTML to PDF (1 tool)

## Testing

```bash
# Run all tests
cargo test --all-features

# Run specific test suite
cargo test --test ffmpeg_integration_tests
cargo test --test audio_integration_tests
cargo test --test svg_integration_tests

# Test with FFmpeg
cargo test --all-features -- --test-threads=1
```

## Examples

```bash
# Generate favicons
cargo run --example generate_favicons --features image-svg

# Convert logo
cargo run --example convert_logo_native --features image-svg
```

## Architecture

- **Native Rust** - Zero external dependencies for core tools
- **FFmpeg Integration** - Battle-tested video/audio processing
- **Async/Parallel** - Tokio + Rayon for performance
- **Type-Safe** - Strong typing with comprehensive error handling
- **Well-Tested** - 421 passing tests covering all verified tools

## Performance

- **SVG Rendering**: Native resvg (no ImageMagick overhead)
- **Image Processing**: Native Rust image crate
- **Archive Operations**: Native zip/tar (no external tools)
- **Parallel Processing**: Rayon for multi-threaded operations

## License

MIT OR Apache-2.0
