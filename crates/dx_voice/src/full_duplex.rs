//! Full-duplex voice conversation engine.
//!
//! Enhances the basic conversation loop with:
//! - Full-duplex mode (simultaneous listening and speaking)
//! - Interrupt detection (user speaks while TTS is playing)
//! - Streaming TTS playback (start speaking before full LLM response)
//! - LLM course-correction pass on raw transcription

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Full-duplex conversation mode that supports interruption and streaming.
pub struct FullDuplexEngine {
    /// Whether the engine is currently in a conversation.
    is_active: Arc<AtomicBool>,
    /// Whether we're currently speaking TTS output.
    is_speaking: Arc<AtomicBool>,
    /// Whether we've detected the user trying to interrupt.
    interrupt_detected: Arc<AtomicBool>,
    /// Configuration for interrupt detection.
    config: FullDuplexConfig,
    /// Timestamp of last user speech detected.
    last_user_speech: Option<Instant>,
}

/// Configuration for full-duplex voice conversation.
#[derive(Debug, Clone)]
pub struct FullDuplexConfig {
    /// Minimum audio energy to consider as speech (0.0 - 1.0).
    pub vad_threshold: f32,
    /// How long user must speak to trigger interrupt (milliseconds).
    pub interrupt_threshold_ms: u64,
    /// Whether to enable LLM course-correction on raw transcription.
    pub enable_course_correction: bool,
    /// Whether to stream TTS (start playing before full response).
    pub enable_streaming_tts: bool,
    /// Maximum duration for a single turn (seconds).
    pub max_turn_duration_secs: u64,
    /// Silence duration to end a turn (milliseconds).
    pub silence_end_turn_ms: u64,
}

impl Default for FullDuplexConfig {
    fn default() -> Self {
        Self {
            vad_threshold: 0.3,
            interrupt_threshold_ms: 300,
            enable_course_correction: true,
            enable_streaming_tts: true,
            max_turn_duration_secs: 120,
            silence_end_turn_ms: 1500,
        }
    }
}

/// Result of interrupt detection analysis.
#[derive(Debug, Clone)]
pub enum InterruptResult {
    /// No interrupt detected — continue speaking.
    NoInterrupt,
    /// User is speaking — pause TTS, start listening.
    UserSpeaking {
        /// Estimated audio energy level.
        energy_level: f32,
        /// Duration of detected speech.
        speech_duration: Duration,
    },
    /// User explicitly said a stop word ("stop", "wait", "hold on").
    StopWordDetected {
        word: String,
    },
}

impl FullDuplexEngine {
    pub fn new(config: FullDuplexConfig) -> Self {
        Self {
            is_active: Arc::new(AtomicBool::new(false)),
            is_speaking: Arc::new(AtomicBool::new(false)),
            interrupt_detected: Arc::new(AtomicBool::new(false)),
            config,
            last_user_speech: None,
        }
    }

    /// Start a full-duplex conversation session.
    pub fn start(&self) {
        self.is_active.store(true, Ordering::SeqCst);
        log::info!("Full-duplex conversation started");
    }

    /// Stop the conversation session.
    pub fn stop(&self) {
        self.is_active.store(false, Ordering::SeqCst);
        self.is_speaking.store(false, Ordering::SeqCst);
        self.interrupt_detected.store(false, Ordering::SeqCst);
        log::info!("Full-duplex conversation stopped");
    }

    /// Check if the engine is active.
    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::SeqCst)
    }

    /// Mark that TTS is currently playing.
    pub fn set_speaking(&self, speaking: bool) {
        self.is_speaking.store(speaking, Ordering::SeqCst);
    }

    /// Check if TTS is currently playing.
    pub fn is_speaking(&self) -> bool {
        self.is_speaking.load(Ordering::SeqCst)
    }

    /// Analyze an audio chunk for interrupt detection.
    ///
    /// Called continuously while TTS is playing to detect if the user
    /// is trying to speak (interrupt).
    pub fn check_interrupt(&mut self, audio_energy: f32) -> InterruptResult {
        if !self.is_speaking() {
            return InterruptResult::NoInterrupt;
        }

        if audio_energy > self.config.vad_threshold {
            let now = Instant::now();
            if let Some(last) = self.last_user_speech {
                let speech_duration = now.duration_since(last);
                if speech_duration.as_millis() as u64 >= self.config.interrupt_threshold_ms {
                    self.interrupt_detected.store(true, Ordering::SeqCst);
                    return InterruptResult::UserSpeaking {
                        energy_level: audio_energy,
                        speech_duration,
                    };
                }
            } else {
                self.last_user_speech = Some(now);
            }
        } else {
            self.last_user_speech = None;
        }

        InterruptResult::NoInterrupt
    }

    /// Apply LLM course-correction to raw Whisper transcription.
    ///
    /// Fixes common Whisper errors:
    /// - Homophones ("their" vs "there" vs "they're")
    /// - Missing punctuation
    /// - Filler word removal ("um", "uh", "like")
    /// - Technical term correction based on context
    pub fn course_correct_prompt(raw_transcription: &str) -> String {
        if !raw_transcription.contains(' ') {
            return raw_transcription.to_string();
        }

        format!(
            "Fix any transcription errors in this spoken text. \
             Remove filler words (um, uh, like). Fix homophones. \
             Add proper punctuation. Keep the original meaning.\n\n\
             Raw transcription: {}\n\nCorrected text:",
            raw_transcription
        )
    }

    /// Check for stop words in a transcription fragment.
    pub fn detect_stop_word(text: &str) -> Option<String> {
        let stop_words = ["stop", "wait", "hold on", "pause", "shut up", "quiet", "enough"];
        let lower = text.to_lowercase();
        for word in &stop_words {
            if lower.contains(word) {
                return Some(word.to_string());
            }
        }
        None
    }
}

/// Streaming TTS token buffer.
///
/// Collects LLM output tokens and triggers TTS synthesis at sentence
/// boundaries, allowing speech to start before the full response is ready.
pub struct StreamingTtsBuffer {
    /// Accumulated text not yet sent to TTS.
    buffer: String,
    /// Sentences already sent to TTS.
    sent_sentences: Vec<String>,
    /// Minimum characters before triggering TTS (to avoid tiny fragments).
    min_chunk_chars: usize,
}

impl StreamingTtsBuffer {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            sent_sentences: Vec::new(),
            min_chunk_chars: 40,
        }
    }

    /// Add a token from the LLM stream.
    /// Returns Some(text) if a sentence boundary was found and should be spoken.
    pub fn push_token(&mut self, token: &str) -> Option<String> {
        self.buffer.push_str(token);

        // Check for sentence-ending punctuation
        if let Some(pos) = self.buffer.rfind(|c: char| c == '.' || c == '!' || c == '?') {
            let sentence_end = pos + 1;
            if sentence_end >= self.min_chunk_chars {
                let sentence = self.buffer[..sentence_end].trim().to_string();
                self.buffer = self.buffer[sentence_end..].to_string();
                self.sent_sentences.push(sentence.clone());
                return Some(sentence);
            }
        }

        None
    }

    /// Flush any remaining text (called when LLM stream ends).
    pub fn flush(&mut self) -> Option<String> {
        if self.buffer.trim().is_empty() {
            return None;
        }
        let remaining = self.buffer.trim().to_string();
        self.buffer.clear();
        self.sent_sentences.push(remaining.clone());
        Some(remaining)
    }

    /// Get all sentences sent so far.
    pub fn sent_sentences(&self) -> &[String] {
        &self.sent_sentences
    }

    /// Reset the buffer.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.sent_sentences.clear();
    }
}

impl Default for StreamingTtsBuffer {
    fn default() -> Self {
        Self::new()
    }
}
