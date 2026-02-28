
# Changelog

All notable changes to DX Media will be documented in this file. The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## 1.0.0 - 2026-01-13

### Added

- Constants Module (`src/constants.rs`)
- `EARLY_EXIT_MULTIPLIER`
- Documented constant for search early-exit threshold (3x)
- `DEFAULT_FAILURE_THRESHOLD`
- Circuit breaker failure threshold (3)
- `DEFAULT_RESET_TIMEOUT_SECS`
- Circuit breaker reset timeout (60s)
- `DEFAULT_RATE_LIMIT_REQUESTS`
- Default rate limit (100 requests)
- `DEFAULT_RATE_LIMIT_WINDOW_SECS`
- Rate limit window (60s)
- `BASE_BACKOFF_MS`
- HTTP retry base delay (1000ms)
- `MAX_BACKOFF_JITTER_MS`
- Backoff jitter (500ms)
- Builder Methods
- `MediaAssetBuilder::build_or_log()`
- Build with debug-level logging on failure
- Integration Tests
- Wiremock-based integration tests for NASA and Openverse providers
- Test fixtures for provider response parsing
- Rate limiting integration tests
- Property-Based Tests
- Lock poisoning recovery property test
- Provider response parsing correctness property test
- Builder error message specificity property test
- Documentation
- External dependencies section with minimum versions
- Docker deployment examples (full and minimal)
- Troubleshooting guide for common issues
- Dependency matrix showing which tools require which dependencies

### Changed

- Circuit Breaker
- Safe lock handling that recovers from poisoned locks instead of panicking
- User-Agent
- Changed from browser impersonation to honest identification (`dx-media/VERSION`)
- Clippy Configuration
- Reduced blanket suppressions from 50+ to justified item-level suppressions
- HTTP Client
- Uses documented constants instead of magic numbers

### Deprecated

- `MediaAssetBuilder::try_build()`
- Use `build()` for explicit errors or `build_or_log()` for logging

### Removed

- Unused `timeout` field from HTTP client
- Dead code with "future use" comments
- Blanket `#[allow(dead_code)]` annotations

### Fixed

- Circuit breaker no longer panics on lock poisoning
- Builder validation errors now specify which field is missing

### Security

- Honest User-Agent string for responsible API usage
- SSRF prevention in URL validation
- Content-type verification for downloads
- Filename sanitization for downloaded files

## 0.1.0 - 2025-11-30

### Added

- Core Library (`dx_media`)
- `DxMedia` facade for easy library usage with fluent search builder API
- `SearchEngine` for multi-provider parallel searching
- `Downloader` with async file downloads and retry logic
- `FileManager` for organized file storage by provider/type
- `HttpClient` with built-in rate limiting and exponential backoff
- Provider Support
- Unsplash provider (images)
- requires API key
- Pexels provider (images, videos)
- requires API key
- Pixabay provider (images, videos, vectors)
- requires API key
- `ProviderRegistry` for dynamic provider management
- `Provider` trait for implementing custom providers
- CLI (`dx`)
- `dx search <query>`
- Search across all configured providers-`--type` filter (image, video, audio, gif, vector)
- `--provider` filter for specific providers
- `--count` and `--page` for pagination
- `--orientation` filter (landscape, portrait, square)
- `--color` filter for dominant color
- `--download` flag to auto-download first result
- `dx download <provider:id>`
- Download specific asset
- `dx scrape <url>`
- Scrape and download media from any website-`--type` filter (image, video, audio, gif, vector, all)
- `--count` limit for number of assets
- `--depth` for link-following depth
- `--pattern` for file pattern matching
- `--dry-run` to preview without downloading
- `dx providers`
- List available providers and their status
- `dx config`
- Show current configuration
- Multiple output formats: text, json, json-compact, tsv
- Configuration
- Environment variable configuration
- `.env` file support via dotenvy
- Configurable download directory, timeouts, retry attempts
- Per-provider API key configuration
- Types
- `MediaType` enum (Image, Video, Audio, Gif, Vector, Document, Data, Model3D, Code, Text)
- `MediaAsset` with comprehensive metadata
- `SearchQuery` with filters and pagination
- `SearchResult` with aggregated results from multiple providers
- `License` types (CC0, CC-BY, Unsplash, Pexels, Pixabay, etc.)

### Technical Details

- Built with Rust 2024 Edition
- Async runtime: Tokio with full features
- HTTP client: reqwest with rustls-tls, gzip, brotli compression
- CLI framework: clap with derive macros
- Serialization: serde + serde_json
- Error handling: thiserror + anyhow
- Logging: tracing with env-filter
