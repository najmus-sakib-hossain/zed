//! # Text-to-Speech Integration
//!
//! Unified TTS interface supporting multiple providers:
//! - ElevenLabs (premium quality)
//! - OpenAI TTS (fallback)
//! - Edge TTS (free fallback)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::tts::{TtsProvider, TtsConfig};
//!
//! let config = TtsConfig::from_file("~/.dx/config/tts.sr")?;
//! let tts = TtsProvider::new(&config)?;
//!
//! // Speak text
//! tts.speak("Hello, world!").await?;
//!
//! // With specific voice
//! tts.speak_with_voice("Hello!", "voice_id").await?;
//!
//! // Get audio bytes for custom playback
//! let audio = tts.synthesize("Hello!").await?;
//! ```

pub mod elevenlabs;
pub mod openai;
pub mod edge;

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

pub use elevenlabs::ElevenLabsClient;
pub use openai::OpenAiTtsClient;
pub use edge::EdgeTtsClient;

/// TTS provider selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TtsProviderType {
    /// ElevenLabs API (premium)
    #[default]
    ElevenLabs,
    /// OpenAI TTS API
    OpenAi,
    /// Microsoft Edge TTS (free)
    Edge,
    /// Local TTS (Piper, etc.)
    Local,
}

/// TTS voice configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsVoice {
    /// Voice identifier (provider-specific)
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Voice language/locale
    pub language: String,
    /// Voice gender
    pub gender: Option<VoiceGender>,
    /// Voice description
    pub description: Option<String>,
}

/// Voice gender
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VoiceGender {
    Male,
    Female,
    Neutral,
}

/// Auto-speak mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AutoMode {
    /// Always speak responses
    Always,
    /// Never auto-speak
    Never,
    /// Smart mode based on content length
    #[default]
    Smart,
}

/// TTS configuration loaded from tts.sr
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    /// Selected TTS provider
    #[serde(default)]
    pub provider: TtsProviderType,
    /// Auto-speak mode
    #[serde(default)]
    pub auto_mode: AutoMode,
    /// Maximum text length before summarizing
    #[serde(default = "default_max_text_length")]
    pub max_text_length: usize,
    /// Whether to summarize long content
    #[serde(default = "default_true")]
    pub summarize_long: bool,
    /// ElevenLabs-specific config
    pub elevenlabs: Option<ElevenLabsConfig>,
    /// OpenAI TTS-specific config
    pub openai: Option<OpenAiTtsConfig>,
    /// Edge TTS-specific config
    pub edge: Option<EdgeTtsConfig>,
    /// Local TTS-specific config
    pub local: Option<LocalTtsConfig>,
}

fn default_max_text_length() -> usize {
    1500
}

fn default_true() -> bool {
    true
}

/// ElevenLabs configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElevenLabsConfig {
    /// API key (from env: ELEVENLABS_API_KEY)
    #[serde(default)]
    pub api_key: String,
    /// Default voice ID
    #[serde(default = "default_elevenlabs_voice")]
    pub voice_id: String,
    /// Model ID
    #[serde(default = "default_elevenlabs_model")]
    pub model_id: String,
    /// Voice stability (0.0 - 1.0)
    #[serde(default = "default_stability")]
    pub stability: f32,
    /// Similarity boost (0.0 - 1.0)
    #[serde(default = "default_similarity")]
    pub similarity_boost: f32,
    /// Style (0.0 - 1.0)
    #[serde(default)]
    pub style: f32,
    /// Speech speed multiplier
    #[serde(default = "default_speed")]
    pub speed: f32,
}

fn default_elevenlabs_voice() -> String {
    "pMsXgVXv3BLzUgSXRplE".to_string() // Adam voice
}

fn default_elevenlabs_model() -> String {
    "eleven_multilingual_v2".to_string()
}

fn default_stability() -> f32 {
    0.5
}

fn default_similarity() -> f32 {
    0.75
}

fn default_speed() -> f32 {
    1.0
}

/// OpenAI TTS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiTtsConfig {
    /// API key (from env: OPENAI_API_KEY)
    #[serde(default)]
    pub api_key: String,
    /// Voice name (alloy, echo, fable, onyx, nova, shimmer)
    #[serde(default = "default_openai_voice")]
    pub voice: String,
    /// Model (tts-1, tts-1-hd)
    #[serde(default = "default_openai_model")]
    pub model: String,
    /// Speed multiplier (0.25 - 4.0)
    #[serde(default = "default_speed")]
    pub speed: f32,
    /// Response format (mp3, opus, aac, flac)
    #[serde(default = "default_openai_format")]
    pub response_format: String,
}

fn default_openai_voice() -> String {
    "alloy".to_string()
}

fn default_openai_model() -> String {
    "tts-1".to_string()
}

fn default_openai_format() -> String {
    "mp3".to_string()
}

/// Edge TTS configuration (free)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeTtsConfig {
    /// Voice name (e.g., "en-US-AriaNeural")
    #[serde(default = "default_edge_voice")]
    pub voice: String,
    /// Speech rate (-50% to +100%)
    #[serde(default)]
    pub rate: i32,
    /// Pitch adjustment (-50Hz to +50Hz)
    #[serde(default)]
    pub pitch: i32,
    /// Volume adjustment (-100% to +100%)
    #[serde(default)]
    pub volume: i32,
}

fn default_edge_voice() -> String {
    "en-US-AriaNeural".to_string()
}

/// Local TTS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalTtsConfig {
    /// Path to local TTS engine (e.g., Piper)
    pub engine_path: PathBuf,
    /// Model path
    pub model_path: PathBuf,
    /// Sample rate
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u32,
}

fn default_sample_rate() -> u32 {
    22050
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            provider: TtsProviderType::default(),
            auto_mode: AutoMode::default(),
            max_text_length: default_max_text_length(),
            summarize_long: true,
            elevenlabs: None,
            openai: None,
            edge: Some(EdgeTtsConfig {
                voice: default_edge_voice(),
                rate: 0,
                pitch: 0,
                volume: 0,
            }),
            local: None,
        }
    }
}

impl TtsConfig {
    /// Load configuration from a .sr file
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| DrivenError::Io(e))?;
        
        // Parse using DX Serializer (sr format)
        // For now, fallback to default if parsing fails
        Self::parse_sr(&content).or_else(|_| Ok(Self::default()))
    }

    /// Parse from SR format string
    fn parse_sr(content: &str) -> Result<Self> {
        // TODO: Use dx-serializer for proper .sr parsing
        // For now, use a simple TOML-like parser
        Ok(Self::default())
    }

    /// Resolve environment variables in config
    pub fn resolve_env_vars(&mut self) {
        if let Some(ref mut el) = self.elevenlabs {
            if el.api_key.is_empty() || el.api_key.starts_with('$') {
                el.api_key = std::env::var("ELEVENLABS_API_KEY").unwrap_or_default();
            }
        }
        if let Some(ref mut oai) = self.openai {
            if oai.api_key.is_empty() || oai.api_key.starts_with('$') {
                oai.api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
            }
        }
    }
}

/// Synthesized audio result
#[derive(Debug, Clone)]
pub struct SynthesizedAudio {
    /// Audio data bytes
    pub data: Vec<u8>,
    /// Audio format (mp3, wav, ogg, etc.)
    pub format: AudioFormat,
    /// Duration in milliseconds
    pub duration_ms: Option<u64>,
    /// Sample rate
    pub sample_rate: Option<u32>,
}

/// Audio format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    Mp3,
    Wav,
    Ogg,
    Opus,
    Aac,
    Flac,
    Pcm,
}

impl AudioFormat {
    /// Get file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Wav => "wav",
            Self::Ogg => "ogg",
            Self::Opus => "opus",
            Self::Aac => "aac",
            Self::Flac => "flac",
            Self::Pcm => "pcm",
        }
    }

    /// Get MIME type for this format
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Mp3 => "audio/mpeg",
            Self::Wav => "audio/wav",
            Self::Ogg => "audio/ogg",
            Self::Opus => "audio/opus",
            Self::Aac => "audio/aac",
            Self::Flac => "audio/flac",
            Self::Pcm => "audio/pcm",
        }
    }
}

/// Unified TTS provider interface
pub struct TtsProvider {
    config: TtsConfig,
    elevenlabs: Option<ElevenLabsClient>,
    openai: Option<OpenAiTtsClient>,
    edge: Option<EdgeTtsClient>,
    /// Currently active provider
    active: TtsProviderType,
}

impl TtsProvider {
    /// Create a new TTS provider from config
    pub fn new(config: &TtsConfig) -> Result<Self> {
        let mut config = config.clone();
        config.resolve_env_vars();

        let elevenlabs = config.elevenlabs.as_ref().map(|c| ElevenLabsClient::new(c));
        let openai = config.openai.as_ref().map(|c| OpenAiTtsClient::new(c));
        let edge = config.edge.as_ref().map(|c| EdgeTtsClient::new(c));

        Ok(Self {
            active: config.provider,
            config,
            elevenlabs,
            openai,
            edge,
        })
    }

    /// Create with ElevenLabs as primary
    pub fn elevenlabs(config: &ElevenLabsConfig) -> Result<Self> {
        let mut tts_config = TtsConfig::default();
        tts_config.provider = TtsProviderType::ElevenLabs;
        tts_config.elevenlabs = Some(config.clone());
        Self::new(&tts_config)
    }

    /// Create with OpenAI as primary
    pub fn openai(config: &OpenAiTtsConfig) -> Result<Self> {
        let mut tts_config = TtsConfig::default();
        tts_config.provider = TtsProviderType::OpenAi;
        tts_config.openai = Some(config.clone());
        Self::new(&tts_config)
    }

    /// Create with Edge TTS as primary (free)
    pub fn edge(config: &EdgeTtsConfig) -> Result<Self> {
        let mut tts_config = TtsConfig::default();
        tts_config.provider = TtsProviderType::Edge;
        tts_config.edge = Some(config.clone());
        Self::new(&tts_config)
    }

    /// Synthesize text to audio
    pub async fn synthesize(&self, text: &str) -> Result<SynthesizedAudio> {
        // Check if text needs summarization
        let text_to_speak = if self.config.summarize_long && text.len() > self.config.max_text_length {
            self.summarize_text(text)?
        } else {
            text.to_string()
        };

        // Try primary provider, fallback to others on failure
        match self.active {
            TtsProviderType::ElevenLabs => {
                if let Some(ref client) = self.elevenlabs {
                    return client.synthesize(&text_to_speak).await;
                }
            }
            TtsProviderType::OpenAi => {
                if let Some(ref client) = self.openai {
                    return client.synthesize(&text_to_speak).await;
                }
            }
            TtsProviderType::Edge => {
                if let Some(ref client) = self.edge {
                    return client.synthesize(&text_to_speak).await;
                }
            }
            TtsProviderType::Local => {
                // TODO: Implement local TTS
                return Err(DrivenError::Config("Local TTS not yet implemented".into()));
            }
        }

        // Fallback chain: OpenAI -> Edge
        if let Some(ref client) = self.openai {
            if let Ok(audio) = client.synthesize(&text_to_speak).await {
                return Ok(audio);
            }
        }
        if let Some(ref client) = self.edge {
            return client.synthesize(&text_to_speak).await;
        }

        Err(DrivenError::Config("No TTS provider available".into()))
    }

    /// Synthesize with a specific voice
    pub async fn synthesize_with_voice(&self, text: &str, voice_id: &str) -> Result<SynthesizedAudio> {
        match self.active {
            TtsProviderType::ElevenLabs => {
                if let Some(ref client) = self.elevenlabs {
                    return client.synthesize_with_voice(text, voice_id).await;
                }
            }
            TtsProviderType::OpenAi => {
                if let Some(ref client) = self.openai {
                    return client.synthesize_with_voice(text, voice_id).await;
                }
            }
            TtsProviderType::Edge => {
                if let Some(ref client) = self.edge {
                    return client.synthesize_with_voice(text, voice_id).await;
                }
            }
            _ => {}
        }
        Err(DrivenError::Config("TTS provider not configured".into()))
    }

    /// Speak text (synthesize and play)
    pub async fn speak(&self, text: &str) -> Result<()> {
        let audio = self.synthesize(text).await?;
        self.play_audio(&audio).await
    }

    /// Speak with a specific voice
    pub async fn speak_with_voice(&self, text: &str, voice_id: &str) -> Result<()> {
        let audio = self.synthesize_with_voice(text, voice_id).await?;
        self.play_audio(&audio).await
    }

    /// List available voices
    pub async fn list_voices(&self) -> Result<Vec<TtsVoice>> {
        match self.active {
            TtsProviderType::ElevenLabs => {
                if let Some(ref client) = self.elevenlabs {
                    return client.list_voices().await;
                }
            }
            TtsProviderType::OpenAi => {
                return Ok(OpenAiTtsClient::available_voices());
            }
            TtsProviderType::Edge => {
                if let Some(ref client) = self.edge {
                    return client.list_voices().await;
                }
            }
            _ => {}
        }
        Ok(vec![])
    }

    /// Check if should auto-speak based on mode and content
    pub fn should_auto_speak(&self, text: &str) -> bool {
        match self.config.auto_mode {
            AutoMode::Always => true,
            AutoMode::Never => false,
            AutoMode::Smart => {
                // Auto-speak for short to medium responses
                text.len() < self.config.max_text_length && !text.contains("```")
            }
        }
    }

    /// Summarize long text for TTS
    fn summarize_text(&self, text: &str) -> Result<String> {
        // Simple summarization: take first N sentences
        let sentences: Vec<&str> = text
            .split(|c| c == '.' || c == '!' || c == '?')
            .filter(|s| !s.trim().is_empty())
            .take(5)
            .collect();
        
        Ok(sentences.join(". ") + ".")
    }

    /// Play synthesized audio
    async fn play_audio(&self, audio: &SynthesizedAudio) -> Result<()> {
        // TODO: Implement audio playback using rodio or similar
        // For now, this is a placeholder
        tracing::info!(
            "Playing audio: {} bytes, format: {:?}",
            audio.data.len(),
            audio.format
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TtsConfig::default();
        assert_eq!(config.provider, TtsProviderType::ElevenLabs);
        assert_eq!(config.auto_mode, AutoMode::Smart);
        assert_eq!(config.max_text_length, 1500);
    }

    #[test]
    fn test_audio_format() {
        assert_eq!(AudioFormat::Mp3.extension(), "mp3");
        assert_eq!(AudioFormat::Wav.mime_type(), "audio/wav");
    }
}
