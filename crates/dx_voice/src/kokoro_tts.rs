//! Kokoro TTS — fast, lightweight local TTS engine with natural prosody.
//! Supports multiple languages and runs entirely on CPU via ONNX.

use anyhow::Result;
use dx_core::{MicroCost, TtsProvider, TtsProviderId, TtsRequest, TtsResponse};
use std::path::PathBuf;

/// Built-in Kokoro voice presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KokoroVoice {
    /// American female — warm, clear
    AfHeart,
    /// American female — bright, energetic
    AfStar,
    /// American male — deep, confident
    AmAdam,
    /// American male — calm, steady
    AmMichael,
    /// British female — crisp, professional
    BfEmma,
    /// British male — authoritative
    BmGeorge,
}

impl KokoroVoice {
    pub fn id(&self) -> &str {
        match self {
            Self::AfHeart => "af_heart",
            Self::AfStar => "af_star",
            Self::AmAdam => "am_adam",
            Self::AmMichael => "am_michael",
            Self::BfEmma => "bf_emma",
            Self::BmGeorge => "bm_george",
        }
    }

    pub fn all() -> &'static [KokoroVoice] {
        &[
            Self::AfHeart, Self::AfStar, Self::AmAdam,
            Self::AmMichael, Self::BfEmma, Self::BmGeorge,
        ]
    }
}

/// Kokoro TTS provider — ultra-fast local speech synthesis.
pub struct KokoroTts {
    id: TtsProviderId,
    model_path: Option<PathBuf>,
    voice: KokoroVoice,
    speed: f32,
    available: bool,
}

impl KokoroTts {
    pub fn new() -> Self {
        let model_path = Self::default_model_path();
        let available = model_path
            .as_ref()
            .map(|p| p.exists())
            .unwrap_or(false);
        Self {
            id: TtsProviderId::new("kokoro"),
            model_path,
            voice: KokoroVoice::AfHeart,
            speed: 1.0,
            available,
        }
    }

    pub fn with_voice(mut self, voice: KokoroVoice) -> Self {
        self.voice = voice;
        self
    }

    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed.clamp(0.5, 2.0);
        self
    }

    fn default_model_path() -> Option<PathBuf> {
        let data_dir = dirs::data_dir()?;
        Some(data_dir.join("dx").join("models").join("kokoro"))
    }
}

impl Default for KokoroTts {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TtsProvider for KokoroTts {
    fn id(&self) -> &TtsProviderId { &self.id }
    fn name(&self) -> &str { "Kokoro TTS" }
    fn is_available(&self) -> bool { self.available }

    async fn synthesize(&self, request: &TtsRequest) -> Result<TtsResponse> {
        if !self.available {
            return Err(anyhow::anyhow!(
                "Kokoro model not found at {:?}. Download with `dx model download kokoro`.",
                self.model_path
            ));
        }
        log::info!(
            "Kokoro synthesize: text_len={}, voice={}, speed={}",
            request.text.len(), self.voice.id(), self.speed
        );
        // Placeholder — actual implementation loads ONNX model via ort crate.
        Ok(TtsResponse {
            audio_data: Vec::new(),
            sample_rate: 24000,
            channels: 1,
            duration_ms: 0,
            cost: MicroCost::ZERO,
        })
    }

    fn supported_voices(&self) -> Vec<String> {
        KokoroVoice::all().iter().map(|v| v.id().to_string()).collect()
    }

    fn max_text_length(&self) -> usize {
        // Kokoro handles up to ~500 chars per segment efficiently
        500
    }
}
