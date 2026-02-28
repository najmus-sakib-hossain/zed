//! # Voice Wake Integration
//!
//! Voice activation and wake word detection using local processing.
//! Supports Whisper.cpp for speech recognition.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::voice::{VoiceWake, VoiceWakeConfig};
//!
//! let config = VoiceWakeConfig::from_file("~/.dx/config/voice-wake.sr")?;
//! let mut wake = VoiceWake::new(&config)?;
//!
//! // Start listening for wake word
//! wake.start_listening().await?;
//!
//! // Process audio stream
//! wake.on_wake(|transcript| {
//!     println!("Wake detected: {}", transcript);
//! }).await?;
//! ```

pub mod wake;
pub mod whisper;

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

pub use wake::{WakeWordDetector, WakeWord};
pub use whisper::WhisperClient;

/// Voice wake configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceWakeConfig {
    /// Whether voice wake is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Wake words to listen for
    #[serde(default = "default_wake_words")]
    pub wake_words: Vec<String>,
    /// Sensitivity (0.0 - 1.0)
    #[serde(default = "default_sensitivity")]
    pub sensitivity: f32,
    /// Audio sample rate
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u32,
    /// Path to Whisper model
    pub whisper_model: Option<PathBuf>,
    /// Whether to use VAD (Voice Activity Detection)
    #[serde(default = "default_true")]
    pub use_vad: bool,
    /// Silence threshold in dB
    #[serde(default = "default_silence_threshold")]
    pub silence_threshold: f32,
    /// Minimum speech duration in ms
    #[serde(default = "default_min_speech_duration")]
    pub min_speech_duration: u32,
}

fn default_true() -> bool {
    true
}

fn default_wake_words() -> Vec<String> {
    vec!["hey dx".to_string(), "ok dx".to_string()]
}

fn default_sensitivity() -> f32 {
    0.5
}

fn default_sample_rate() -> u32 {
    16000
}

fn default_silence_threshold() -> f32 {
    -40.0
}

fn default_min_speech_duration() -> u32 {
    300
}

impl Default for VoiceWakeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            wake_words: default_wake_words(),
            sensitivity: default_sensitivity(),
            sample_rate: default_sample_rate(),
            whisper_model: None,
            use_vad: true,
            silence_threshold: default_silence_threshold(),
            min_speech_duration: default_min_speech_duration(),
        }
    }
}

impl VoiceWakeConfig {
    /// Load from .sr config file
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| DrivenError::Io(e))?;
        Self::parse_sr(&content)
    }

    fn parse_sr(_content: &str) -> Result<Self> {
        // TODO: Use dx-serializer for proper parsing
        Ok(Self::default())
    }
}

/// Voice wake event
#[derive(Debug, Clone)]
pub enum VoiceWakeEvent {
    /// Wake word detected
    WakeDetected {
        /// Detected wake word
        wake_word: String,
        /// Confidence score (0.0 - 1.0)
        confidence: f32,
    },
    /// Speech transcribed after wake
    SpeechTranscribed {
        /// Transcribed text
        text: String,
        /// Confidence score
        confidence: f32,
    },
    /// Voice activity started
    SpeechStarted,
    /// Voice activity ended
    SpeechEnded,
    /// Error occurred
    Error(String),
}

/// Voice wake state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceWakeState {
    /// Not listening
    Idle,
    /// Listening for wake word
    Listening,
    /// Wake word detected, listening for command
    Activated,
    /// Processing speech
    Processing,
}

/// Voice wake controller
pub struct VoiceWake {
    config: VoiceWakeConfig,
    state: Arc<RwLock<VoiceWakeState>>,
    detector: Option<WakeWordDetector>,
    whisper: Option<WhisperClient>,
    event_tx: Option<mpsc::Sender<VoiceWakeEvent>>,
}

impl VoiceWake {
    /// Create a new voice wake controller
    pub fn new(config: &VoiceWakeConfig) -> Result<Self> {
        let detector = if config.enabled {
            Some(WakeWordDetector::new(&config.wake_words, config.sensitivity)?)
        } else {
            None
        };

        let whisper = if let Some(ref model_path) = config.whisper_model {
            Some(WhisperClient::new(model_path)?)
        } else {
            None
        };

        Ok(Self {
            config: config.clone(),
            state: Arc::new(RwLock::new(VoiceWakeState::Idle)),
            detector,
            whisper,
            event_tx: None,
        })
    }

    /// Check if voice wake is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled && self.detector.is_some()
    }

    /// Get current state
    pub async fn state(&self) -> VoiceWakeState {
        *self.state.read().await
    }

    /// Start listening for wake word
    pub async fn start_listening(&mut self) -> Result<mpsc::Receiver<VoiceWakeEvent>> {
        if !self.is_enabled() {
            return Err(DrivenError::Config("Voice wake not enabled".into()));
        }

        let (tx, rx) = mpsc::channel(100);
        self.event_tx = Some(tx);

        *self.state.write().await = VoiceWakeState::Listening;

        // Start audio capture in background
        self.start_audio_capture().await?;

        Ok(rx)
    }

    /// Stop listening
    pub async fn stop_listening(&mut self) -> Result<()> {
        *self.state.write().await = VoiceWakeState::Idle;
        self.event_tx = None;
        Ok(())
    }

    /// Process audio samples
    pub async fn process_audio(&mut self, samples: &[f32]) -> Result<()> {
        let state = *self.state.read().await;

        match state {
            VoiceWakeState::Idle => return Ok(()),
            VoiceWakeState::Listening => {
                // Check for wake word
                if let Some(ref detector) = self.detector {
                    if let Some((wake_word, confidence)) = detector.detect(samples)? {
                        self.emit_event(VoiceWakeEvent::WakeDetected {
                            wake_word,
                            confidence,
                        }).await;
                        *self.state.write().await = VoiceWakeState::Activated;
                    }
                }
            }
            VoiceWakeState::Activated => {
                // Listen for command after wake
                self.emit_event(VoiceWakeEvent::SpeechStarted).await;
                *self.state.write().await = VoiceWakeState::Processing;
            }
            VoiceWakeState::Processing => {
                // Transcribe speech
                if let Some(ref whisper) = self.whisper {
                    match whisper.transcribe(samples).await {
                        Ok((text, confidence)) => {
                            self.emit_event(VoiceWakeEvent::SpeechTranscribed {
                                text,
                                confidence,
                            }).await;
                        }
                        Err(e) => {
                            self.emit_event(VoiceWakeEvent::Error(e.to_string())).await;
                        }
                    }
                }
                self.emit_event(VoiceWakeEvent::SpeechEnded).await;
                *self.state.write().await = VoiceWakeState::Listening;
            }
        }

        Ok(())
    }

    /// Start audio capture
    async fn start_audio_capture(&self) -> Result<()> {
        // TODO: Use cpal or similar for audio capture
        // This would run in a background task
        tracing::info!("Starting audio capture at {} Hz", self.config.sample_rate);
        Ok(())
    }

    /// Emit an event to listeners
    async fn emit_event(&self, event: VoiceWakeEvent) {
        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(event).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = VoiceWakeConfig::default();
        assert!(config.enabled);
        assert_eq!(config.wake_words, vec!["hey dx", "ok dx"]);
        assert_eq!(config.sample_rate, 16000);
    }

    #[tokio::test]
    async fn test_voice_wake_creation() {
        let config = VoiceWakeConfig::default();
        let wake = VoiceWake::new(&config);
        assert!(wake.is_ok());
    }
}
