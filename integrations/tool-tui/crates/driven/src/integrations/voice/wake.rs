//! Wake Word Detection
//!
//! Detects wake words in audio streams using pattern matching.

use crate::error::{DrivenError, Result};

/// Wake word definition
#[derive(Debug, Clone)]
pub struct WakeWord {
    /// The wake word phrase
    pub phrase: String,
    /// Phoneme representation (for matching)
    pub phonemes: Vec<String>,
    /// Custom sensitivity for this word
    pub sensitivity: Option<f32>,
}

impl WakeWord {
    /// Create a new wake word
    pub fn new(phrase: impl Into<String>) -> Self {
        let phrase = phrase.into();
        Self {
            phonemes: Self::to_phonemes(&phrase),
            phrase,
            sensitivity: None,
        }
    }

    /// Create with custom sensitivity
    pub fn with_sensitivity(mut self, sensitivity: f32) -> Self {
        self.sensitivity = Some(sensitivity);
        self
    }

    /// Convert phrase to phoneme approximation
    fn to_phonemes(phrase: &str) -> Vec<String> {
        // Simple phoneme approximation
        phrase
            .to_lowercase()
            .split_whitespace()
            .map(|w| w.to_string())
            .collect()
    }
}

/// Wake word detector
pub struct WakeWordDetector {
    wake_words: Vec<WakeWord>,
    sensitivity: f32,
    /// Audio buffer for detection
    buffer: Vec<f32>,
    /// Buffer size in samples
    buffer_size: usize,
}

impl WakeWordDetector {
    /// Default buffer size (1 second at 16kHz)
    const DEFAULT_BUFFER_SIZE: usize = 16000;

    /// Create a new wake word detector
    pub fn new(phrases: &[String], sensitivity: f32) -> Result<Self> {
        let wake_words = phrases
            .iter()
            .map(|p| WakeWord::new(p))
            .collect();

        Ok(Self {
            wake_words,
            sensitivity,
            buffer: Vec::with_capacity(Self::DEFAULT_BUFFER_SIZE),
            buffer_size: Self::DEFAULT_BUFFER_SIZE,
        })
    }

    /// Add a wake word
    pub fn add_wake_word(&mut self, word: WakeWord) {
        self.wake_words.push(word);
    }

    /// Detect wake word in audio samples
    /// Returns (wake_word, confidence) if detected
    pub fn detect(&self, samples: &[f32]) -> Result<Option<(String, f32)>> {
        // This is a placeholder for actual wake word detection
        // In production, this would use:
        // 1. Voice Activity Detection (VAD)
        // 2. Feature extraction (MFCC)
        // 3. Pattern matching or neural network

        // For now, we just return None
        // Real implementation would use porcupine, snowboy, or custom model
        Ok(None)
    }

    /// Process audio and update internal buffer
    pub fn process(&mut self, samples: &[f32]) {
        self.buffer.extend_from_slice(samples);
        
        // Keep only the last buffer_size samples
        if self.buffer.len() > self.buffer_size {
            let excess = self.buffer.len() - self.buffer_size;
            self.buffer.drain(0..excess);
        }
    }

    /// Clear the internal buffer
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Get current buffer
    pub fn buffer(&self) -> &[f32] {
        &self.buffer
    }

    /// Calculate RMS energy of audio
    fn calculate_energy(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum: f32 = samples.iter().map(|s| s * s).sum();
        (sum / samples.len() as f32).sqrt()
    }

    /// Check if audio contains speech (simple VAD)
    pub fn is_speech(&self, samples: &[f32], threshold: f32) -> bool {
        let energy = Self::calculate_energy(samples);
        let db = 20.0 * energy.log10();
        db > threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wake_word_creation() {
        let word = WakeWord::new("hey dx");
        assert_eq!(word.phrase, "hey dx");
        assert_eq!(word.phonemes, vec!["hey", "dx"]);
    }

    #[test]
    fn test_detector_creation() {
        let phrases = vec!["hey dx".to_string()];
        let detector = WakeWordDetector::new(&phrases, 0.5);
        assert!(detector.is_ok());
    }

    #[test]
    fn test_energy_calculation() {
        let samples = vec![0.5, -0.5, 0.5, -0.5];
        let energy = WakeWordDetector::calculate_energy(&samples);
        assert!((energy - 0.5).abs() < 0.001);
    }
}
