//! Local Piper TTS provider — fast neural TTS, runs on Raspberry Pi.
//!
//! Uses piper-rs crate for ONNX-based VITS models.

use anyhow::Result;
use dx_core::{
    MicroCost, TtsOutput, TtsProvider, TtsProviderId, TtsProviderLocation,
    TtsRequest, VoiceGender, VoiceInfo,
};

/// Piper TTS local provider.
pub struct PiperTtsProvider {
    id: TtsProviderId,
    model_loaded: bool,
}

impl PiperTtsProvider {
    pub fn new() -> Self {
        Self {
            id: dx_core::tts_providers::piper(),
            model_loaded: false,
        }
    }

    /// Load the Piper model from disk.
    pub fn load_model(&mut self, _model_path: &str) -> Result<()> {
        // Placeholder — real implementation loads ONNX model via piper-rs
        log::info!("Piper TTS: Loading model");
        self.model_loaded = true;
        Ok(())
    }
}

impl Default for PiperTtsProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl TtsProvider for PiperTtsProvider {
    fn id(&self) -> &TtsProviderId {
        &self.id
    }

    fn name(&self) -> &str {
        "Piper TTS (Local)"
    }

    fn location(&self) -> TtsProviderLocation {
        TtsProviderLocation::Local
    }

    fn is_available(&self) -> bool {
        self.model_loaded
    }

    async fn list_voices(&self) -> Result<Vec<VoiceInfo>> {
        Ok(vec![
            VoiceInfo {
                id: "en_US-lessac-medium".into(),
                name: "Lessac (Medium)".into(),
                provider_id: self.id.clone(),
                language: "en-US".into(),
                gender: Some(VoiceGender::Male),
                preview_url: None,
                supports_cloning: false,
            },
            VoiceInfo {
                id: "en_US-amy-medium".into(),
                name: "Amy (Medium)".into(),
                provider_id: self.id.clone(),
                language: "en-US".into(),
                gender: Some(VoiceGender::Female),
                preview_url: None,
                supports_cloning: false,
            },
            VoiceInfo {
                id: "en_GB-alba-medium".into(),
                name: "Alba (Medium)".into(),
                provider_id: self.id.clone(),
                language: "en-GB".into(),
                gender: Some(VoiceGender::Female),
                preview_url: None,
                supports_cloning: false,
            },
        ])
    }

    async fn speak(&self, request: &TtsRequest) -> Result<TtsOutput> {
        if !self.model_loaded {
            return Err(anyhow::anyhow!("Piper model not loaded"));
        }

        // Placeholder — real implementation runs ONNX inference via piper-rs
        log::info!(
            "Piper TTS: Synthesizing {} chars with voice {}",
            request.text.len(),
            request.voice_id
        );

        let estimated_duration = request.text.len() as f64 / 15.0; // ~15 chars/sec
        let sample_count = (estimated_duration * request.sample_rate as f64) as usize;

        Ok(TtsOutput {
            audio_data: vec![0.0; sample_count],
            sample_rate: request.sample_rate,
            duration_seconds: estimated_duration,
            cost: MicroCost::ZERO, // Local is free
        })
    }

    fn cost_per_character(&self) -> Option<MicroCost> {
        Some(MicroCost::ZERO) // Free — local
    }
}
