//! DX Voice — Real-time voice conversation engine.
//!
//! Covers the entire voice pipeline:
//! - Local STT via Whisper (whisper-rs, whisper-cpp-plus)
//! - Local TTS via Piper, Chatterbox-Turbo, Kokoro
//! - Cloud TTS via ElevenLabs, Fish Audio, Cartesia, etc.
//! - Conversation loop: User speaks → Whisper → LLM → TTS → User
//! - Flow Bar states: idle, listening, transcribing, post-processing, result, speaking

mod conversation;
mod flow_bar;
mod piper_tts;
mod stt_engine;
mod tts_manager;

pub use conversation::*;
pub use flow_bar::*;
pub use piper_tts::*;
pub use stt_engine::*;
pub use tts_manager::*;

use gpui::App;

/// Initialize the DX voice subsystem.
pub fn init(_cx: &mut App) {
    log::info!("DX Voice engine initialized");
}
