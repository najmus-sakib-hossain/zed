//! # i18n - Internationalization Library
//!
//! A Rust library for translation, text-to-speech, and speech-to-text.
//!
//! ## Modules
//! - `locale`: Translation functionality supporting multiple providers
//! - `tts`: Text-to-speech functionality supporting Microsoft Edge TTS and Google TTS
//! - `sts`: Speech-to-text functionality using Whisper

pub mod error;
pub mod locale;
pub mod sts;
pub mod tts;

#[cfg(feature = "wisprflow")]
pub mod wisprflow;

pub use error::{I18nError, Result};
