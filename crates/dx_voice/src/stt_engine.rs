//! Speech-to-Text engine — local Whisper integration.
//!
//! Tiered Whisper models:
//! - Tier 1-2: Whisper Tiny.en (~75MB)
//! - Tier 3: Whisper Base.en (~142MB)
//! - Tier 4: Whisper Small.en (~244MB)
//! - Tier 5: Whisper Large-v3 (~1.5GB)

use anyhow::Result;
use dx_core::DeviceTier;
use serde::{Deserialize, Serialize};

/// STT model configuration based on device tier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttModelConfig {
    pub model_name: String,
    pub model_path: Option<String>,
    pub language: String,
    pub sample_rate: u32,
    pub use_gpu: bool,
}

impl SttModelConfig {
    /// Get the recommended Whisper model for a device tier.
    pub fn for_tier(tier: DeviceTier) -> Self {
        match tier {
            DeviceTier::UltraLow | DeviceTier::Low => Self {
                model_name: "whisper-tiny.en".into(),
                model_path: None,
                language: "en".into(),
                sample_rate: 16000,
                use_gpu: false,
            },
            DeviceTier::Mid => Self {
                model_name: "whisper-base.en".into(),
                model_path: None,
                language: "en".into(),
                sample_rate: 16000,
                use_gpu: false,
            },
            DeviceTier::High => Self {
                model_name: "whisper-small.en".into(),
                model_path: None,
                language: "en".into(),
                sample_rate: 16000,
                use_gpu: true,
            },
            DeviceTier::Ultra => Self {
                model_name: "whisper-large-v3".into(),
                model_path: None,
                language: "en".into(),
                sample_rate: 16000,
                use_gpu: true,
            },
        }
    }
}

/// The states of the STT engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SttState {
    /// Not listening.
    Idle,
    /// Actively recording audio.
    Listening,
    /// Processing audio through Whisper.
    Transcribing,
    /// Running LLM post-processing on transcription.
    PostProcessing,
    /// Result ready for user review.
    ResultReady,
}

/// A transcription result from the STT engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    /// The raw transcription from Whisper.
    pub raw_text: String,
    /// The cleaned/corrected text after LLM post-processing.
    pub cleaned_text: Option<String>,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f64,
    /// Language detected.
    pub language: String,
    /// Duration of the audio segment in seconds.
    pub duration_seconds: f64,
}

/// The STT engine manager.
pub struct SttEngine {
    config: SttModelConfig,
    state: SttState,
    audio_buffer: Vec<f32>,
}

impl SttEngine {
    pub fn new(config: SttModelConfig) -> Self {
        Self {
            config,
            state: SttState::Idle,
            audio_buffer: Vec::new(),
        }
    }

    pub fn state(&self) -> SttState {
        self.state
    }

    pub fn config(&self) -> &SttModelConfig {
        &self.config
    }

    /// Start listening — begin capturing audio.
    pub fn start_listening(&mut self) {
        self.state = SttState::Listening;
        self.audio_buffer.clear();
        log::info!("STT: Started listening with model {}", self.config.model_name);
    }

    /// Stop listening and begin transcription.
    pub fn stop_listening(&mut self) -> Result<()> {
        if self.state != SttState::Listening {
            return Err(anyhow::anyhow!("STT engine is not listening"));
        }
        self.state = SttState::Transcribing;
        log::info!("STT: Stopped listening, {} samples captured", self.audio_buffer.len());
        Ok(())
    }

    /// Feed audio samples while listening.
    pub fn feed_audio(&mut self, samples: &[f32]) {
        if self.state == SttState::Listening {
            self.audio_buffer.extend_from_slice(samples);
        }
    }

    /// Transcribe the captured audio.
    ///
    /// In the real implementation, this calls whisper-rs/whisper-cpp-plus.
    /// For now, returns a placeholder.
    pub async fn transcribe(&mut self) -> Result<TranscriptionResult> {
        self.state = SttState::Transcribing;

        let duration = self.audio_buffer.len() as f64 / self.config.sample_rate as f64;

        // Placeholder — actual implementation would invoke Whisper.
        log::info!(
            "STT: Transcribing {:.1}s of audio with {}",
            duration,
            self.config.model_name
        );

        self.state = SttState::ResultReady;

        Ok(TranscriptionResult {
            raw_text: String::new(),
            cleaned_text: None,
            confidence: 0.0,
            language: self.config.language.clone(),
            duration_seconds: duration,
        })
    }

    /// Reset to idle state.
    pub fn reset(&mut self) {
        self.state = SttState::Idle;
        self.audio_buffer.clear();
    }
}
