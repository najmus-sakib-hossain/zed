//! Speech-to-Text module
//!
//! This module provides speech-to-text (STT) functionality using multiple providers.
//!
//! ## Providers
//! - `GoogleSTT`: Free Google Speech Recognition API
//! - `WhisperSTT`: Offline Whisper model
//! - `AutoSTT`: Automatic fallback (Google -> Whisper)

pub mod auto;
pub mod base;
pub mod google;
pub mod whisper;

#[cfg(feature = "whisper")]
pub mod whisper_cached;

pub use auto::AutoSTT;
pub use base::SpeechToText;
pub use google::GoogleSTT;

#[cfg(feature = "whisper")]
pub use whisper::WhisperSTT;

#[cfg(feature = "whisper")]
pub use whisper_cached::CachedWhisperSTT;
