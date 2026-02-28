//! Wispr Flow - Voice dictation with auto-editing
//!
//! Features:
//! - Rule-based text enhancement (filler word removal, grammar fixes)
//! - Automatic punctuation and capitalization
//! - Custom sound-to-text mapping

#[cfg(feature = "wisprflow")]
pub mod processor;
#[cfg(feature = "wisprflow")]
pub mod simple_enhancer;

#[cfg(feature = "wisprflow")]
pub use processor::{WisprFlow, WisprFlowResult};
