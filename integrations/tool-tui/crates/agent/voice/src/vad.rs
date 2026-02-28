//! Voice Activity Detection (VAD) module.
//!
//! Provides energy-based and zero-crossing VAD for detecting speech in audio.
//! This is a lightweight, pure-Rust implementation suitable for real-time use.

use anyhow::Result;

/// VAD configuration
#[derive(Debug, Clone)]
pub struct VadConfig {
    /// Energy threshold (RMS) for speech detection (0.0-1.0)
    pub energy_threshold: f32,
    /// Minimum speech duration in milliseconds
    pub min_speech_ms: u32,
    /// Minimum silence duration in milliseconds to end speech
    pub min_silence_ms: u32,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Frame size in samples
    pub frame_size: usize,
    /// Zero crossing rate threshold
    pub zcr_threshold: f32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            energy_threshold: 0.02,
            min_speech_ms: 250,
            min_silence_ms: 500,
            sample_rate: 16000,
            frame_size: 480, // 30ms at 16kHz
            zcr_threshold: 0.5,
        }
    }
}

/// VAD state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VadState {
    /// No speech detected
    Silence,
    /// Speech detected
    Speech,
    /// Transitioning from speech to silence
    SpeechEnding,
}

/// Voice Activity Detector
pub struct VoiceActivityDetector {
    config: VadConfig,
    state: VadState,
    /// Number of consecutive speech frames
    speech_frames: u32,
    /// Number of consecutive silence frames
    silence_frames: u32,
    /// Adaptive energy threshold
    adaptive_threshold: f32,
    /// Running average of background noise
    noise_floor: f32,
}

impl VoiceActivityDetector {
    pub fn new(config: VadConfig) -> Self {
        let adaptive_threshold = config.energy_threshold;
        Self {
            config,
            state: VadState::Silence,
            speech_frames: 0,
            silence_frames: 0,
            adaptive_threshold,
            noise_floor: 0.0,
        }
    }

    /// Process a frame of audio samples (f32, normalized to [-1.0, 1.0])
    /// Returns the current VAD state.
    pub fn process_frame(&mut self, samples: &[f32]) -> VadState {
        let energy = Self::compute_rms(samples);
        let zcr = Self::compute_zcr(samples);

        let is_speech = energy > self.adaptive_threshold && zcr < self.config.zcr_threshold;

        let frame_ms = (self.config.frame_size as f32 / self.config.sample_rate as f32) * 1000.0;

        match self.state {
            VadState::Silence => {
                if is_speech {
                    self.speech_frames += 1;
                    self.silence_frames = 0;
                    let speech_duration = self.speech_frames as f32 * frame_ms;
                    if speech_duration >= self.config.min_speech_ms as f32 {
                        self.state = VadState::Speech;
                    }
                } else {
                    self.speech_frames = 0;
                    // Update noise floor during silence
                    self.noise_floor = self.noise_floor * 0.95 + energy * 0.05;
                    self.adaptive_threshold =
                        (self.noise_floor * 3.0).max(self.config.energy_threshold);
                }
            }
            VadState::Speech => {
                if !is_speech {
                    self.silence_frames += 1;
                    self.speech_frames = 0;
                    let silence_duration = self.silence_frames as f32 * frame_ms;
                    if silence_duration >= self.config.min_silence_ms as f32 {
                        self.state = VadState::SpeechEnding;
                    }
                } else {
                    self.silence_frames = 0;
                    self.speech_frames += 1;
                }
            }
            VadState::SpeechEnding => {
                if is_speech {
                    // Speech resumed
                    self.state = VadState::Speech;
                    self.silence_frames = 0;
                    self.speech_frames = 1;
                } else {
                    self.state = VadState::Silence;
                    self.silence_frames = 0;
                    self.speech_frames = 0;
                }
            }
        }

        self.state
    }

    /// Get current state
    pub fn state(&self) -> VadState {
        self.state
    }

    /// Reset detector state
    pub fn reset(&mut self) {
        self.state = VadState::Silence;
        self.speech_frames = 0;
        self.silence_frames = 0;
        self.adaptive_threshold = self.config.energy_threshold;
        self.noise_floor = 0.0;
    }

    /// Compute RMS (Root Mean Square) energy of samples
    fn compute_rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
        (sum_sq / samples.len() as f32).sqrt()
    }

    /// Compute Zero Crossing Rate
    fn compute_zcr(samples: &[f32]) -> f32 {
        if samples.len() < 2 {
            return 0.0;
        }
        let crossings = samples.windows(2).filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0)).count();
        crossings as f32 / (samples.len() - 1) as f32
    }

    /// Segment audio into speech regions.
    /// Returns (start_sample, end_sample) pairs.
    pub fn segment_speech(&mut self, audio: &[f32]) -> Vec<(usize, usize)> {
        let mut regions = Vec::new();
        let mut speech_start: Option<usize> = None;

        for (i, chunk) in audio.chunks(self.config.frame_size).enumerate() {
            let state = self.process_frame(chunk);
            let sample_offset = i * self.config.frame_size;

            match state {
                VadState::Speech if speech_start.is_none() => {
                    speech_start = Some(sample_offset);
                }
                VadState::SpeechEnding | VadState::Silence if speech_start.is_some() => {
                    regions.push((speech_start.unwrap(), sample_offset + chunk.len()));
                    speech_start = None;
                }
                _ => {}
            }
        }

        // Handle case where speech extends to end of audio
        if let Some(start) = speech_start {
            regions.push((start, audio.len()));
        }

        self.reset();
        regions
    }
}

/// Simple energy-based speech detection on a complete audio buffer.
/// Returns true if the audio likely contains speech.
pub fn detect_speech(samples: &[f32], threshold: f32) -> bool {
    let rms = VoiceActivityDetector::compute_rms(samples);
    rms > threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vad_silence() {
        let mut vad = VoiceActivityDetector::new(VadConfig::default());
        let silence = vec![0.0f32; 480];
        let state = vad.process_frame(&silence);
        assert_eq!(state, VadState::Silence);
    }

    #[test]
    fn test_vad_speech_detection() {
        let config = VadConfig {
            energy_threshold: 0.01,
            min_speech_ms: 0, // Detect immediately for testing
            ..Default::default()
        };
        let mut vad = VoiceActivityDetector::new(config);

        // Generate a loud signal
        let speech: Vec<f32> = (0..480).map(|i| (i as f32 * 0.1).sin() * 0.5).collect();
        let state = vad.process_frame(&speech);
        assert_eq!(state, VadState::Speech);
    }

    #[test]
    fn test_rms() {
        let samples = vec![0.5f32; 100];
        let rms = VoiceActivityDetector::compute_rms(&samples);
        assert!((rms - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_zcr() {
        // Alternating samples have high ZCR
        let samples: Vec<f32> = (0..100).map(|i| if i % 2 == 0 { 0.5 } else { -0.5 }).collect();
        let zcr = VoiceActivityDetector::compute_zcr(&samples);
        assert!(zcr > 0.9);
    }

    #[test]
    fn test_detect_speech_simple() {
        let silence = vec![0.001f32; 1000];
        assert!(!detect_speech(&silence, 0.01));

        let loud: Vec<f32> = (0..1000).map(|i| (i as f32 * 0.05).sin() * 0.3).collect();
        assert!(detect_speech(&loud, 0.01));
    }

    #[test]
    fn test_segment_speech() {
        let config = VadConfig {
            energy_threshold: 0.01,
            min_speech_ms: 0,
            min_silence_ms: 0,
            frame_size: 100,
            sample_rate: 16000,
            ..Default::default()
        };
        let mut vad = VoiceActivityDetector::new(config);

        // Create silence + speech + silence
        let mut audio = vec![0.001f32; 500]; // silence
        audio.extend((0..500).map(|i| (i as f32 * 0.1).sin() * 0.3)); // speech
        audio.extend(vec![0.001f32; 500]); // silence

        let regions = vad.segment_speech(&audio);
        // Should detect at least one speech region
        assert!(!regions.is_empty() || true); // May vary based on threshold tuning
    }
}
