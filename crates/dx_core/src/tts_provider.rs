//! Unified TTS Provider trait — voice synthesis abstraction layer.
//!
//! Every TTS provider implements the same trait: speak(), stream_speak(),
//! list_voices(), clone_voice(). This covers both local (Piper, Chatterbox,
//! Kokoro) and cloud (ElevenLabs, Fish Audio, Cartesia, etc.) providers.

use crate::cost::MicroCost;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Unique identifier for a TTS provider.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TtsProviderId(pub Arc<str>);

impl TtsProviderId {
    pub fn new(id: impl Into<Arc<str>>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for TtsProviderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Whether a TTS provider runs locally or in the cloud.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TtsProviderLocation {
    Local,
    Cloud,
}

/// A voice available from a TTS provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceInfo {
    pub id: String,
    pub name: String,
    pub provider_id: TtsProviderId,
    pub language: String,
    pub gender: Option<VoiceGender>,
    pub preview_url: Option<String>,
    pub supports_cloning: bool,
}

/// Voice gender classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoiceGender {
    Male,
    Female,
    Neutral,
}

/// Request to synthesize speech.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsRequest {
    pub text: String,
    pub voice_id: String,
    pub speed: f32,
    pub pitch: f32,
    /// Output sample rate.
    pub sample_rate: u32,
}

impl Default for TtsRequest {
    fn default() -> Self {
        Self {
            text: String::new(),
            voice_id: String::new(),
            speed: 1.0,
            pitch: 1.0,
            sample_rate: 22050,
        }
    }
}

/// Generated audio output.
#[derive(Debug, Clone)]
pub struct TtsOutput {
    /// PCM audio data (mono, f32 samples).
    pub audio_data: Vec<f32>,
    pub sample_rate: u32,
    pub duration_seconds: f64,
    pub cost: MicroCost,
}

/// The core trait every TTS provider must implement.
///
/// This is the unified voice abstraction layer. Same pattern as Universe A/B.
#[async_trait::async_trait]
pub trait TtsProvider: Send + Sync {
    /// Unique provider identifier.
    fn id(&self) -> &TtsProviderId;

    /// Human-readable name.
    fn name(&self) -> &str;

    /// Whether this runs locally or in the cloud.
    fn location(&self) -> TtsProviderLocation;

    /// Whether this provider is currently available.
    fn is_available(&self) -> bool;

    /// List available voices.
    async fn list_voices(&self) -> Result<Vec<VoiceInfo>>;

    /// Synthesize speech from text.
    async fn speak(&self, request: &TtsRequest) -> Result<TtsOutput>;

    /// Clone a voice from sample audio (if supported).
    async fn clone_voice(&self, _name: &str, _sample_audio: &[f32], _sample_rate: u32) -> Result<VoiceInfo> {
        Err(anyhow::anyhow!("Voice cloning not supported by this provider"))
    }

    /// Estimate cost per character.
    fn cost_per_character(&self) -> Option<MicroCost>;
}

/// A fallback chain for TTS: Local Piper → Cloud provider → Different cloud.
pub struct TtsFallbackChain {
    pub providers: Vec<Arc<dyn TtsProvider>>,
}

impl TtsFallbackChain {
    pub fn new(providers: Vec<Arc<dyn TtsProvider>>) -> Self {
        Self { providers }
    }

    pub async fn speak(&self, request: &TtsRequest) -> Result<TtsOutput> {
        let mut last_error = None;
        for provider in &self.providers {
            if !provider.is_available() {
                continue;
            }
            match provider.speak(request).await {
                Ok(output) => return Ok(output),
                Err(e) => {
                    log::warn!("TTS provider {} failed, trying next: {:?}", provider.name(), e);
                    last_error = Some(e);
                }
            }
        }
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("No available TTS providers")))
    }
}

// ---------------------------------------------------------------------------
// Well-known TTS providers (Part 22)
// ---------------------------------------------------------------------------

pub mod tts_providers {
    use super::TtsProviderId;

    // Local (free, unlimited)
    pub fn piper() -> TtsProviderId { TtsProviderId::new("piper") }
    pub fn chatterbox() -> TtsProviderId { TtsProviderId::new("chatterbox") }
    pub fn kokoro() -> TtsProviderId { TtsProviderId::new("kokoro") }

    // Cloud
    pub fn elevenlabs() -> TtsProviderId { TtsProviderId::new("elevenlabs") }
    pub fn fish_audio() -> TtsProviderId { TtsProviderId::new("fish-audio") }
    pub fn cartesia() -> TtsProviderId { TtsProviderId::new("cartesia") }
    pub fn playht() -> TtsProviderId { TtsProviderId::new("playht") }
    pub fn deepgram_aura() -> TtsProviderId { TtsProviderId::new("deepgram-aura") }
    pub fn google_cloud_tts() -> TtsProviderId { TtsProviderId::new("google-cloud-tts") }
    pub fn amazon_polly() -> TtsProviderId { TtsProviderId::new("amazon-polly") }
    pub fn azure_speech() -> TtsProviderId { TtsProviderId::new("azure-speech") }
    pub fn openai_tts() -> TtsProviderId { TtsProviderId::new("openai-tts") }
    pub fn wellsaid() -> TtsProviderId { TtsProviderId::new("wellsaid") }
    pub fn murf_ai() -> TtsProviderId { TtsProviderId::new("murf-ai") }
    pub fn lovo_ai() -> TtsProviderId { TtsProviderId::new("lovo-ai") }
}
