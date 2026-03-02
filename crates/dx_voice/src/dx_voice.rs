//! DX Voice — Real-time voice conversation engine.
//!
//! Covers the entire voice pipeline:
//! - Local STT via Whisper (whisper-rs, whisper-cpp-plus)
//! - Local TTS via Piper, Chatterbox-Turbo, Kokoro
//! - Cloud TTS via ElevenLabs, Fish Audio, Cartesia, PlayHT, Deepgram,
//!   Google Cloud TTS, Amazon Polly, Azure Speech, OpenAI TTS,
//!   WellSaid Labs, Murf AI, Lovo AI
//! - Conversation loop: User speaks → Whisper → LLM → TTS → User
//! - Full-duplex mode with interrupt detection and streaming TTS
//! - Quality-based TTS routing (latency vs quality vs cost)
//! - Flow Bar states: idle, listening, transcribing, post-processing, result, speaking

// ── Core modules ─────────────────────────────────────────────────────────
mod conversation;
mod flow_bar;
mod piper_tts;
mod stt_engine;
mod tts_manager;

// ── Local TTS engines (Part 21) ─────────────────────────────────────────
pub mod chatterbox_tts;
pub mod cloud_tts;
pub mod kokoro_tts;

// ── Full-duplex & routing (Part 23) ─────────────────────────────────────
pub mod full_duplex;
pub mod quality_router;

// ── Cloud TTS adapters (Part 22) ────────────────────────────────────────
pub mod amazon_polly_tts;
pub mod azure_speech_tts;
pub mod cartesia_tts;
pub mod deepgram_tts;
pub mod elevenlabs_tts;
pub mod extra_cloud_tts;
pub mod fish_audio_tts;
pub mod google_cloud_tts;
pub mod openai_tts;
pub mod playht_tts;

// ── Core re-exports ─────────────────────────────────────────────────────
pub use conversation::*;
pub use flow_bar::*;
pub use piper_tts::*;
pub use stt_engine::*;
pub use tts_manager::*;

// ── Cloud TTS re-exports ────────────────────────────────────────────────
pub use amazon_polly_tts::AmazonPollyTts;
pub use azure_speech_tts::AzureSpeechTts;
pub use cartesia_tts::CartesiaTts;
pub use deepgram_tts::DeepgramTts;
pub use elevenlabs_tts::ElevenLabsTts;
pub use extra_cloud_tts::{LovoTts, MurfTts, WellSaidTts};
pub use fish_audio_tts::FishAudioTts;
pub use full_duplex::{FullDuplexConfig, FullDuplexEngine, StreamingTtsBuffer};
pub use google_cloud_tts::GoogleCloudTts;
pub use openai_tts::OpenAiTts;
pub use playht_tts::PlayHtTts;
pub use quality_router::{route_tts_request, QualityLevel, RouteContext, RouteDecision};

use dx_core::tts_provider::TtsProvider;
use gpui::App;

/// Initialize the DX voice subsystem.
pub fn init(_cx: &mut App) {
    log::info!("DX Voice engine initialized");
}

/// Register all cloud TTS providers and return them.
///
/// Each provider auto-detects its API key from environment variables.
/// Only providers with valid keys will report `is_available() == true`.
pub fn register_cloud_tts_providers() -> Vec<Box<dyn TtsProvider>> {
    let providers: Vec<Box<dyn TtsProvider>> = vec![
        Box::new(ElevenLabsTts::from_env()),
        Box::new(FishAudioTts::from_env()),
        Box::new(CartesiaTts::from_env()),
        Box::new(PlayHtTts::from_env()),
        Box::new(DeepgramTts::from_env()),
        Box::new(GoogleCloudTts::from_env()),
        Box::new(AmazonPollyTts::from_env()),
        Box::new(AzureSpeechTts::from_env()),
        Box::new(OpenAiTts::from_env()),
        Box::new(WellSaidTts::from_env()),
        Box::new(MurfTts::from_env()),
        Box::new(LovoTts::from_env()),
    ];

    let available_count = providers.iter().filter(|p| p.is_available()).count();
    log::info!(
        "DX Voice: registered {} cloud TTS providers ({} available)",
        providers.len(),
        available_count
    );

    providers
}
