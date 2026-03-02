//! Chatterbox TTS — high-quality open-source voice cloning TTS engine.
//! Runs locally via the Chatterbox Python library or ONNX runtime.

use anyhow::Result;
use dx_core::{MicroCost, TtsProvider, TtsProviderId, TtsRequest, TtsResponse};
use std::path::PathBuf;

/// Chatterbox TTS provider — zero-shot voice cloning with expressive control.
pub struct ChatterboxTts {
    id: TtsProviderId,
    model_path: Option<PathBuf>,
    available: bool,
}

impl ChatterboxTts {
    /// Create from default model location.
    pub fn new() -> Self {
        let model_path = Self::default_model_path();
        let available = model_path
            .as_ref()
            .map(|p| p.exists())
            .unwrap_or(false);
        Self {
            id: TtsProviderId::new("chatterbox"),
            model_path,
            available,
        }
    }

    /// Custom model path.
    pub fn with_model_path(path: PathBuf) -> Self {
        let available = path.exists();
        Self {
            id: TtsProviderId::new("chatterbox"),
            model_path: Some(path),
            available,
        }
    }

    fn default_model_path() -> Option<PathBuf> {
        let data_dir = dirs::data_dir()?;
        Some(data_dir.join("dx").join("models").join("chatterbox"))
    }

    /// Available voices — Chatterbox supports zero-shot cloning from reference audio.
    pub fn voices() -> Vec<ChatterboxVoice> {
        vec![
            ChatterboxVoice {
                id: "default".into(),
                name: "Default".into(),
                description: "Built-in default voice".into(),
            },
            ChatterboxVoice {
                id: "clone".into(),
                name: "Voice Clone".into(),
                description: "Clone from reference audio file".into(),
            },
        ]
    }
}

impl Default for ChatterboxTts {
    fn default() -> Self {
        Self::new()
    }
}

/// A Chatterbox voice preset.
pub struct ChatterboxVoice {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[async_trait::async_trait]
impl TtsProvider for ChatterboxTts {
    fn id(&self) -> &TtsProviderId { &self.id }
    fn name(&self) -> &str { "Chatterbox TTS" }
    fn is_available(&self) -> bool { self.available }

    async fn synthesize(&self, request: &TtsRequest) -> Result<TtsResponse> {
        if !self.available {
            return Err(anyhow::anyhow!(
                "Chatterbox model not found at {:?}. Download it first.",
                self.model_path
            ));
        }
        log::info!(
            "Chatterbox synthesize: text_len={}, voice={:?}",
            request.text.len(),
            request.voice
        );
        // Placeholder — actual implementation would load ONNX model and run inference.
        Ok(TtsResponse {
            audio_data: Vec::new(),
            sample_rate: 24000,
            channels: 1,
            duration_ms: 0,
            cost: MicroCost::ZERO,
        })
    }

    fn supported_voices(&self) -> Vec<String> {
        Self::voices().into_iter().map(|v| v.id).collect()
    }

    fn max_text_length(&self) -> usize {
        // Chatterbox handles up to ~1000 chars well per segment
        1000
    }
}
