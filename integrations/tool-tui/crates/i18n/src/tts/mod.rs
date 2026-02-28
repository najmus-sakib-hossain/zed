//! Text-to-Speech module
//!
//! This module provides TTS functionality through multiple providers.

pub mod base;
pub mod constants;
pub mod edge;
pub mod google;

pub use base::TextToSpeech;
pub use edge::EdgeTTS;
pub use google::GoogleTTS;
